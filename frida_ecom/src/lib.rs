use actix_web::{get, web, HttpResponse};
use frida_core::model::{Importable, Processible};

pub mod ecom_db_model;
pub mod ecom_import_model;
pub mod rule_based_scorer;
pub mod sqlite_order_storage;

pub async fn import_transaction<I, P>(
    importer: web::Data<frida_core::importer::Importer<I, P>>,
    transaction: web::Json<I>,
) -> HttpResponse
where
    P: Processible,
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
