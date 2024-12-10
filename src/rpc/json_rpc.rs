use crate::rpc::JsonRpcAccountReceiver;
use anyhow::Context;
use async_trait::async_trait;
use raydium_amm::solana_program::pubkey::Pubkey;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::account::Account;
use solana_sdk::commitment_config::CommitmentConfig;

#[async_trait]
impl JsonRpcAccountReceiver for RpcClient {
    async fn get_account(&self, pubkey: &Pubkey) -> anyhow::Result<Account> {
        self.get_account_with_commitment(pubkey, CommitmentConfig::processed())
            .await?
            .value
            .context("no account found")
    }

    async fn get_multiple_accounts(&self, pubkeys: &[Pubkey]) -> anyhow::Result<Vec<Account>> {
        self.get_multiple_accounts(pubkeys)
            .await?
            .into_iter()
            .map(|account| account.context("no account found"))
            .collect::<Result<Vec<_>, _>>()
    }

    async fn get_account_data(&self, pubkey: &Pubkey) -> anyhow::Result<Vec<u8>> {
        Ok(self.get_account_data(pubkey).await?)
    }
}
