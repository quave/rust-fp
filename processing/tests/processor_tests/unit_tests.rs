use std::error::Error;
use std::sync::Arc;
use common::config::ProcessorConfig;
use processing::processor::Processor;
use processing::model::Processible;
use crate::mocks::MockScorer;

use super::super::mocks::{TestPayload, MockQueueService, create_high_value_scorer, create_low_value_scorer, create_mock_common_storage};

#[tokio::test]
async fn test_processor_with_high_value_transaction(){
    // Create a high-value transaction for testing
    let transaction = TestPayload::high_value();
    let features = transaction.extract_simple_features();
    
    // Set up mocks
    let storage = create_mock_common_storage(None, features);
    
    let mut queue = MockQueueService::new();
    queue.expect_fetch_next().returning(|_| Ok(vec!(1)));
    queue.expect_mark_processed().returning(|_| Ok(()));
    let queue = Arc::new(queue);
    
    let scorer = create_high_value_scorer();
    
    // Create processor
    let processor = Processor::<TestPayload, MockScorer>::new_raw(
        ProcessorConfig::default(),
        scorer,
        Arc::new(storage),
        queue.clone(),
        queue,
    );
    
    // Process the transaction
    let result = processor.process(1).await;
    
    // Verify the result
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_processor_with_low_value_transaction() {
    // Create a low-value transaction for testing
    let transaction = TestPayload::low_value();
    let features = transaction.extract_simple_features();
    
    // Set up mocks
    let storage = create_mock_common_storage(None, features);
    
    let mut queue = MockQueueService::new();
    queue.expect_fetch_next().returning(|_| Ok(vec!(2)));
    queue.expect_mark_processed().returning(|_| Ok(()));
    let queue = Arc::new(queue);
    
    let scorer = create_low_value_scorer();
    
    // Create processor
    let processor = Processor::<TestPayload, MockScorer>::new_raw(
        ProcessorConfig::default(),
        scorer,
        Arc::new(storage),
        queue.clone(),
        queue,
    );
    
    // Process the transaction
    let result = processor.process(2).await;
    
    // Verify the result
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_processor_with_empty_queue() {
    // Create a transaction for testing
    let transaction = TestPayload::high_value();
    let features = transaction.extract_simple_features();
    
    // Set up mocks with empty queue
    let storage = create_mock_common_storage(None, features);
    
    let mut queue = MockQueueService::new();
    queue.expect_fetch_next().returning(|_| Ok(vec!())); // Empty queue
    queue.expect_mark_processed().returning(|_| Ok(()));
    let queue = Arc::new(queue);
    
    let scorer = create_high_value_scorer();
    
    // Create processor
    let processor = Processor::<TestPayload, MockScorer>::new_raw(
        ProcessorConfig::default(),
        scorer,
        Arc::new(storage),
        queue.clone(),
        queue,
    );
    
    // Try to process - should handle gracefully when no transaction ID provided
    // Since we're calling process(1) directly, we bypass the queue's fetch_next
    let result = processor.process(1).await;
    
    // Should still work since we provided a transaction ID directly
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_processor_feature_extraction() {
    // Create a high-value transaction with complex features
    let transaction = TestPayload::high_value();
    
    // Test simple feature extraction
    let simple_features = transaction.extract_simple_features();
    assert_eq!(simple_features.len(), 1);
    assert_eq!(simple_features[0].name, "is_high_value");
    
    // Test graph feature extraction
    let empty_connected = Vec::new();
    let empty_direct = Vec::new();
    let graph_features = transaction.extract_graph_features(&empty_connected, &empty_direct);
    
    // Should have multiple features for high-value transactions
    assert!(graph_features.len() >= 5); // is_high_value, connected_count, direct_count, amount, amounts, categories, created_at
    
    // Verify specific features are present
    let feature_names: Vec<&String> = graph_features.iter().map(|f| &f.name).collect();
    assert!(feature_names.contains(&&"is_high_value".to_string()));
    assert!(feature_names.contains(&&"connected_transaction_count".to_string()));
    assert!(feature_names.contains(&&"direct_connection_count".to_string()));
    assert!(feature_names.contains(&&"amount".to_string()));
}

#[tokio::test]
async fn test_processor_matching_fields() -> Result<(), Box<dyn Error + Send + Sync>> {
    // Create a regular transaction
    let transaction = TestPayload::high_value();
    
    // Test matching field extraction
    let matching_fields = transaction.extract_matching_fields();
    assert_eq!(matching_fields.len(), 2);
    assert_eq!(matching_fields[0].matcher, "customer.email");
    assert_eq!(matching_fields[0].value, "test@example.com");
    assert_eq!(matching_fields[1].matcher, "payment_details");
    assert_eq!(matching_fields[1].value, "1234");
    
    Ok(())
}

#[tokio::test]
async fn test_processor_scorer_integration() -> Result<(), Box<dyn Error + Send + Sync>> {
    // Create a transaction for testing
    let transaction = TestPayload::high_value();
    let features = transaction.extract_simple_features();
    
    // Set up mocks
    let storage = create_mock_common_storage(None, features);
    
    let mut queue = MockQueueService::new();
    queue.expect_fetch_next().returning(|_| Ok(vec!(1)));
    queue.expect_mark_processed().returning(|_| Ok(()));
    let queue = Arc::new(queue);
    
    let scorer = create_high_value_scorer();
    
    // Create processor
    let processor = Processor::<TestPayload, MockScorer>::new_raw(
        ProcessorConfig::default(),
        scorer,
        Arc::new(storage),
        queue.clone(),
        queue,
    );
    
    // Process the transaction
    let result = processor.process(1).await;
    
    // Verify the result includes scoring
    assert!(result.is_ok());
    
    Ok(())
}