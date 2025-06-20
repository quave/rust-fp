use actix_web::{web, App, HttpResponse, HttpServer};

// Export the handler function
// Update the HTTP handler
pub async fn handle_transaction<S: Scorer, ST: Storage<Order>, Q: QueueService<Order>>(
    importer: web::Data<Importer<Order, S, ST, Q>>,
    transaction: web::Json<Order>,
) -> HttpResponse {
    match importer.import(transaction.into_inner()).await {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}
