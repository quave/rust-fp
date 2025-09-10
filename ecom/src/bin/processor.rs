use std::error::Error;
use std::sync::Arc;

use processing::{
    executable_utils::initialize_executable,
    processor::Processor,
    scorers::ExpressionBasedScorer,
};

use ecom::{
    processible::EcomOrder,
    order_storage::OrderStorage,
    expression_based_scorer::get_expression_based_scorer,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("Starting processor...");
    let config = initialize_executable()?;
    let model_storage = Arc::new(OrderStorage::new(&config.common.database_url).await?);
    let processor = Processor::<EcomOrder, ExpressionBasedScorer>::new(
        config.common,
        config.processor,
        get_expression_based_scorer(),
        model_storage,
    ).await?;
    processor.start_processing_worker().await?;

    Ok(())
}
