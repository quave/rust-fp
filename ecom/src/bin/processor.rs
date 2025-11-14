use std::error::Error;

use processing::{
    executable_utils::initialize_executable,
    processor::Processor,
    scorers::ExpressionBasedScorer,
};

use ecom::{
    expression_based_scorer::get_expression_based_scorer,
    model::EcomOrder,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("Starting processor...");
    let config = initialize_executable()?;
    let processor = Processor::<EcomOrder, ExpressionBasedScorer>::new(
        config.common,
        config.processor,
        get_expression_based_scorer(),
    ).await?;
    processor.start_processing_worker().await?;

    Ok(())
}
