use clap::Parser;
use std::{error::Error, fmt::Debug, sync::Arc};
use axum::{
    routing::{post, get},
    Router,
    extract::Json,
    response::IntoResponse,
    http::StatusCode,
};
use tower_http::trace::TraceLayer;
use common::config::{Config, ImporterConfig, BackendConfig};
use crate::{
    importer::Importer,
    model::{Importable, ImportableSerde, WebTransaction},
    queue::QueueService,
    storage::{ImportableStorage, WebStorage},
};

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
) -> Result<(), Box<dyn Error + Send + Sync>>
where
    T: WebTransaction + Clone + 'static,
{
    let app = Router::new()
        .route("/transactions", get(list_transactions::<T>))
        .route("/health", get(health_check))
        .layer(TraceLayer::new_for_http())
        .with_state(storage);

    tracing::info!("Starting importer service at {}", config.server_address);
    let listener = tokio::net::TcpListener::bind(&config.server_address).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

pub async fn list_transactions<T>(
    axum::extract::State(storage): axum::extract::State<Arc<dyn WebStorage<T>>>,
) -> impl IntoResponse
where
    T: WebTransaction,
{
    match storage.get_transactions().await {
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