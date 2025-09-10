use async_graphql::http::GraphiQLSource;
use clap::Parser;
use std::{error::Error, fmt::Debug, sync::Arc};
use axum::{
    extract::Json, http::StatusCode, response::{self, IntoResponse, Response}, routing::{get, post}, Router
};
use async_graphql_axum::GraphQL;
use tower_http::{
    trace::TraceLayer,
    cors::{CorsLayer, Any},
};
use http::header;
use common::config::{Config, ImporterConfig, BackendConfig};
use crate::{
    importer::Importer,
    model::{FraudLevel, Importable, ImportableSerde, LabelSource, ModelId, WebTransaction},
    queue::QueueService,
    storage::{CommonStorage, ImportableStorage, WebStorage},
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
    println!("Starting with env:");
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
    println!("Starting...");
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
    match importer.import(transaction.clone()).await {
        Ok(id) => {
            tracing::info!("Successfully imported transaction with ID: {:?}", id);
            (StatusCode::OK, Json(id)).into_response()
        }
        Err(e) => {
            // Enhanced error logging with transaction information
            tracing::error!(
                error = %e,
                transaction_type = std::any::type_name::<I>(),
                "Failed to import transaction"
            );
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    }
}

pub async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "OK").into_response()
}

async fn graphiql() -> impl IntoResponse {
    response::Html(GraphiQLSource::build().endpoint("/api/transactions/graphql").finish())
}

pub async fn run_backend<T>(
    config: BackendConfig,
    storage: Arc<dyn WebStorage<T>>,
    common_storage: Arc<dyn CommonStorage>,
) -> Result<(), Box<dyn Error + Send + Sync>>
where
    T: WebTransaction + Send + Sync + Clone + 'static,
{
    let state = AppState {
        web_storage: storage,
        common_storage,
    };

    let schema = state.web_storage.get_seaography_schema();
    
    let app = Router::new()
        .route("/api/transactions/graphql", get(graphiql).post_service(GraphQL::new(schema)))
        
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
pub struct AppState<T: WebTransaction + Send + Sync + 'static> {
    web_storage: Arc<dyn WebStorage<T>>,
    common_storage: Arc<dyn CommonStorage>,
}

impl<T: WebTransaction + Send + Sync + 'static> AppState<T> {
    pub fn new(
        web_storage: Arc<dyn WebStorage<T>>,
        common_storage: Arc<dyn CommonStorage>,
    ) -> Self {
        Self {
            web_storage,
            common_storage,
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

pub async fn label_transaction<T: WebTransaction + Send + Sync>(
    axum::extract::State(state): axum::extract::State<AppState<T>>,
    Json(label_request): Json<LabelRequest>,
) -> Response 
{
    // Log the incoming request
    tracing::info!(
        transaction_ids = ?label_request.transaction_ids, 
        "Processing label request for {} transactions", 
        label_request.transaction_ids.len()
    );
    
    // Use the new business logic method
    match state.common_storage.label_transactions(
        &label_request.transaction_ids,
        label_request.fraud_level,
        label_request.fraud_category,
        LabelSource::Manual,
        label_request.labeled_by,
    ).await {
        Ok(()) => {
            tracing::info!("Successfully labeled all {} transactions", label_request.transaction_ids.len());
            StatusCode::OK.into_response()
        }
        Err(e) => {
            tracing::error!(
                error = %e,
                transaction_count = %label_request.transaction_ids.len(),
                "Failed to execute labeling operation"
            );
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    }
}
