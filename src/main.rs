use crate::account_data_receiver::AccountDataReceiverMock;
use crate::arced::Arced;
use crate::config::Settings;
use crate::price_fetcher::PriceFetchService;
use crate::storage::PostgresStorage;
use std::io;
use std::sync::Arc;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

mod account_data_receiver;
mod config;
mod duration_ext;
mod models;
mod price_fetcher;
mod storage;
mod supervisor;
mod arced;

#[tokio::main]
async fn main() -> io::Result<()> {
    let settings =
        Settings::load(None, Some("peanut/src/config")).unwrap_or_else(|e| panic!("Configuration failed: '{e}'!"));
    set_up_logging("info");

    let (storage, account_data) = (
        PostgresStorage::from_settings(&settings)
            .await
            .expect("Can't create storage")
            .arced(),
        AccountDataReceiverMock.arced(),
    );

    let fetcher = PriceFetchService::from_settings(&settings, storage.clone(), storage, account_data)
        .await
        .expect("Can't create price fetch service");
    fetcher.start_price_fetching().expect("Can't start price fetch");
    loop {}
    Ok(())
}

fn set_up_logging(log_level: &str) {
    let fmt_layer = tracing_subscriber::fmt::layer()
        .pretty()
        .with_thread_names(true)
        .with_writer(Arc::new(io::stdout()));

    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(EnvFilter::new(log_level))
        .init();
}
