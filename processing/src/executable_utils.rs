use async_graphql::http::GraphiQLSource;
use clap::Parser;
use std::{error::Error, fmt::Debug, marker::PhantomData, sync::Arc};
use axum::{
    extract::Json, http::StatusCode, response::{self, IntoResponse, Response}, routing::{get, post}, Router
};
use async_graphql_axum::GraphQL;
use tower_http::{
    trace::TraceLayer,
    cors::{CorsLayer, Any},
};
use http::header;
use common::config::{Config, BackendConfig};
use crate::{
    importer::Importer,
    model::{FraudLevel, LabelSource, ModelId, Processible, ProcessibleSerde},
    queue::QueueService,
    storage::{CommonStorage, ProdCommonStorage},
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

pub async fn run_importer<P>(
    config: Config,
    queue: Arc<dyn QueueService>,
) -> Result<(), Box<dyn Error + Send + Sync>>
where
    P: Processible + ProcessibleSerde,
{
    let storage = Arc::new(ProdCommonStorage::<P>::new(&config.common.database_url).await?);
    let importer = Importer::<P>::new(storage, queue);
    let app = Router::new()
        .route("/import", post(import_transaction::<P>))
        .route("/health", get(health_check))
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin("http://localhost:8080".parse::<header::HeaderValue>().unwrap())
                .allow_methods(Any)
                .allow_headers(Any)
        )
        .with_state(importer);

    tracing::info!("Starting importer service at {}", config.importer.server_address);
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
    response::Html(GraphiQLSource::build().endpoint("/api/transactions/graphql").finish())
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

    let app = Router::new()
        .route("/api/transactions/graphql", get(graphiql).post_service(GraphQL::new(schema)))
        
        .route("/api/transactions/label", post(label_transaction::<P>))
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
#[derive(serde::Deserialize)]
pub struct LabelRequest {
    pub transaction_ids: Vec<ModelId>,
    pub fraud_level: FraudLevel,
    pub fraud_category: String,
    pub labeled_by: String,
}

pub async fn label_transaction<P: Processible + Send + Sync>(
    axum::extract::State(state): axum::extract::State<AppState<P>>,
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


// Positioned {
// pos: Pos(40:5),
// node: Field(Positioned {
//     pos: Pos(40:5),
//     node: Field {
//     alias: None,
//     name: Positioned {
//         pos: Pos(40:5),
//         node: Name("order_number")
//     },
//     arguments: [],
//     directives: [],
//     selection_set: Positioned {
//         pos: Pos(0:0),
//         node: SelectionSet {
//         items: []
//         }
//     }
//     }
// })
// },
// Positioned {
// pos: Pos(41:5),
// node: Field(Positioned {
//     pos: Pos(41:5),
//     node: Field {
//     alias: None,
//     name: Positioned {
//         pos: Pos(41:5),
//         node: Name("items")
//     },
//     arguments: [],
//     directives: [],
//     selection_set: Positioned {
//         pos: Pos(41:11),
//         node: SelectionSet {
//         items: [
//             Positioned {
//             pos: Pos(42:7),
//             node: Field(Positioned {
//                 pos: Pos(42:7),
//                 node: Field {
//                 alias: None,
//                 name: Positioned {
//                     pos: Pos(42:7),
//                     node: Name("category")
//                 },
//                 arguments: [],
//                 directives: [],
//                 selection_set: Positioned {
//                     pos: Pos(0:0),
//                     node: SelectionSet {
//                     items: []
//                     }
//                 }
//                 }
//             })
//             },
//             Positioned {
//             pos: Pos(43:7),
//             node: Field(Positioned {
//                 pos: Pos(43:7),
//                 node: Field {
//                 alias: None,
//                 name: Positioned {
//                     pos: Pos(43:7),
//                     node: Name("name")
//                 },
//                 arguments: [],
//                 directives: [],
//                 selection_set: Positioned {
//                     pos: Pos(0:0),
//                     node: SelectionSet {
//                     items: []
//                     }
//                 }
//                 }
//             })
//             },
//             Positioned {
//             pos: Pos(44:7),
//             node: Field(Positioned {
//                 pos: Pos(44:7),
//                 node: Field {
//                 alias: None,
//                 name: Positioned {
//                     pos: Pos(44:7),
//                     node: Name("price")
//                 },
//                 arguments: [],
//                 directives: [],
//                 selection_set: Positioned {
//                     pos: Pos(0:0),
//                     node: SelectionSet {
//                     items: []
//                     }
//                 }
//                 }
//             })
//             }
//         ]
//         }
//     }
//     }
// })
// }
