use async_trait::async_trait;
use solana_sdk::account::Account;
use solana_sdk::pubkey::Pubkey;

pub mod yellowstone_grpc;

#[async_trait]
pub trait GeyserPluginDataReceiver: Send + Sync {
    async fn get_account_data(&self, account_pubkey: &Pubkey) -> anyhow::Result<Vec<u8>>;
}

#[async_trait]
pub trait JsonRpcAccountReceiver: Send + Sync {
    async fn get_account(&self, pubkey: &Pubkey) -> anyhow::Result<Account>;
    async fn get_multiple_accounts(&self, pubkeys: &[Pubkey]) -> anyhow::Result<Vec<Account>>;
}


pub struct AccountDataReceiverMock;

#[async_trait]
impl GeyserPluginDataReceiver for AccountDataReceiverMock {
    async fn get_account_data(&self, _: &Pubkey) -> anyhow::Result<Vec<u8>> {
        todo!()
    }
}