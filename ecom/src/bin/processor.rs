use std::error::Error;
use std::sync::Arc;
use std::time::Duration;

use processing::{
    executable_utils::initialize_executable,
    processor::Processor,
    queue::ProdQueue,
    scorers::RuleBasedScorer,
    storage::ProdCommonStorage,
};

use ecom::{
    ecom_db_model::Order,
    ecom_order_storage::EcomOrderStorage,
    rule_based_scorer::get_rule_based_scorer,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
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
    let queue = Arc::new(ProdQueue::new(&config.common.database_url).await?);
    
    // Create processor
    let processor = Processor::<Order, RuleBasedScorer>::new(
        get_rule_based_scorer(),
        common_storage,
        model_storage,
        queue,
    );
    
    // Run processor
    loop {
        match processor.process().await {
            Ok(Some(_)) => continue,
            Ok(None) => tokio::time::sleep(Duration::from_millis(config.processor.sleep_ms)).await,
            Err(e) => {
                eprintln!("Error processing: {}", e);
                tokio::time::sleep(Duration::from_millis(config.processor.sleep_ms)).await;
            }
        }
    }
}
