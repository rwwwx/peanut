use futures::{SinkExt, StreamExt};
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use tokio::sync::mpsc::Sender;
use tracing::info;
use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::geyser::{SubscribeRequest, SubscribeRequestFilterAccounts};
use yellowstone_grpc_proto::prelude::subscribe_update::UpdateOneof;

// pub fn calc_coin_in_pc(pool_data: &CalculateResult) -> anyhow::Result<f64> {
//     pc_amount * pc_price = coin_amount * coin_price
//     coin_price = pc_price * (pc_amount / coin_amount)
//     Ok((pool_data.pool_pc_vault_amount as f64)
//         / 10_f64.powf(pool_data.pool_pc_decimals as f64)
//         / (pool_data.pool_coin_vault_amount as f64)
//         * 10_f64.powf(pool_data.pool_coin_decimals as f64))
// }

// pool_pc_vault_amount
// pool_pc_decimals
// pool_coin_vault_amount
// pool_coin_decimals

type AccountAddressAndData = (Pubkey, Vec<u8>);

pub struct AccountDataReceiverConf {
    pub account_address: Pubkey,
    pub sender: Sender<AccountAddressAndData>,
    pub yellowstone_grpc_endpoint: String,
}

pub async fn get_account_data(conf: AccountDataReceiverConf) -> anyhow::Result<()> {
    let subscribe_req = SubscribeRequest {
        accounts: HashMap::from_iter(vec![(
            "".to_string(),
            SubscribeRequestFilterAccounts {
                account: vec![String::from(&conf.account_address.clone().to_string())],
                owner: vec![],
                filters: vec![],
            },
        )]),
        slots: Default::default(),
        transactions: Default::default(),
        blocks: Default::default(),
        blocks_meta: Default::default(),
        entry: Default::default(),
        commitment: None,
        accounts_data_slice: vec![],
        ping: None,
    };

    let mut client =
        GeyserGrpcClient::connect::<_, String>(conf.yellowstone_grpc_endpoint, None, None)?;
    let (mut sink, mut stream) = client.subscribe().await?;

    let account_address = conf.account_address.clone().to_string();
    let send = async move {
        sink.send(subscribe_req).await.inspect(|_| info!("Subscribed to account: {}", account_address))?;
        Ok::<(), anyhow::Error>(())
    };

    let account_address = conf.account_address.clone();
    let receive = async move {
        while let Some(msg) = stream.next().await.transpose().ok().flatten() {
            if let Some(UpdateOneof::Account(subscribe_update)) = msg.update_oneof {
                println!("Received update: {:#?}", &subscribe_update);

                let account_data = match subscribe_update.account {
                    None => continue,
                    Some(account_info) => {
                        conf.sender.send((account_address.clone(), account_info.data)).await.ok()
                    }
                };
            }
        }

        Ok::<(), anyhow::Error>(())
    };

    futures::try_join!(send, receive);
    Ok(())
}
