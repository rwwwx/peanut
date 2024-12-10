use chrono::NaiveDateTime;
use solana_sdk::pubkey::Pubkey;
use sqlx::FromRow;

#[derive(Clone, Debug, Default, PartialEq, FromRow)]
pub struct PoolAndPrice {
    pub pool_pubkey: Pubkey,
    pub price: f64,
    pub updated_at: NaiveDateTime,
}

impl PoolAndPrice {
    pub fn new(pool_pubkey: Pubkey, price: f64, updated_at: NaiveDateTime) -> Self {
        Self { pool_pubkey, price, updated_at }
    }
}
