use actix_web::{test, web, App};
use frida_core::executable_utils::{health_check, import_transaction};
use frida_core::test_utils::{setup_test_environment, MockQueue, get_test_database_url};
use frida_ecom::{
    ecom_import_model::ImportOrder, ecom_order_storage::EcomOrderStorage,
};
use serde_json::json;
use std::sync::Arc;
use std::error::Error;
use tokio::sync::OnceCell;

static SETUP: OnceCell<()> = OnceCell::const_new();

async fn ensure_setup() {
    SETUP.get_or_init(|| async {
        setup_test_environment().await.expect("Failed to setup test environment");
    }).await;
}

#[actix_web::test]
async fn test_import_endpoint() -> Result<(), Box<dyn Error + Send + Sync>> {
    ensure_setup().await;
    
    let storage = Arc::new(EcomOrderStorage::new(&get_test_database_url()).await?);
    let queue = Arc::new(MockQueue::new());

    // Create test app
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(frida_core::importer::Importer::<
                ImportOrder,
            >::new(storage, queue)))
            .service(health_check)
            .route(
                "/import",
                web::post().to(import_transaction::<ImportOrder>),
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
    assert!(resp.status().is_success());

    Ok(())
}
