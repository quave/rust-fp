use std::{error::Error, sync::Arc};

use processing::{
    executable_utils::{initialize_executable, initialize_tracing, run_importer},
    queue::ProdQueue,
};

use ecom::{
    model::EcomOrder,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("Starting importer...");
    let config = initialize_executable()?;
    initialize_tracing(&config.importer.log_level);
    let queue = Arc::new(ProdQueue::new(&config.common.database_url).await?);
    run_importer::<EcomOrder>(config, queue).await
}
