use axum::{
    routing::{post, get},
    Router,
};
use tower::util::ServiceExt;
use processing::executable_utils::{health_check, import_transaction};
use processing::test_helpers::{setup_test_environment, MockQueue, get_test_database_url};
use processing::importer::Importer;
use ecom::{
    ecom_order_storage::EcomOrderStorage,
    ecom_import_model::ImportOrder,
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

#[tokio::test]
async fn test_import_endpoint() -> Result<(), Box<dyn Error + Send + Sync>> {
    ensure_setup().await;
    
    let storage = Arc::new(EcomOrderStorage::new(&get_test_database_url()).await?);
    let queue = Arc::new(MockQueue::new());

    // Create test app
    let app = Router::new()
        .route("/import", post(import_transaction::<ImportOrder>))
        .route("/health", get(health_check))
        .with_state(Importer::<ImportOrder>::new(storage, queue));

    // Create test request
    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/import")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_string(&json!({
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
            }))?,
        ))?;

    // Send request and check response
    let resp = app.oneshot(req).await?;
    assert!(resp.status().is_success());

    Ok(())
}
