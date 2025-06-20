use std::error::Error;
use std::sync::Arc;

use processing::executable_utils::{initialize_executable, run_backend};
use processing::storage::{CommonStorage, ProdCommonStorage};

use ecom::{
    ecom_db_model::Order, ecom_order_storage::EcomOrderStorage
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let config = initialize_executable()?;
    
    // Create storage
    let web_storage = Arc::new(EcomOrderStorage::new(&config.common.database_url).await?);
    
    // Create common storage
    let common_storage: Arc<dyn CommonStorage> = Arc::new(ProdCommonStorage::new(&config.common.database_url).await?);
    
    // Run with concrete types
    run_backend::<Order>(config.backend, web_storage, common_storage).await
}
