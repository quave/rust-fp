use std::{error::Error, time::Duration};

use frida_core::{
    executable_utils::initialize_executable, model::Processible, processor::Processor,
    queue::ProdQueue, scorers::RuleBasedScorer, storage::ProdCommonStorage,
};
use frida_ecom::{
    ecom_db_model::Order, ecom_order_storage::EcomOrderStorage,
    rule_based_scorer::get_rule_based_scorer,
};
use log::{error, info, trace};
use std::sync::Arc;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let config = initialize_executable()?;

    // Create shared resources
    let common_storage = Arc::new(ProdCommonStorage::new(&config.common.database_url).await?);
    info!("Storage common initialized");

    let model_storage = Arc::new(EcomOrderStorage::new(&config.common.database_url).await?);
    info!("Storage model initialized");

    let queue = Arc::new(ProdQueue::new(&config.common.database_url).await?);
    info!("Queue initialized");

    let scorer = get_rule_based_scorer();
    info!("Scorer initialized");

    // Create processor
    let processor =
        Processor::<Order, RuleBasedScorer>::new(scorer, common_storage, model_storage, queue);
    let processor = std::sync::Arc::new(processor);
    info!("Processor created");

    // Spawn processing threads
    let mut handles = vec![];
    for i in 0..config.processor.threads {
        let processor = processor.clone();
        let sleep_ms = config.processor.sleep_ms;
        let handle = tokio::spawn(async move {
            info!("Starting processor thread {}", i);
            loop {
                match processor.process().await {
                    Ok(Some(transaction)) => {
                        info!("Processed transaction: {:?}", transaction.tx_id());
                    }
                    Ok(None) => {
                        trace!("No transactions to process, sleeping...");
                        sleep(Duration::from_millis(sleep_ms)).await;
                    }
                    Err(e) => {
                        error!("Error processing transaction: {}", e);
                        sleep(Duration::from_millis(sleep_ms)).await;
                    }
                }
            }
        });
        handles.push(handle);
    }

    info!("All processor threads spawned");

    // Wait for all threads
    for handle in handles {
        handle.await?;
    }

    Ok(())
}
