use actix_web::{web, App, HttpServer};
use clap::Parser;
use frida_core::{
    importer::Importer,
    in_memory_queue::InMemoryQueue,
    model::{Importable, Processible},
    queue_service::QueueService,
    storage::Storage,
};
use frida_ecom::{config::ImporterConfig, import_transaction};
use std::{error::Error, fmt::Debug, str::FromStr};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to config file
    #[arg(short, long, default_value = "frida_ecom/config/importer.toml")]
    config: String,
}

async fn run_importer<P, I, ST, Q>(
    config: ImporterConfig,
) -> Result<(), Box<dyn Error + Send + Sync>>
where
    P: Processible + 'static,
    I: Importable + 'static,
    ST: Storage<I, P> + 'static,
    Q: QueueService<P> + 'static,
    P::Id: Debug + FromStr + Send + Sync,
{
    let storage = ST::new(&config.database_url)
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    storage
        .initialize_schema()
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    let queue = Q::new();

    let service = web::Data::new(Importer::<P, I, ST, Q>::new(storage, queue));

    log::info!("Starting importer service at {}", config.server_address);
    HttpServer::new(move || {
        App::new()
            .app_data(service.clone())
            .route("/import", web::post().to(import_transaction::<P, I, ST, Q>))
            .route("/health", web::get().to(frida_ecom::health_check))
    })
    .bind(&config.server_address)?
    .run()
    .await?;

    Ok(())
}

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args = Args::parse();
    let config = ImporterConfig::from_file(&args.config)?;

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(&config.log_level))
        .init();

    // Run with concrete types
    use frida_ecom::{
        ecom_db_model::Order, ecom_import_model::ImportOrder,
        sqlite_order_storage::SqliteOrderStorage,
    };
    run_importer::<Order, ImportOrder, SqliteOrderStorage, InMemoryQueue<Order>>(config).await
}
