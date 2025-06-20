use std::error::Error;

use frida_core::{
    executable_utils::{initialize_executable, run_importer},
    queue::ProdQueue,
    storage::ImportableStorage,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let config = initialize_executable()?;

    // Run with concrete types
    use frida_ecom::{
        ecom_db_model::Order, ecom_import_model::ImportOrder,
        sqlite_order_storage::SqliteOrderStorage,
    };
    let storage = Arc::new(SqliteOrderStorage::new(&config.common.database_url).await?);
    storage.initialize_schema().await?;
    let queue = Arc::new(ProdQueue::new(&config.common.database_url).await?);
    log::info!("Queue initialized");

    run_importer::<ImportOrder, Order>(config.importer, storage, queue).await
}
