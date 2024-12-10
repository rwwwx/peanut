use crate::amm_math::{calc_coin_in_pc, load_pool_state};
use crate::config::Settings;
use crate::trait_ext::duration_ext::DurationExt;
use crate::models::PoolAndPrice;
use crate::rpc::yellowstone_grpc::{get_account_data, AccountDataReceiverConf};
use crate::rpc::JsonRpcAccountReceiver;
use crate::storage::{OldRecordCleaner, PoolPriceStorage};
use chrono::Utc;
use serde::Deserialize;
use solana_sdk::pubkey::Pubkey;
use std::fmt::Display;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::spawn;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

#[derive(Clone)]
pub struct PriceFetchService {
    old_record_cleaner: Arc<dyn OldRecordCleaner + Sync + Send>,
    storage: Arc<dyn PoolPriceStorage + Sync + Send>,
    json_rpc_account_receiver: Arc<dyn JsonRpcAccountReceiver + Send + Sync>,
    config: Settings,
    liquidity_pools_account_addresses: Vec<Pubkey>,
}

impl PriceFetchService {
    pub async fn from_settings(
        settings: &Settings,
        old_record_cleaner: Arc<dyn OldRecordCleaner + Sync + Send>,
        storage: Arc<dyn PoolPriceStorage + Sync + Send>,
        json_rpc_account_receiver: Arc<dyn JsonRpcAccountReceiver + Send + Sync>,
    ) -> anyhow::Result<Self> {
        let fetcher =
            Self::new(settings.clone(), old_record_cleaner, storage, json_rpc_account_receiver);

        if fetcher.config.database.clear_old_records {
            fetcher
                .old_record_cleaner
                .clear_old_records(Duration::from_minutes(30))
                .await?
        }

        Ok(fetcher)
    }

    fn new(
        settings: Settings,
        old_record_cleaner: Arc<dyn OldRecordCleaner + Sync + Send>,
        storage: Arc<dyn PoolPriceStorage + Sync + Send>,
        json_rpc_account_receiver: Arc<dyn JsonRpcAccountReceiver + Send + Sync>,
    ) -> Self {
        let liquidity_pools_account_addresses = settings
            .liquidity_pool
            .account_addresses_base54
            .iter()
            .map(|pubkey| Pubkey::from_str(pubkey).expect(&format!("Failed to parse pubkey: {pubkey}")))
            .collect();

        Self {
            old_record_cleaner,
            storage,
            json_rpc_account_receiver,
            config: settings,
            liquidity_pools_account_addresses,
        }
    }

    pub async fn current(&self, pool_pubkey: &Pubkey) -> PriceFetchResponse {
        let pool_pubkey_as_string = pool_pubkey.to_string();

        match self.storage.current(pool_pubkey).await {
            Ok(zero) if zero == -1.0 => PriceFetchResponse::no_data_found(&pool_pubkey_as_string),
            Ok(res) => PriceFetchResponse::current(&pool_pubkey_as_string, res),
            Err(e) => PriceFetchResponse::generic_err(&pool_pubkey_as_string, e.to_string()),
        }
    }

    pub async fn average(&self, pool_pubkey: &Pubkey) -> PriceFetchResponse {
        let (pool_pubkey_as_string, for_interval) = (pool_pubkey.to_string(), Duration::from_minutes(5));

        match self.storage.average(pool_pubkey, for_interval).await {
            Ok(zero) if zero == -1.0 => PriceFetchResponse::no_data_found(&pool_pubkey_as_string),
            Ok(res) => PriceFetchResponse::average(&pool_pubkey_as_string, res, for_interval),
            Err(e) => PriceFetchResponse::generic_err(&pool_pubkey_as_string, e.to_string()),
        }
    }

    pub fn start_price_fetching_in_background(&self) -> anyhow::Result<()> {
        spawn(self.clone().start_price_fetch());
        Ok(())
    }

    pub async fn start_price_fetch(self) -> anyhow::Result<()> {
        let (tx, mut rx) = mpsc::channel::<(Pubkey, Vec<u8>)>(32);

        // TODO: Add supervisor for gRPC Yellowstone data receiver.
        for address in self.liquidity_pools_account_addresses {
            let account_data_receiver_conf = AccountDataReceiverConf {
                account_address: address,
                sender: tx.clone(),
                yellowstone_grpc_endpoint: self.config.rpc.yellowstone_grpc_endpoint.clone(),
            };

            spawn(get_account_data(account_data_receiver_conf));
        }

        while let Some((pool_address, account_data)) = rx.recv().await {
            info!("Successfully received data from: {pool_address}", pool_address = pool_address.to_string());

            let Ok(price) =
                load_pool_state(self.json_rpc_account_receiver.clone(), account_data, &Pubkey::from_str("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8").unwrap(), &pool_address)
                    .await
                    .inspect_err(|e| error!("Failed to load pool state: {e}"))
                    .map(|state| calc_coin_in_pc(&state.pool))
            else {
                continue;
            };

            self.storage
                .save(&PoolAndPrice { pool_pubkey: pool_address, price, updated_at: Utc::now().naive_utc() })
                .await
                .inspect_err(|e| error!("Failed to save price for '{pool_address}'. Cause: {e:?}"))
                .ok();
        }

        warn!("Liquidity pool price fetcher stopped!");

        Ok(())
    }
}

#[derive(Deserialize, Debug, Clone)]
pub enum PriceFetchResponseType {
    CurrentPrice(f64),
    AveragePrice(f64, Duration),
    GenericError(String),
    NoDataFound,
}

impl Display for PriceFetchResponseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            PriceFetchResponseType::CurrentPrice(price) => String::from(format!("Current price: {price}.")),
            PriceFetchResponseType::AveragePrice(price, for_duration) => String::from(format!(
                "Average price for last {last_minutes} minutes: {price}.",
                last_minutes = for_duration.as_minutes()
            )),
            PriceFetchResponseType::GenericError(err_msg) => format!("Error: {err_msg}."),
            PriceFetchResponseType::NoDataFound => "No data found.".to_string(),
        };

        write!(f, "{}", str)
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct PriceFetchResponse {
    pool_address: String,
    response_type: PriceFetchResponseType,
}

impl PriceFetchResponse {
    pub fn current(pool_address: &str, price: f64) -> Self {
        Self {
            pool_address: pool_address.to_string(),
            response_type: PriceFetchResponseType::CurrentPrice(price),
        }
    }

    pub fn average(pool_address: &str, price: f64, for_intrval: Duration) -> Self {
        Self {
            pool_address: pool_address.to_string(),
            response_type: PriceFetchResponseType::AveragePrice(price, for_intrval),
        }
    }

    pub fn no_data_found(pool_address: &str) -> Self {
        Self {
            pool_address: pool_address.to_string(),
            response_type: PriceFetchResponseType::NoDataFound,
        }
    }

    pub fn generic_err(pool_address: &str, err: String) -> Self {
        Self {
            pool_address: pool_address.to_string(),
            response_type: PriceFetchResponseType::GenericError(err),
        }
    }
}

impl Display for PriceFetchResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = format!(
            "{response_type} Pool address: {pool_address}",
            response_type = self.response_type.to_string(),
            pool_address = self.pool_address
        );
        write!(f, "{}", str)
    }
}
