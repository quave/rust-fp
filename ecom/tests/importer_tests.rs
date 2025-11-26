use async_trait::async_trait;
use common::test_helpers::{
    create_test_connection, get_test_database_url, setup_test_environment,
    truncate_processing_tables,
};
use ecom::model::*;
use mockall::mock;
use processing::{executable_utils::import_transaction, storage::ProdCommonStorage};
use processing::{importer::Importer, queue::QueueService};
use std::error::Error;
use std::sync::Arc;
use tokio::sync::OnceCell;

static SETUP: OnceCell<()> = OnceCell::const_new();

async fn ensure_setup() {
    SETUP
        .get_or_init(|| async {
            setup_test_environment()
                .await
                .expect("Failed to setup test environment");
        })
        .await;
}

// Direct mockall implementation - much cleaner than custom adapter!
mock! {
    QueueService {}

    #[async_trait]
    impl QueueService for QueueService {
        async fn fetch_next(&self, number: i32) -> Result<Vec<i64>, Box<dyn Error + Send + Sync>>;
        async fn mark_processed(&self, tx_id: i64) -> Result<(), Box<dyn Error + Send + Sync>>;
        async fn enqueue(&self, tx_id: i64) -> Result<(), Box<dyn Error + Send + Sync>>;
    }
}

#[tokio::test]
async fn test_import_endpoint() -> Result<(), Box<dyn Error + Send + Sync>> {
    ensure_setup().await;
    println!("test_import_endpoint 1");

    // Clean up any existing test data first
    let database_url = get_test_database_url();
    let db = create_test_connection().await?;
    truncate_processing_tables(&db).await?;

    let storage = Arc::new(ProdCommonStorage::<EcomOrder>::new(&database_url).await?);

    // Create mockall-based queue service with basic expectations
    let mut queue = MockQueueService::new();
    queue.expect_fetch_next().returning(|_| Ok(vec![])); // Empty queue for this test
    queue.expect_mark_processed().returning(|_| Ok(()));
    queue.expect_enqueue().returning(|_| Ok(()));

    let queue = Arc::new(queue);

    // Create test request using standard test data patterns
    let test_order = EcomOrder {
        order_number: "TEST123".to_string(),
        delivery_type: "express".to_string(),
        delivery_details: "test details".to_string(),
        created_at: chrono::Utc::now(),
        customer: CustomerData {
            name: "Test Customer".to_string(),
            email: "test@example.com".to_string(),
        },
        billing: BillingData {
            payment_type: "credit_card".to_string(),
            payment_details: "credit_card".to_string(),
            billing_address: "123 Test St, Test City".to_string(),
        },
        items: vec![OrderItem {
            price: 10.50,
            name: "Test Item".to_string(),
            category: "Test Category".to_string(),
        }],
    };

    let importer = Importer::<EcomOrder>::new(storage, queue);
    let resp =
        import_transaction::<EcomOrder>(axum::extract::State(importer), axum::Json(test_order))
            .await;

    println!("test_import_endpoint: {:?}", resp);
    assert!(resp.status().is_success(), "Import endpoint should succeed");

    Ok(())
}
