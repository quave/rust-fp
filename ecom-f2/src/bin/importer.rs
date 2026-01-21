use std::error::Error;
use processing::executable_utils::{initialize_executable, run_importer};
use ecom_f2::model::EcomF2Order;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("Starting importer...");
    let config = initialize_executable()?;
    run_importer::<EcomF2Order>(config).await
}
