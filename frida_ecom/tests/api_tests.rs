use actix_web::{test, web, App};
use frida_core::storage::ImportableStorage;
use frida_core::test_utils::MockQueue;
use frida_ecom::{
    ecom_db_model::Order, ecom_import_model::ImportOrder, sqlite_order_storage::SqliteOrderStorage,
};
use frida_ecom::{health_check, import_transaction};
use serde_json::json;
use std::sync::Arc;

#[actix_web::test]
async fn test_import_endpoint() {
    // Initialize test database
    let storage = Arc::new(SqliteOrderStorage::new("sqlite::memory:").await.unwrap());
    storage.initialize_schema().await.unwrap();

    // Initialize mock queue
    let queue = Arc::new(MockQueue::new());

    // Create test app
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(frida_core::importer::Importer::<
                ImportOrder,
                Order,
            >::new(storage, queue)))
            .service(health_check)
            .route(
                "/import",
                web::post().to(import_transaction::<ImportOrder, Order>),
            ),
    )
    .await;

    // Create test request
    let req = test::TestRequest::post()
        .uri("/import")
        .insert_header(("content-type", "application/json"))
        .set_json(json!({
            "order_number": "TEST123",
            "delivery_type": "express",
            "delivery_details": "test details",
            "customer": {
                "name": "Test Customer",
                "email": "test@example.com",
            },
            "billing": {
                "payment_type": "credit_card",
                "payment_details": "credit_card",
                "billing_address": "123 Test St, Test City"
            },
            "items": [
                {
                    "price": 10.50,
                    "name": "Test Item",
                    "category": "Test Category",
                }
            ]
        }))
        .to_request();

    // Send request and check response
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body = test::read_body(resp).await;

    println!("Response status: {}", status);
    println!("Response body: {:?}", String::from_utf8(body.to_vec()));

    assert!(status.is_success());
}
