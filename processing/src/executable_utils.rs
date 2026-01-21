use crate::{
    importer::Importer,
    model::{FraudLevel, LabelSource, Processible, ProcessibleSerde},
    processor::Processor,
    queue::{ProdQueue, QueueName},
    scorers::{ExpressionBasedScorer, Scorer},
    storage::{CommonStorage, mongo_common::MongoCommonStorage},
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
use metrics::gauge;

use clap::Parser;
use common::config::Config;
use http::header;
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};
use mongodb::bson::oid::ObjectId;
use once_cell::sync::OnceCell;
use std::{error::Error, fmt::Debug, marker::PhantomData, sync::Arc};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing_subscriber::EnvFilter;
#[cfg(not(test))]
use tracing::{error, info};
#[cfg(test)]
use { println as info, println as error};


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

pub fn initialize_processor_metrics(config: &Config) -> Result<(), Box<dyn Error + Send + Sync>> {
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
    let g = gauge!("frida_processor_threads", "threads" => "count");
    g.set(config.processor.threads as f64);
    Ok(())
}

pub async fn run_processor<P: Processible + ProcessibleSerde<Id = ObjectId>>(config: Config) -> Result<(), Box<dyn Error + Send + Sync>> {
    initialize_tracing(&config.processor.log_level);
    // Initialize Prometheus and spawn metrics server
    initialize_processor_metrics(&config)?;

    let common_storage =
        Arc::new(MongoCommonStorage::new(&config.common.database_url, "frida").await?);
    // Pick the first available channel (lowest id). Consider making this configurable.


    let active_channels = common_storage.get_active_model_activations().await?;
    let scorers: Vec<Arc<dyn Scorer>> = active_channels
        .iter()
        .map(|channel| 
            Arc::new(ExpressionBasedScorer::new(channel.clone())) as Arc<dyn Scorer>
        )
        .collect();


    let processor =
        Arc::new(Processor::<P>::new(
            config.common,
            Arc::new(config.processor.clone()),
            scorers,
        ).await?);

    let mut set = tokio::task::JoinSet::new();
    for _ in 0..config.processor.threads {
        set.spawn(processor.clone().start_processing_worker());
    }
    info!("computation started");

    while let Some(join_res) = set.join_next().await {
        match join_res {
            Ok(Ok(())) => {
                info!("computation finished ok");
            }
            Ok(Err(e)) => {
                error!("computation finished with error: {:?}", e);
            }
            Err(e) => {
                error!("computation finished with join error: {:?}", e);
            }
        }
    }

    Ok(())
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
    config: Config
) -> Result<(), Box<dyn Error + Send + Sync>>
where
    P: Processible + ProcessibleSerde<Id = ObjectId>,
{
    initialize_tracing(&config.importer.log_level);
    let queue = Arc::new(ProdQueue::new(&config.common, QueueName::Processing).await?);

    let storage = Arc::new(MongoCommonStorage::new(&config.common.database_url, "frida").await?);
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
    P: Processible + ProcessibleSerde<Id = ObjectId> + Clone,
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
    config: Config
) -> Result<(), Box<dyn Error + Send + Sync>>
where
    P: Processible + ProcessibleSerde<Id = ObjectId> + Send + Sync + Clone + 'static,
{
    initialize_tracing(&config.backend.log_level);
    let common_storage: Arc<dyn CommonStorage<P::Id>> =
        Arc::new(MongoCommonStorage::new(&config.common.database_url, "frida").await?);

    let schema = crate::storage::graphql_schema::schema::<P>(common_storage.clone()).unwrap();

    let state = AppState {
        _phantom: PhantomData,
        common_storage,
    };

    // init prometheus and capture handle for /metrics
    let metrics_handle = init_prometheus()?;
    let metrics_path = config.backend.metrics_path.clone();
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

    tracing::info!("Starting backend service at {}", config.backend.server_address);
    let listener = tokio::net::TcpListener::bind(&config.backend.server_address).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// Move AppState outside the function so it's visible to handler functions
#[derive(Clone)]
pub struct AppState<T: Processible + Send + Sync + ProcessibleSerde<Id = ObjectId> + 'static> {
    // web_storage: Arc<dyn WebStorage<T>>,
    common_storage: Arc<dyn CommonStorage<T::Id>>,
    _phantom: PhantomData<T>,
}

impl<T: Processible + Send + Sync + ProcessibleSerde<Id = ObjectId> + 'static> AppState<T> {
    pub fn new(
        // web_storage: Arc<dyn WebStorage<T>>,
        common_storage: Arc<dyn CommonStorage<T::Id>>,
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
    pub payload_numbers: Vec<String>,
    pub fraud_level: FraudLevel,
    pub fraud_category: String,
    pub labeled_by: String,
}

pub async fn label_transaction<P: Processible + Send + Sync + ProcessibleSerde<Id = ObjectId>>(
    axum::extract::State(state): axum::extract::State<AppState<P>>,
    Json(label_request): Json<LabelRequest>,
) -> Response {
    // Log the incoming request
    tracing::info!(
        transaction_ids = ?label_request.payload_numbers,
        "Processing label request for {} transactions",
        label_request.payload_numbers.len()
    );

    // Use the new business logic method
    match state
        .common_storage
        .label_transactions(
            &label_request.payload_numbers,
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
                transaction_count = %label_request.payload_numbers.len(),
                "Failed to execute labeling operation"
            );
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    }
}
