use std::error::Error;
use std::sync::Arc;

use processing::executable_utils::{initialize_executable, run_backend};
use processing::storage::{CommonStorage, ProdCommonStorage};

use ecom::{
    processible::EcomOrder, order_storage::OrderStorage
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("Starting backend...");
    let config = initialize_executable()?;
    let web_storage = Arc::new(OrderStorage::new(&config.common.database_url).await?);
    let common_storage: Arc<dyn CommonStorage> = Arc::new(ProdCommonStorage::new(&config.common.database_url).await?);
    run_backend::<EcomOrder>(config.backend, web_storage, common_storage).await
}
