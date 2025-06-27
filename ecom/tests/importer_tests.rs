use axum::{
    routing::{post, get},
    Router,
};
use tower::util::ServiceExt;
use processing::executable_utils::{health_check, import_transaction};
use common::test_helpers::{setup_test_environment, get_test_database_url};
use processing::{importer::Importer, queue::QueueService};
use ecom::{
    ecom_order_storage::EcomOrderStorage,
    ecom_import_model::ImportOrder,
};
use serde_json::json;
use std::sync::Arc;
use std::error::Error;
use tokio::sync::OnceCell;
use async_trait::async_trait;
use mockall::mock;

static SETUP: OnceCell<()> = OnceCell::const_new();

async fn ensure_setup() {
    SETUP.get_or_init(|| async {
        setup_test_environment().await.expect("Failed to setup test environment");
    }).await;
}

// Direct mockall implementation - much cleaner than custom adapter!
mock! {
    QueueService {}

    #[async_trait]
    impl QueueService for QueueService {
        async fn fetch_next(&self) -> Result<Option<i64>, Box<dyn Error + Send + Sync>>;
        async fn mark_processed(&self, tx_id: i64) -> Result<(), Box<dyn Error + Send + Sync>>;
        async fn enqueue(&self, tx_id: i64) -> Result<(), Box<dyn Error + Send + Sync>>;
    }
}

#[tokio::test]
async fn test_import_endpoint() -> Result<(), Box<dyn Error + Send + Sync>> {
    ensure_setup().await;
    
    // Clean up any existing test data first
    let database_url = get_test_database_url();
    let pool = common::test_helpers::create_test_pool().await?;
    common::test_helpers::truncate_processing_tables(&pool).await?;
    
    let storage = Arc::new(EcomOrderStorage::new(&database_url).await?);
    
    // Create mockall-based queue service with basic expectations
    let mut queue = MockQueueService::new();
    queue.expect_fetch_next()
        .returning(|| Ok(None)); // Empty queue for this test
    queue.expect_mark_processed()
        .returning(|_| Ok(()));
    queue.expect_enqueue()
        .returning(|_| Ok(()));
    
    let queue = Arc::new(queue);

    // Create test app with centralized test utilities
    let app = Router::new()
        .route("/import", post(import_transaction::<ImportOrder>))
        .route("/health", get(health_check))
        .with_state(Importer::<ImportOrder>::new(storage, queue));

    // Create test request using standard test data patterns
    let test_order = json!({
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
    });

    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/import")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(serde_json::to_string(&test_order)?))?;

    // Send request and verify response
    let resp = app.oneshot(req).await?;
    assert!(resp.status().is_success(), "Import endpoint should succeed");

    Ok(())
}
