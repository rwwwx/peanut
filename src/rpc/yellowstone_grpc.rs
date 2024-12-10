use futures::{SinkExt, StreamExt};
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use tokio::sync::mpsc::Sender;
use tracing::{debug, info};
use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::geyser::{SubscribeRequest, SubscribeRequestFilterAccounts};
use yellowstone_grpc_proto::prelude::subscribe_update::UpdateOneof;

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

    let mut client = GeyserGrpcClient::connect::<_, String>(conf.yellowstone_grpc_endpoint, None, None)?;
    let (mut sink, mut stream) = client.subscribe().await?;

    let account_address = conf.account_address.clone().to_string();
    let send = async move {
        sink.send(subscribe_req.clone())
            .await
            .inspect(|_| info!("Subscribed to pool: {}", account_address))?;

        Ok::<(), anyhow::Error>(())
    };

    let account_address = conf.account_address.clone();
    let receive = async move {
        while let Some(msg) = stream.next().await.transpose().ok().flatten() {
            if let Some(UpdateOneof::Account(subscribe_update)) = msg.update_oneof {
                debug!("Received update {account_address}", account_address = account_address.to_string());

                let Some(account_info) = subscribe_update.account else {
                    continue;
                };

                conf.sender
                    .send((account_address.clone(), account_info.data))
                    .await
                    .ok();
            }
        }

        Ok::<(), anyhow::Error>(())
    };

    futures::try_join!(send, receive).ok();
    Ok(())
}
