pub mod postgres;

use crate::models::PoolAndPrice;
use async_trait::async_trait;
pub use postgres::PostgresStorage;
use solana_sdk::pubkey::Pubkey;
use std::time::Duration;

#[async_trait]
pub trait PoolPriceStorage {
    async fn save(&self, price_for_pool: &PoolAndPrice) -> anyhow::Result<Pubkey>;
    async fn average(&self, pool_pubkey: &Pubkey, for_period: Duration) -> anyhow::Result<f64>;
    async fn current(&self, pool_pubkey: &Pubkey) -> anyhow::Result<f64>;
}

#[async_trait]
pub trait OldRecordCleaner {
    async fn clear_old_records(&self, in_interval: Duration) -> anyhow::Result<()>;
}
