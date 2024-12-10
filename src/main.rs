use crate::config::Settings;
use crate::price_fetcher::PriceFetchService;
use crate::storage::PostgresStorage;
use solana_client::nonblocking::rpc_client::RpcClient;
use std::io;
use std::sync::Arc;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;
use trait_ext::arced_ext::Arced;
use crate::bot::setup_bot;

mod amm_math;
mod bot;
mod config;
mod models;
mod price_fetcher;
mod rpc;
mod storage;
mod trait_ext;

#[tokio::main]
async fn main() -> io::Result<()> {
    let settings =
        Settings::load(None, Some("peanut/src/config")).unwrap_or_else(|e| panic!("Configuration failed: '{e}'!"));
    set_up_logging("info");

    let storage = PostgresStorage::from_settings(&settings)
        .await
        .expect("Can't create storage")
        .arced();
    let json_rpc = RpcClient::new(settings.rpc.json_rpc_endpoint.clone()).arced();

    let price_fetcher = PriceFetchService::from_settings(&settings, storage.clone(), storage, json_rpc)
        .await
        .expect("Can't create price fetch service")
        .arced();
    price_fetcher
        .start_price_fetching_in_background()
        .expect("Can't start price fetch");

    setup_bot(price_fetcher).await.ok();


    Ok(())
}

fn set_up_logging(log_level: &str) {
    let fmt_layer = tracing_subscriber::fmt::layer()
        .pretty()
        .with_thread_names(true)
        .with_writer(Arc::new(io::stdout()));

    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(EnvFilter::new(log_level))
        .init();
}
