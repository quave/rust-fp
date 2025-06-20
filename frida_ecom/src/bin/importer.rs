use std::error::Error;

use frida_core::{
    executable_utils::{initialize_executable, run_importer},
    queue::ProdQueue,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let config = initialize_executable()?;

    // Run with concrete types
    use frida_ecom::{
        ecom_import_model::ImportOrder, ecom_order_storage::EcomOrderStorage,
    };
    let storage = Arc::new(EcomOrderStorage::new(&config.common.database_url).await?);
    // storage.initialize_schema().await?;
    let queue = Arc::new(ProdQueue::new(&config.common.database_url).await?);
    log::info!("Queue initialized");

    run_importer::<ImportOrder>(config.importer, storage, queue).await
}
