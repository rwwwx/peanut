-- Add down migration script here
DROP INDEX IF EXISTS idx_metrics_created_at;

DROP TABLE IF EXISTS raydium_pools_prices;