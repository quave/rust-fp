use std::error::Error;
use std::sync::Arc;

use ecom::model::EcomOrder;
use processing::executable_utils::{initialize_executable, run_backend};
use processing::storage::{CommonStorage, ProdCommonStorage};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("Starting backend...");
    let config = initialize_executable()?;
    let common_storage: Arc<dyn CommonStorage> = Arc::new(ProdCommonStorage::<EcomOrder>::new(&config.common.database_url).await?);
    run_backend::<EcomOrder>(config.backend, common_storage).await
}
