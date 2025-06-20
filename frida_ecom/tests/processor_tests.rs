use chrono::{Utc, Duration};
use frida_core::{
    model::FeatureValue,
    processor::Processor,
    queue::{ProdQueue, QueueService},
    scorers::RuleBasedScorer,
    storage::{CommonStorage, ProdCommonStorage},
    test_utils::{setup_test_environment, create_test_pool, create_test_common_storage, get_test_database_url, MockQueue},
};
use frida_ecom::{
    ecom_db_model::Order,
    ecom_order_storage::EcomOrderStorage,
    rule_based_scorer::get_rule_based_scorer,
};
use sqlx::PgPool;
use std::{error::Error, sync::Arc};
use log::info;
use tokio::sync::OnceCell;

static SETUP: OnceCell<()> = OnceCell::const_new();

async fn ensure_setup() {
    SETUP.get_or_init(|| async {
        setup_test_environment().await.expect("Failed to setup test environment");
    }).await;
}

async fn get_test_resources() -> Result<(PgPool, ProdCommonStorage, EcomOrderStorage, ProdQueue), Box<dyn Error + Send + Sync>> {
    ensure_setup().await;
    let pool = create_test_pool().await?;
    let common_storage = create_test_common_storage().await?;
    let model_storage = EcomOrderStorage::new(&get_test_database_url()).await?;
    let queue = ProdQueue::new(&get_test_database_url()).await?;
    
    Ok((pool, common_storage, model_storage, queue))
}

#[tokio::test]
async fn test_order_processing_flow() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (pool, common_storage, model_storage, queue) = get_test_resources().await?;
    
    // Create test data directly in the database
    let mut tx = pool.begin().await?;
    let now = Utc::now();
    
    let transaction_id = common_storage.insert_transaction().await?;
    
    // Insert order
    let order_id = model_storage.insert_order(
        &mut tx,
        transaction_id,
        "TEST-001",
        "standard",
        "address",
        now,
    ).await?;
    
    // Insert order items
    model_storage.insert_order_item(
        &mut tx,
        order_id,
        "Item 1",
        "electronics",
        600.0,
        now,
    ).await?;
    
    model_storage.insert_order_item(
        &mut tx,
        order_id,
        "Item 2",
        "clothing",
        500.0,
        now,
    ).await?;
    
    // Insert customer data
    model_storage.insert_customer(
        &mut tx,
        order_id,
        "Test Customer",
        "test@example.com",
        now,
    ).await?;
    
    // Insert billing data
    model_storage.insert_billing(
        &mut tx,
        order_id,
        "credit_card",
        "****1234",
        "123 Test St",
        now,
    ).await?;
    
    tx.commit().await?;
    
    // Enqueue the order
    queue.enqueue(transaction_id).await?;
    
    // Create processor
    let processor = Processor::<Order, RuleBasedScorer>::new(
        get_rule_based_scorer(),
        Arc::new(common_storage.clone()),
        Arc::new(model_storage),
        Arc::new(queue),
    );
    
    // Process the order
    let result = processor.process().await?;
    assert!(result.is_some());
    
    // Verify features were created
    let features = common_storage.get_features(transaction_id).await?;
    info!("Found {} features:", features.len());
    for feature in &features {
        info!("  - {}: {:?}", feature.name, feature.value);
    }
    
    assert_eq!(features.len(), 5);
    
    // Verify created_at feature
    let created_at = features.iter().find(|f| f.name == "created_at").unwrap();
    match created_at.value.as_ref() {
        FeatureValue::DateTime(dt) => {
            let diff = now.signed_duration_since(*dt);
            assert!(diff < Duration::seconds(1), "Timestamp should be within the last second");
        },
        _ => panic!("Expected DateTime value for created_at"),
    }
    
    // Verify is_high_value feature
    let is_high_value = features.iter().find(|f| f.name == "is_high_value").unwrap();
    match is_high_value.value.as_ref() {
        FeatureValue::Bool(value) => assert!(*value),
        _ => panic!("Expected Bool value for is_high_value"),
    }

    // Verify amount feature
    let amount = features.iter().find(|f| f.name == "amount").unwrap();
    match amount.value.as_ref() {
        FeatureValue::Double(value) => assert_eq!(*value, 1100.0),
        _ => panic!("Expected Double value for amount"),
    }

    // Verify amounts feature
    let amounts = features.iter().find(|f| f.name == "amounts").unwrap();
    match amounts.value.as_ref() {
        FeatureValue::DoubleList(values) => {
            assert_eq!(values.len(), 2);
            assert_eq!(values[0], 600.0);
            assert_eq!(values[1], 500.0);
        },
        _ => panic!("Expected DoubleList value for amounts"),
    }

    // Verify categories feature
    let categories = features.iter().find(|f| f.name == "categories").unwrap();
    match categories.value.as_ref() {
        FeatureValue::StringList(values) => {
            assert_eq!(values.len(), 2);
            assert_eq!(values[0], "electronics");
            assert_eq!(values[1], "clothing");
        },
        _ => panic!("Expected StringList value for categories"),
    }
    
    Ok(())
}

#[tokio::test]
async fn test_order_processing_with_empty_queue() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (_pool, common_storage, model_storage, _queue) = get_test_resources().await?;
    
    // Create processor with MockQueue
    let mock_queue = Arc::new(MockQueue::new());
    let processor = Processor::<Order, RuleBasedScorer>::new(
        get_rule_based_scorer(),
        Arc::new(common_storage.clone()),
        Arc::new(model_storage),
        mock_queue,
    );
    
    // Process with empty queue
    let result = processor.process().await?;
    assert!(result.is_none());
    
    Ok(())
} 