use std::error::Error;
use std::sync::Arc;

use processing::executable_utils::{initialize_executable, run_backend};

use ecom::{
    ecom_db_model::Order, ecom_order_storage::EcomOrderStorage
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let config = initialize_executable()?;
    
    // Create storage
    let storage = Arc::new(EcomOrderStorage::new(&config.common.database_url).await?);
    
    // Run with concrete types
    run_backend::<Order>(config.backend, storage).await
}
