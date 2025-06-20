use clap::Parser;
use std::{error::Error, fmt::Debug, sync::Arc};

use crate::{
    config::{Config, ImporterConfig},
    importer::Importer,
    model::{Importable, ImportableSerde},
    queue::QueueService,
    storage::ImportableStorage,
};
use actix_web::{get, middleware::Logger, web, App, HttpResponse, HttpServer};

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

    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or(&config.importer.log_level),
    )
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
    I: ImportableSerde + 'static,
{
    let service = web::Data::new(Importer::<I>::new(storage, queue));

    log::info!("Starting importer service at {}", config.server_address);
    HttpServer::new(move || {
        App::new()
            .app_data(service.clone())
            .wrap(Logger::default())
            .route("/import", web::post().to(import_transaction::<I>))
            .service(health_check)
    })
    .bind(&config.server_address)?
    .run()
    .await?;

    Ok(())
}

pub async fn import_transaction<I>(
    importer: web::Data<Importer<I>>,
    transaction: web::Json<I>,
) -> HttpResponse
where
    I: Importable,
{
    match importer.import(transaction.into_inner()).await {
        Ok(id) => {
            log::info!("Successfully imported transaction with ID: {:?}", id);
            HttpResponse::Ok().json(id)
        }
        Err(e) => {
            log::error!("Failed to import transaction: {}", e);
            HttpResponse::InternalServerError().body(e.to_string())
        }
    }
}

#[get("/health")]
pub async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}
