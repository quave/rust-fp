use std::error::Error;
use std::sync::Arc;

use processing::{
    executable_utils::{initialize_executable, run_importer},
    queue::ProdQueue,
};

use ecom::{
    ecom_import_model::ImportOrder,
    ecom_order_storage::EcomOrderStorage,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("Starting importer...");
    let config = initialize_executable()?;
    
    // Create storage
    let storage = Arc::new(EcomOrderStorage::new(&config.common.database_url).await?);
    
    // Create queue
    let queue = Arc::new(ProdQueue::new(&config.common.database_url).await?);
    
    // Run with concrete types
    run_importer::<ImportOrder>(config.importer, storage, queue).await
}
