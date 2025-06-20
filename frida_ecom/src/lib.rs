use actix_web::{web, HttpResponse};
use frida_core::{
    model::{Importable, Processible},
    queue_service::QueueService,
    storage::Storage,
};
use std::{fmt::Debug, str::FromStr};

pub mod config;
pub mod ecom_db_model;
pub mod ecom_import_model;
pub mod rule_based_scorer;
pub mod sqlite_order_storage;

pub async fn import_transaction<T, IT, ST, Q>(
    importer: web::Data<frida_core::importer::Importer<T, IT, ST, Q>>,
    transaction: web::Json<IT>,
) -> HttpResponse
where
    T: Processible + 'static,
    IT: Importable + 'static,
    ST: Storage<IT, T> + 'static,
    Q: QueueService<T> + 'static,
    T::Id: Debug + FromStr + Send + Sync,
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

pub async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}
