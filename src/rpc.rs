use async_trait::async_trait;
use solana_sdk::account::Account;
use solana_sdk::pubkey::Pubkey;

mod json_rpc;
pub mod yellowstone_grpc;

#[async_trait]
pub trait JsonRpcAccountReceiver: Send + Sync {
    async fn get_account(&self, pubkey: &Pubkey) -> anyhow::Result<Account>;

    async fn get_multiple_accounts(&self, pubkeys: &[Pubkey]) -> anyhow::Result<Vec<Account>>;

    async fn get_account_data(&self, pubkey: &Pubkey) -> anyhow::Result<Vec<u8>>;
}
