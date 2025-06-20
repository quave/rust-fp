use clap::Parser;
use std::{error::Error, fmt::Debug, sync::Arc};
use axum::{
    routing::{post, get},
    Router,
    extract::Json,
    response::IntoResponse,
    http::StatusCode,
};
use tower_http::{
    trace::TraceLayer,
    cors::{CorsLayer, Any},
};
use http::header;
use common::config::{Config, ImporterConfig, BackendConfig};
use crate::{
    importer::Importer,
    model::{FraudLevel, Importable, ImportableSerde, Label, LabelSource, ModelId, ModelRegistryProvider, WebTransaction},
    queue::QueueService,
    storage::{CommonStorage, ImportableStorage, WebStorage}, ui_model::FilterRequest,
};
use chrono::Utc;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Path to config file
    #[arg(short, long, default_value = "target/debug/config/total_config.yaml")]
    pub config: String,
}

pub fn initialize_executable() -> Result<Config, Box<dyn Error + Send + Sync>> {
    // Add this at the very start, before any other code
    println!("Starting importer with env:");
    for (key, value) in std::env::vars() {
        println!("{key}={value}");
    }

    match std::env::current_dir() {
        Ok(dir) => println!("Current directory: {:?}", dir),
        Err(e) => eprintln!("Failed to get current directory: {}", e),
    }

    let args = Args::parse();
    println!("Loading config from: {}", args.config);
    let config = Config::load(&args.config)?;
    println!("Loaded config: {:#?}", config);

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env()
            .add_directive(tracing::Level::INFO.into()))
        .init();

    // At the start of main:
    println!("Starting importer...");
    println!("Working directory: {:?}", std::env::current_dir()?);
    println!(
        "Config directory: {:?}",
        std::env::var("CONFIG_DIR").unwrap_or_default()
    );
    println!(
        "Database URL: {:?}",
        std::env::var("DATABASE_URL").unwrap_or_default()
    );

    Ok(config)
}

pub async fn run_importer<I>(
    config: ImporterConfig,
    storage: Arc<dyn ImportableStorage<I>>,
    queue: Arc<dyn QueueService>,
) -> Result<(), Box<dyn Error + Send + Sync>>
where
    I: ImportableSerde + Clone + 'static,
{
    let importer = Importer::<I>::new(storage, queue);
    let app = Router::new()
        .route("/import", post(import_transaction::<I>))
        .route("/health", get(health_check))
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin("http://localhost:8080".parse::<header::HeaderValue>().unwrap())
                .allow_methods(Any)
                .allow_headers(Any)
        )
        .with_state(importer);

    tracing::info!("Starting importer service at {}", config.server_address);
    let listener = tokio::net::TcpListener::bind(&config.server_address).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

pub async fn import_transaction<I>(
    axum::extract::State(importer): axum::extract::State<Importer<I>>,
    Json(transaction): Json<I>,
) -> impl IntoResponse
where
    I: Importable + Clone,
{
    match importer.import(transaction).await {
        Ok(id) => {
            tracing::info!("Successfully imported transaction with ID: {:?}", id);
            (StatusCode::OK, Json(id)).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to import transaction: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    }
}

pub async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "OK").into_response()
}

pub async fn run_backend<T>(
    config: BackendConfig,
    storage: Arc<dyn WebStorage<T>>,
    common_storage: Arc<dyn CommonStorage>,
) -> Result<(), Box<dyn Error + Send + Sync>>
where
    T: WebTransaction + ModelRegistryProvider + Send + Sync + Clone + 'static,
{
    let state = AppState {
        web_storage: storage,
        common_storage,
    };
    
    let app = Router::new()
        .route("/api/transactions", get(list_transactions::<T>))
        .route("/api/transactions/label", post(label_transaction::<T>))
        .route("/health", get(health_check))
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin("http://localhost:5173".parse::<header::HeaderValue>().unwrap())
                .allow_methods(Any)
                .allow_headers(Any)
        )
        .with_state(state);

    tracing::info!("Starting backend service at {}", config.server_address);
    let listener = tokio::net::TcpListener::bind(&config.server_address).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// Move AppState outside the function so it's visible to handler functions
#[derive(Clone)]
pub struct AppState<T: WebTransaction + ModelRegistryProvider + Send + Sync + 'static> {
    web_storage: Arc<dyn WebStorage<T>>,
    common_storage: Arc<dyn CommonStorage>,
}

pub async fn list_transactions<T: ModelRegistryProvider>(
    axum::extract::State(state): axum::extract::State<AppState<T>>,
) -> impl IntoResponse
where
    T: WebTransaction + Send + Sync,
{
    match state.web_storage.get_transactions(FilterRequest::default()).await {
        Ok(transactions) => {
            tracing::trace!("Loaded list of transaction");
            (StatusCode::OK, Json(transactions)).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to import transaction: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    }
}

// Define the request structure for labeling
#[derive(serde::Deserialize)]
pub struct LabelRequest {
    pub transaction_ids: Vec<ModelId>,
    pub fraud_level: FraudLevel,
    pub fraud_category: String,
    pub labeled_by: String,
}

pub async fn label_transaction<T: ModelRegistryProvider>(
    axum::extract::State(state): axum::extract::State<AppState<T>>,
    Json(label_req): Json<LabelRequest>,
) -> impl IntoResponse 
where
    T: WebTransaction + Send + Sync,
{
    // Create label object
    let label = Label {
        id: 0, // Will be filled by the database
        fraud_level: label_req.fraud_level,
        fraud_category: label_req.fraud_category,
        label_source: LabelSource::Manual, // Always Manual when coming from the backend
        labeled_by: label_req.labeled_by,
        created_at: Utc::now(),
    };
    
    // Save the label and get its ID
    let save_result = state.common_storage.save_label(&label).await;
    
    match save_result {
        Ok(label_id) => {
            let mut success_count = 0;
            let mut failed_ids = Vec::new();
            
            // Apply the label to each transaction ID in the batch
            for transaction_id in &label_req.transaction_ids {
                match state.common_storage.update_transaction_label(*transaction_id, label_id).await {
                    Ok(_) => {
                        success_count += 1;
                        tracing::info!("Successfully labeled transaction {}: label_id={}", transaction_id, label_id);
                    },
                    Err(e) => {
                        tracing::error!("Failed to update transaction {} with label: {}", transaction_id, e);
                        failed_ids.push(*transaction_id);
                    }
                }
            }
            
            if failed_ids.is_empty() {
                (StatusCode::OK, Json(label_id)).into_response()
            } else {
                let message = format!(
                    "Partially successful: labeled {}/{} transactions. Failed IDs: {:?}",
                    success_count, 
                    label_req.transaction_ids.len(), 
                    failed_ids
                );
                (StatusCode::PARTIAL_CONTENT, message).into_response()
            }
        }
        Err(e) => {
            tracing::error!("Failed to save label: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    }
}