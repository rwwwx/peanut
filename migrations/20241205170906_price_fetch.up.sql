-- Add up migration script here
CREATE TABLE IF NOT EXISTS raydium_pools_prices(
    pool_pk BYTEA NOT NULL,
    price DOUBLE PRECISION NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_metrics_created_at ON raydium_pools_prices (updated_at);
