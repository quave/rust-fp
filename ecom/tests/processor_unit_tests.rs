use std::error::Error;
use std::sync::Arc;

use processing::{
    model::{FeatureValue, Processible},
    processor::Processor,
    queue::{ProdQueue, QueueService},
    storage::{CommonStorage, ImportableStorage, ProdCommonStorage},
};

use ecom::{
    ecom_db_model::Order,
    ecom_order_storage::EcomOrderStorage,
    ecom_import_model::{ImportOrder, ImportOrderItem, ImportCustomerData, ImportBillingData},
    rule_based_scorer::get_rule_based_scorer,
};

use processing::test_utils::{setup_test_environment, get_test_database_url};
use tokio::sync::OnceCell;

static SETUP: OnceCell<()> = OnceCell::const_new();

async fn ensure_setup() {
    SETUP.get_or_init(|| async {
        setup_test_environment().await.expect("Failed to setup test environment");
    }).await;
}

async fn get_test_storage() -> (Arc<EcomOrderStorage>, Arc<ProdCommonStorage>) {
    ensure_setup().await;
    let database_url = get_test_database_url();
    let order_storage = EcomOrderStorage::new(&database_url)
        .await
        .expect("Failed to create order storage");
    let common_storage = ProdCommonStorage::new(&database_url)
        .await
        .expect("Failed to create common storage");
    (Arc::new(order_storage), Arc::new(common_storage))
}

fn create_test_order(order_number: &str, total_amount: f32) -> ImportOrder {
    ImportOrder {
        order_number: order_number.to_string(),
        items: vec![
            ImportOrderItem {
                name: "Test Item 1".to_string(),
                category: "Test Category 1".to_string(),
                price: total_amount / 2.0,
            },
            ImportOrderItem {
                name: "Test Item 2".to_string(),
                category: "Test Category 2".to_string(),
                price: total_amount / 2.0,
            },
        ],
        customer: ImportCustomerData {
            name: "Test Customer".to_string(),
            email: "test@example.com".to_string(),
        },
        billing: ImportBillingData {
            payment_type: "credit_card".to_string(),
            payment_details: "4111111111111111".to_string(),
            billing_address: "Test Address".to_string(),
        },
        delivery_type: "standard".to_string(),
        delivery_details: "Test Delivery".to_string(),
    }
}

#[tokio::test]
async fn test_processor_with_high_value_order() -> Result<(), Box<dyn Error + Send + Sync>> {
    // Setup
    let (order_storage, common_storage) = get_test_storage().await;
    let queue = Arc::new(ProdQueue::new(&get_test_database_url()).await?);
    let scorer = get_rule_based_scorer();

    // Create a high-value order (over 1000)
    let high_value_order = create_test_order("HIGH_VALUE_ORDER", 1500.0);
    let transaction_id = order_storage.save_transaction(&high_value_order).await?;
    queue.enqueue(transaction_id).await?;

    // Create processor
    let processor = Processor::<Order, _>::new(
        scorer,
        common_storage.clone(),
        order_storage.clone(),
        queue.clone(),
    );

    // Process the order
    let result = processor.process().await?;
    assert!(result.is_some());
    let processed_order = result.unwrap();
    assert_eq!(processed_order.tx_id(), transaction_id);

    // Verify features
    let features = common_storage.get_features(transaction_id).await?;
    assert_eq!(features.len(), 5); // amount, amounts, categories, created_at, is_high_value

    // Verify amount feature
    let amount_feature = features.iter().find(|f| f.name == "amount").unwrap();
    match amount_feature.value.as_ref() {
        FeatureValue::Double(amount) => assert_eq!(*amount, 1500.0),
        _ => panic!("Expected Double value for amount"),
    }

    // Verify amounts feature
    let amounts_feature = features.iter().find(|f| f.name == "amounts").unwrap();
    match amounts_feature.value.as_ref() {
        FeatureValue::DoubleList(amounts) => {
            assert_eq!(amounts.len(), 2);
            assert_eq!(amounts[0], 750.0);
            assert_eq!(amounts[1], 750.0);
        },
        _ => panic!("Expected DoubleList value for amounts"),
    }

    // Verify categories feature
    let categories_feature = features.iter().find(|f| f.name == "categories").unwrap();
    match categories_feature.value.as_ref() {
        FeatureValue::StringList(categories) => {
            assert_eq!(categories.len(), 2);
            assert_eq!(categories[0], "Test Category 1");
            assert_eq!(categories[1], "Test Category 2");
        },
        _ => panic!("Expected StringList value for categories"),
    }

    // Verify is_high_value feature
    let is_high_value_feature = features.iter().find(|f| f.name == "is_high_value").unwrap();
    match is_high_value_feature.value.as_ref() {
        FeatureValue::Bool(is_high_value) => assert!(*is_high_value),
        _ => panic!("Expected Bool value for is_high_value"),
    }

    // Verify scores
    let scores = sqlx::query!(
        r#"
        SELECT rule_name, rule_score
        FROM triggered_rules
        WHERE transaction_id = $1
        "#,
        transaction_id
    )
    .fetch_all(&common_storage.pool)
    .await?;

    assert_eq!(scores.len(), 1);
    assert_eq!(scores[0].rule_name, "High order total");
    assert_eq!(scores[0].rule_score, 70);

    Ok(())
}

#[tokio::test]
async fn test_processor_with_low_value_order() -> Result<(), Box<dyn Error + Send + Sync>> {
    // Setup
    let (order_storage, common_storage) = get_test_storage().await;
    let queue = Arc::new(ProdQueue::new(&get_test_database_url()).await?);
    let scorer = get_rule_based_scorer();

    // Create a low-value order (under 1000)
    let low_value_order = create_test_order("LOW_VALUE_ORDER", 500.0);
    let transaction_id = order_storage.save_transaction(&low_value_order).await?;
    queue.enqueue(transaction_id).await?;

    // Create processor
    let processor = Processor::<Order, _>::new(
        scorer,
        common_storage.clone(),
        order_storage.clone(),
        queue.clone(),
    );

    // Process the order
    let result = processor.process().await?;
    assert!(result.is_some());
    let processed_order = result.unwrap();
    assert_eq!(processed_order.tx_id(), transaction_id);

    // Verify features
    let features = common_storage.get_features(transaction_id).await?;
    assert_eq!(features.len(), 5); // amount, amounts, categories, created_at, is_high_value

    // Verify amount feature
    let amount_feature = features.iter().find(|f| f.name == "amount").unwrap();
    match amount_feature.value.as_ref() {
        FeatureValue::Double(amount) => assert_eq!(*amount, 500.0),
        _ => panic!("Expected Double value for amount"),
    }

    // Verify is_high_value feature
    let is_high_value_feature = features.iter().find(|f| f.name == "is_high_value").unwrap();
    match is_high_value_feature.value.as_ref() {
        FeatureValue::Bool(is_high_value) => assert!(!*is_high_value),
        _ => panic!("Expected Bool value for is_high_value"),
    }

    // Verify no scores were generated
    let scores = sqlx::query!(
        r#"
        SELECT COUNT(*) as count
        FROM triggered_rules
        WHERE transaction_id = $1
        "#,
        transaction_id
    )
    .fetch_one(&common_storage.pool)
    .await?;

    assert_eq!(scores.count.unwrap_or(0), 0);

    Ok(())
} 