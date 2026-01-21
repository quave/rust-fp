use std::error::Error;
use processing::executable_utils::{run_processor, initialize_executable};
use ecom_f2::model::EcomF2Order;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("Starting processor...");
    let config = initialize_executable()?;
    run_processor::<EcomF2Order>(config).await
}
