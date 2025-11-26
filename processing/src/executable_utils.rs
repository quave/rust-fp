use crate::{
    importer::Importer,
    model::{FraudLevel, LabelSource, ModelId, Processible, ProcessibleSerde},
    queue::QueueService,
    storage::{CommonStorage, ProdCommonStorage},
};
use async_graphql::http::GraphiQLSource;
use async_graphql_axum::GraphQL;
use axum::{
    Router,
    extract::Json,
    http::StatusCode,
    response::{self, IntoResponse, Response},
    routing::{get, post},
};
use clap::Parser;
use common::config::{BackendConfig, Config};
use http::header;
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};
use once_cell::sync::OnceCell;
use std::{error::Error, fmt::Debug, marker::PhantomData, sync::Arc};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing_subscriber::EnvFilter;

static PROM_HANDLE: OnceCell<PrometheusHandle> = OnceCell::new();

pub fn init_prometheus() -> Result<PrometheusHandle, Box<dyn Error + Send + Sync>> {
    if let Some(h) = PROM_HANDLE.get() {
        return Ok(h.clone());
    }

    let builder = PrometheusBuilder::new()
        // Buckets for importer
        .set_buckets_for_metric(
            Matcher::Full("frida_import_duration_seconds".to_string()),
            &[0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0],
        )?
        // Buckets for processing
        .set_buckets_for_metric(
            Matcher::Full("frida_processing_stage_seconds".to_string()),
            &[0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0],
        )?
        // Buckets for recalculation
        .set_buckets_for_metric(
            Matcher::Full("frida_recalc_stage_seconds".to_string()),
            &[0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0],
        )?
        // Buckets for backend filter
        .set_buckets_for_metric(
            Matcher::Full("frida_backend_filter_seconds".to_string()),
            &[0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5],
        )?;

    let handle = builder
        .install_recorder()
        .map_err(|e| format!("Failed to install Prometheus recorder: {}", e))?;
    let _ = PROM_HANDLE.set(handle.clone());
    Ok(handle)
}

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
    let mut config = Config::load(&args.config)?;
    if let Ok(db_url) = std::env::var("DATABASE_URL") {
        if !db_url.trim().is_empty() {
            println!("Overriding config.common.database_url from env DATABASE_URL");
            config.common.database_url = db_url;
        }
    }
    println!("Loaded config: {:#?}", config);

    Ok(config)
}

pub fn initialize_tracing(log_directive: &str) {
    let env_filter = EnvFilter::from_default_env().add_directive(log_directive.parse().unwrap());

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_thread_ids(true)
        .with_writer(std::io::stdout)
        .init();
}

pub async fn run_importer<P>(
    config: Config,
    queue: Arc<dyn QueueService>,
) -> Result<(), Box<dyn Error + Send + Sync>>
where
    P: Processible + ProcessibleSerde,
{
    let storage = Arc::new(ProdCommonStorage::<P>::new(&config.common.database_url).await?);
    let importer = Importer::<P>::new(storage, queue);
    // init prometheus and capture handle for /metrics
    let metrics_handle = init_prometheus()?;
    let metrics_path = config.importer.metrics_path.clone();
    let app = Router::new()
        .route("/import", post(import_transaction::<P>))
        .route("/health", get(health_check))
        .route(
            &metrics_path,
            get(move || {
                let h = metrics_handle.clone();
                async move { h.render() }
            }),
        )
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(
                    "http://localhost:8080"
                        .parse::<header::HeaderValue>()
                        .unwrap(),
                )
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(importer);

    tracing::info!(
        "Starting importer service at {}",
        config.importer.server_address
    );
    let listener = tokio::net::TcpListener::bind(&config.importer.server_address).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

pub async fn import_transaction<P>(
    axum::extract::State(importer): axum::extract::State<Importer<P>>,
    Json(transaction): Json<P>,
) -> Response
where
    P: Processible + ProcessibleSerde + Clone,
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
                transaction_type = std::any::type_name::<P>(),
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
    response::Html(
        GraphiQLSource::build()
            .endpoint("/api/transactions/graphql")
            .finish(),
    )
}

pub async fn run_backend<P>(
    config: BackendConfig,
    common_storage: Arc<dyn CommonStorage>,
) -> Result<(), Box<dyn Error + Send + Sync>>
where
    P: Processible + ProcessibleSerde + Send + Sync + Clone + 'static,
{
    let schema = crate::storage::graphql_schema::schema::<P>(common_storage.clone()).unwrap();

    let state = AppState {
        _phantom: PhantomData,
        common_storage,
    };

    // init prometheus and capture handle for /metrics
    let metrics_handle = init_prometheus()?;
    let metrics_path = config.metrics_path.clone();
    let app = Router::new()
        .route(
            "/api/transactions/graphql",
            get(graphiql).post_service(GraphQL::new(schema)),
        )
        .route("/api/transactions/label", post(label_transaction::<P>))
        .route("/health", get(health_check))
        .route(
            &metrics_path,
            get(move || {
                let h = metrics_handle.clone();
                async move { h.render() }
            }),
        )
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(
                    "http://localhost:5173"
                        .parse::<header::HeaderValue>()
                        .unwrap(),
                )
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(state);

    tracing::info!("Starting backend service at {}", config.server_address);
    let listener = tokio::net::TcpListener::bind(&config.server_address).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// Move AppState outside the function so it's visible to handler functions
#[derive(Clone)]
pub struct AppState<T: Processible + Send + Sync + 'static> {
    // web_storage: Arc<dyn WebStorage<T>>,
    common_storage: Arc<dyn CommonStorage>,
    _phantom: PhantomData<T>,
}

impl<T: Processible + Send + Sync + 'static> AppState<T> {
    pub fn new(
        // web_storage: Arc<dyn WebStorage<T>>,
        common_storage: Arc<dyn CommonStorage>,
    ) -> Self {
        Self {
            // web_storage,
            common_storage,
            _phantom: PhantomData,
        }
    }
}

// Define the request structure for labeling
#[derive(serde::Deserialize, Debug)]
pub struct LabelRequest {
    pub transaction_ids: Vec<ModelId>,
    pub fraud_level: FraudLevel,
    pub fraud_category: String,
    pub labeled_by: String,
}

pub async fn label_transaction<P: Processible + Send + Sync>(
    axum::extract::State(state): axum::extract::State<AppState<P>>,
    Json(label_request): Json<LabelRequest>,
) -> Response {
    // Log the incoming request
    tracing::info!(
        transaction_ids = ?label_request.transaction_ids,
        "Processing label request for {} transactions",
        label_request.transaction_ids.len()
    );

    // Use the new business logic method
    match state
        .common_storage
        .label_transactions(
            &label_request.transaction_ids,
            &label_request.fraud_level,
            &label_request.fraud_category,
            &LabelSource::Manual,
            &label_request.labeled_by,
        )
        .await
    {
        Ok(transaction_ids) => {
            tracing::info!(
                "Successfully labeled {:?} {:?} transactions: {:?}",
                label_request.fraud_level,
                &label_request.fraud_category,
                transaction_ids
            );
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
