use clap::Parser;
use frida_core::{
    in_memory_queue::InMemoryQueue, model::Processible, processor::Processor,
    queue_service::QueueService, storage::Storage,
};
use frida_ecom::{
    config::ProcessorConfig, ecom_db_model::Order, ecom_import_model::ImportOrder,
    rule_based_scorer::RuleBasedScorer, sqlite_order_storage::SqliteOrderStorage,
};
use std::{error::Error, time::Duration};
use tokio::time::sleep;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to config file
    #[arg(short, long, default_value = "frida_ecom/config/processor.toml")]
    config: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args = Args::parse();
    let config = ProcessorConfig::from_file(&args.config)?;

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(&config.log_level))
        .init();

    // Create shared resources
    let storage = SqliteOrderStorage::new(&config.database_url).await?;
    storage.initialize_schema().await?;
    let queue: InMemoryQueue<Order> = InMemoryQueue::new();
    let scorer = RuleBasedScorer::new();

    // Create processor
    let processor = Processor::<
        Order,
        ImportOrder,
        RuleBasedScorer,
        SqliteOrderStorage,
        InMemoryQueue<Order>,
    >::new(scorer, storage, queue);
    let processor = std::sync::Arc::new(processor);

    // Spawn processing threads
    let mut handles = vec![];
    for i in 0..config.threads {
        let processor = processor.clone();
        let sleep_ms = config.sleep_ms;
        let handle = tokio::spawn(async move {
            log::info!("Starting processor thread {}", i);
            loop {
                match processor.process().await {
                    Ok(Some(transaction)) => {
                        log::info!("Processed transaction: {:?}", transaction.get_id());
                    }
                    Ok(None) => {
                        log::debug!("No transactions to process, sleeping...");
                        sleep(Duration::from_millis(sleep_ms)).await;
                    }
                    Err(e) => {
                        log::error!("Error processing transaction: {}", e);
                        sleep(Duration::from_millis(sleep_ms)).await;
                    }
                }
            }
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.await?;
    }

    Ok(())
}
