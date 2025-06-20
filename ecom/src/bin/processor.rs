use std::error::Error;
use std::sync::Arc;

use processing::{
    executable_utils::initialize_executable,
    processor::Processor,
    queue::ProdQueue,
    scorers::ExpressionBasedScorer,
    storage::ProdCommonStorage,
};

use ecom::{
    ecom_db_model::Order,
    ecom_order_storage::EcomOrderStorage,
    expression_based_scorer::get_expression_based_scorer,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("Starting processor...");
    let config = initialize_executable()?;
    
    // Create matcher configs from processor config if available
    let matcher_configs = if let Some(configs) = &config.processor.matcher_configs {
        configs.clone()
    } else {
        std::collections::HashMap::new()
    };
    
    // Create common storage with matcher configs
    let common_storage = Arc::new(
        if !matcher_configs.is_empty() {
            ProdCommonStorage::with_configs(&config.common.database_url, matcher_configs).await?
        } else {
            ProdCommonStorage::new(&config.common.database_url).await?
        }
    );
    
    // Create model storage
    let model_storage = Arc::new(EcomOrderStorage::new(&config.common.database_url).await?);
    
    // Create queue
    let proc_queue = Arc::new(ProdQueue::new(&config.common.database_url).await?);
    let recalc_queue = Arc::new(ProdQueue::new(&config.common.database_url).await?);
    // Create processor
    let processor = Processor::<Order, ExpressionBasedScorer>::new(
        config.processor,
        get_expression_based_scorer(),
        common_storage,
        model_storage,
        proc_queue,
        recalc_queue,
    );
    
    // Run processor
    processor.start_processing_worker().await?;

    Ok(())
}
