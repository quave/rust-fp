use std::error::Error;
use ecom_f2::model::EcomF2Order;
use processing::executable_utils::{initialize_executable, run_backend};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("Starting backend...");
    let config = initialize_executable()?;
    run_backend::<EcomF2Order>(config).await
}
