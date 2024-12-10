use crate::config::Settings;
use crate::models::PoolAndPrice;
use crate::storage::{OldRecordCleaner, PoolPriceStorage};
use crate::trait_ext::duration_ext::DurationExt;
use anyhow::{anyhow, Context};
use async_trait::async_trait;
use solana_sdk::pubkey::Pubkey;
use sqlx::postgres::PgPoolOptions;
use sqlx::{query, PgPool, Row};
use std::time::Duration;
use tokio::spawn;
use tokio::time::{interval, sleep};
use tracing::{error, info};

pub struct PostgresStorage {
    pg_pool: PgPool,
}

#[async_trait]
impl PoolPriceStorage for PostgresStorage {
    async fn save(&self, price_and_pool: &PoolAndPrice) -> anyhow::Result<Pubkey> {
        let sql = r#"
            INSERT INTO raydium_pools_prices
            (
                pool_pk,
                price,
                updated_at
            )
            VALUES ($1, $2, $3)
            RETURNING pool_pk
        "#;

        let pool_pk = query(sql)
            .bind(price_and_pool.pool_pubkey.to_bytes())
            .bind(price_and_pool.price)
            .bind(price_and_pool.updated_at)
            .fetch_one(&self.pg_pool)
            .await?
            .try_get::<Vec<u8>, _>("pool_pk")
            .context("Failed to get 'pool_pk'")?;

        Pubkey::try_from(pool_pk)
            .ok()
            .context("Cannot parse pool_pk when insert")
    }

    async fn average(&self, pool_pubkey: &Pubkey, for_interval: Duration) -> anyhow::Result<f64> {
        let sql = r#"
            SELECT AVG(price)
            FROM raydium_pools_prices
            WHERE
                updated_at >= NOW() - $1::interval
                AND
                pool_pk = $2
        "#;

        let Some(average_value) = query(sql)
            .bind(for_interval)
            .bind(pool_pubkey.to_bytes())
            .fetch_one(&self.pg_pool)
            .await?
            .try_get::<Option<f64>, _>("avg")?
        else {
            return Ok(-1.0);
        };

        Ok(average_value)
    }

    async fn current(&self, pool_pubkey: &Pubkey) -> anyhow::Result<f64> {
        let sql = r#"
            SELECT price
            FROM raydium_pools_prices
            WHERE pool_pk = $1
            ORDER BY updated_at DESC
            LIMIT 1
        "#;

        let Some(average) = query(sql)
            .bind(pool_pubkey.to_bytes())
            .fetch_optional(&self.pg_pool)
            .await?
        else {
            return Ok(-1.0)
        };

        Ok(average.try_get::<_, _>("price").context("Failed to get 'price'")?)

    }
}

#[async_trait]
impl OldRecordCleaner for PostgresStorage {
    async fn clear_old_records(&self, for_period: Duration) -> anyhow::Result<()> {
        self.run_old_records_cleaner_in_background(for_period)
    }
}

impl PostgresStorage {
    pub async fn from_settings(settings: &Settings) -> anyhow::Result<Self> {
        let storage = Self::new(
            PgPoolOptions::new()
                .max_connections(settings.database.max_connections)
                .min_connections(settings.database.min_connections)
                .connect(&settings.database.connection_url)
                .await
                .inspect(|_| info!("Connected to DB"))?,
        );

        Ok(storage)
    }

    fn new(pg_pool: PgPool) -> Self {
        Self { pg_pool }
    }

    fn run_old_records_cleaner_in_background(&self, for_interval: Duration) -> anyhow::Result<()> {
        spawn(Self::start_old_records_cleaner(self.pg_pool.clone(), for_interval));
        Ok(())
    }

    async fn start_old_records_cleaner(executor: PgPool, for_interval: Duration) -> anyhow::Result<()> {
        let delay_for_synchronize = sleep(for_interval);
        delay_for_synchronize.await;

        let mut repeat_interval = interval(Duration::from_minutes(30));

        loop {
            repeat_interval.tick().await;
            match Self::clear_old_records(executor.clone(), for_interval).await {
                Ok(rows_affected) => info!("Cleared '{rows_affected}' old records."),
                Err(e) => error!("Failed to clear old records. Cause: {e}"),
            };
        }
    }

    async fn clear_old_records(executor: PgPool, for_interval: Duration) -> anyhow::Result<u64> {
        let sql = r#"
            DELETE FROM raydium_pools_prices
            WHERE updated_at < NOW() - $1::interval
        "#;

        Ok(query(sql)
            .bind(for_interval)
            .execute(&executor)
            .await
            .map_err(|e| anyhow!("{e}"))?
            .rows_affected())
    }

    #[cfg(test)]
    pub async fn refresh_table(pg_pool: PgPool) -> anyhow::Result<Self> {
        query("DELETE FROM raydium_pools_prices").execute(&pg_pool).await?;

        Ok(Self::new(pg_pool))
    }
}

impl From<PgPool> for PostgresStorage {
    fn from(pool: PgPool) -> Self {
        Self { pg_pool: pool }
    }
}

#[cfg(test)]
mod test {
    use crate::models::PoolAndPrice;
    use crate::storage::{PoolPriceStorage, PostgresStorage};
    use crate::trait_ext::duration_ext::DurationExt;
    use chrono::Utc;
    use rand::Rng;
    use solana_sdk::pubkey::Pubkey;
    use sqlx::postgres::PgPoolOptions;
    use sqlx::PgPool;
    use std::thread::sleep;
    use std::time::Duration;

    const PUBLIC_KEY_OF_POOL: &str = "EP2ib6dYdEeqD8MfE2ezHCxX3kP3K2eLKkirfPm5eyMx";
    const MIN_PRICE: f64 = 100.0;
    const MAX_PRICE: f64 = 100_000.00;
    const AMOUNT_OF_TEST_RECORDS: usize = 10;

    async fn get_pg_pool() -> PgPool {
        PgPoolOptions::new()
            .max_connections(5)
            .min_connections(1)
            .connect("postgres://admin:admin@localhost:5432/peanut")
            .await
            .expect("Failed to connect to Postgres")
    }

    fn get_random_price() -> f64 {
        rand::thread_rng().gen_range(MIN_PRICE..=MAX_PRICE)
    }

    fn create_test_models() -> Vec<PoolAndPrice> {
        let mut res = Vec::with_capacity(AMOUNT_OF_TEST_RECORDS);
        for _ in 0..AMOUNT_OF_TEST_RECORDS {
            sleep(Duration::from_millis(100));
            res.push(PoolAndPrice::new(Pubkey::try_from(PUBLIC_KEY_OF_POOL).unwrap(), get_random_price(), Utc::now().naive_utc()))
        }

        res
    }

    #[tokio::test]
    async fn save_test() {
        let storage = PostgresStorage::refresh_table(get_pg_pool().await)
            .await
            .expect("Failed to fresh table");
        for model in create_test_models() {
            assert!(storage.save(&model).await.is_ok());
        }
    }

    #[tokio::test]
    async fn average_test() {
        let storage = PostgresStorage::refresh_table(get_pg_pool().await)
            .await
            .expect("Failed to fresh table");

        let records_for_test = create_test_models();
        let expected_average =
            records_for_test.iter().map(|model| model.price).sum::<f64>() / records_for_test.len() as f64;

        for model in records_for_test {
            assert!(storage.save(&model).await.inspect_err(|x| eprintln!("{x}")).is_ok());
        }

        let actual_average = storage
            .average(&Pubkey::try_from(PUBLIC_KEY_OF_POOL).unwrap(), Duration::from_minutes(5))
            .await
            .expect("Unable to average");
        assert_eq!(expected_average, actual_average);
    }

    #[tokio::test]
    async fn current_test() {
        let storage = PostgresStorage::refresh_table(get_pg_pool().await)
            .await
            .expect("Failed to fresh table");

        let records_for_test = create_test_models();
        let expected_current = records_for_test.last().expect("Unable to get last record").price;
        for model in records_for_test {
            assert!(storage.save(&model).await.inspect_err(|x| eprintln!("{x}")).is_ok());
        }

        let actual_current = storage.current(&Pubkey::try_from(PUBLIC_KEY_OF_POOL).unwrap()).await.expect("Unable to average");
        assert_eq!(expected_current, actual_current);
    }
}
