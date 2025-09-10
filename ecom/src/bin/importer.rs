use std::error::Error;
use std::sync::Arc;

use processing::{
    executable_utils::{initialize_executable, run_importer},
    queue::ProdQueue,
};

use ecom::{
    import_model::ImportOrder,
    order_storage::OrderStorage,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("Starting importer...");
    let config = initialize_executable()?;
    let storage = Arc::new(OrderStorage::new(&config.common.database_url).await?);
    let queue = Arc::new(ProdQueue::new(&config.common.database_url).await?);
    run_importer::<ImportOrder>(config.importer, storage, queue).await
}
