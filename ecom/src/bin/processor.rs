use std::{error::Error, sync::Arc};

use processing::{
    executable_utils::{init_prometheus, initialize_executable, initialize_tracing},
    processor::Processor,
    scorers::ExpressionBasedScorer,
    storage::ProdCommonStorage,
};

use ecom::model::EcomOrder;
use metrics::gauge;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("Starting processor...");
    let config = initialize_executable()?;
    initialize_tracing(&config.processor.log_level);
    // Initialize Prometheus and spawn metrics server
    let handle = init_prometheus()?;
    let metrics_addr = config.processor.metrics_address.clone();
    tokio::spawn(async move {
        let router = axum::Router::new().route(
            "/metrics",
            axum::routing::get(move || {
                let h = handle.clone();
                async move { h.render() }
            }),
        );
        match tokio::net::TcpListener::bind(&metrics_addr).await {
            Ok(listener) => {
                if let Err(e) = axum::serve(listener, router).await {
                    eprintln!("Metrics server error: {}", e);
                }
            }
            Err(e) => eprintln!("Failed to bind metrics server at {}: {}", metrics_addr, e),
        }
    });
    // Report configured threads as a gauge
    {
        let g = gauge!("frida_processor_threads", "threads" => "count");
        g.set(config.processor.threads as f64);
    }
    let common_storage =
        Arc::new(ProdCommonStorage::<EcomOrder>::new(&config.common.database_url).await?);
    // Pick the first available channel (lowest id). Consider making this configurable.
    let scorer = ExpressionBasedScorer::new_init("Basic".to_string(), common_storage).await?;

    let processor =
        Processor::<EcomOrder, ExpressionBasedScorer<ProdCommonStorage<EcomOrder>>>::new(
            config.common,
            config.processor,
            scorer,
        )
        .await?;
    processor.start_processing_worker().await?;

    Ok(())
}
