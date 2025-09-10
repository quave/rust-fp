use std::error::Error;
use std::sync::Arc;
use common::config::ProcessorConfig;
use processing::processor::Processor;
use processing::model::*;
use super::super::mocks::{create_mock_common_storage, MockQueueService, MockScorer, TestTransaction, MockProcessibleStorage};

#[tokio::test]
async fn test_processor_process() -> Result<(), Box<dyn Error + Send + Sync>> {
    // Create test data
    let tx_id = 1;
    
    // Set up mocks using mockall - OPTIMAL FIRST approach
    let mut scorer = MockScorer::new();
    scorer.expect_score()
        .returning(|_| vec![ScorerResult { name: "test_score".to_string(), score: 100 }]);
    
    // Create features for the mock storage
    let features = vec![
        Feature {
            name: "test_feature".to_string(),
            value: Box::new(FeatureValue::Int(42)),
        }
    ];
    
    let storage = create_mock_common_storage(Some(tx_id), features);
    
    // Create mockall-based processible storage
    let mut processible_storage = MockProcessibleStorage::new();
    processible_storage.expect_get_processible()
        .with(mockall::predicate::eq(tx_id))
        .returning(move |_| Ok(TestTransaction::new(tx_id, true)));
    processible_storage.expect_set_transaction_id()
        .with(mockall::predicate::eq(tx_id), mockall::predicate::eq(tx_id))
        .returning(|_, _| Ok(()));
    
    // Create mockall-based queue services
    let mut queue = MockQueueService::new();
    queue.expect_fetch_next()
        .returning(move || Ok(Some(tx_id)));
    queue.expect_mark_processed()
        .with(mockall::predicate::eq(tx_id))
        .returning(|_| Ok(()));
    
    let mut failed_queue = MockQueueService::new();
    failed_queue.expect_fetch_next()
        .returning(move || Ok(Some(tx_id)));
    failed_queue.expect_mark_processed()
        .with(mockall::predicate::eq(tx_id))
        .returning(|_| Ok(()));
    
    // Create processor
    let processor = Processor::new_raw(
        ProcessorConfig::default(),
        scorer,
        Arc::new(storage),
        Arc::new(processible_storage),
        Arc::new(queue),
        Arc::new(failed_queue),
    );
    
    // Process
    let result = processor.process(tx_id).await?;
    
    // Verify result
    assert!(result.is_some());
    
    Ok(())
}

#[tokio::test]
async fn test_processor_process_with_nonexistent_transaction() -> Result<(), Box<dyn Error + Send + Sync>> {
    let tx_id = 999;
    
    // Set up mocks - processible storage returns None
    let mut scorer = MockScorer::new();
    scorer.expect_score()
        .returning(|_| vec![]);
    let storage = create_mock_common_storage(Some(1), vec![]);
    
    // Create mockall-based processible storage that returns an error
    let mut processible_storage = MockProcessibleStorage::new();
    processible_storage.expect_get_processible()
        .with(mockall::predicate::eq(tx_id))
        .returning(|_| Err("No processible found".into()));
    
    let mut queue = MockQueueService::new();
    queue.expect_fetch_next()
        .returning(move || Ok(Some(tx_id)));
    queue.expect_mark_processed()
        .returning(|_| Ok(()));
    
    let mut failed_queue = MockQueueService::new();
    failed_queue.expect_fetch_next()
        .returning(move || Ok(Some(tx_id)));
    failed_queue.expect_mark_processed()
        .returning(|_| Ok(()));
    
    let processor = Processor::new_raw(
        ProcessorConfig::default(),
        scorer,
        Arc::new(storage),
        Arc::new(processible_storage),
        Arc::new(queue),
        Arc::new(failed_queue),
    );
    
    // Should handle the error gracefully and return None or an Err
    let result = processor.process(tx_id).await;
    // Either Ok(None) or Err - both are acceptable for nonexistent transactions
    match result {
        Ok(Some(_)) => panic!("Should not find a nonexistent transaction"),
        Ok(None) => {}, // Acceptable - processor handled gracefully
        Err(_) => {}, // Also acceptable - error bubbled up
    }
    
    Ok(())
}

#[tokio::test]
async fn test_processor_with_custom_matching_config() -> Result<(), Box<dyn Error + Send + Sync>> {
    let tx_id = 1;
    let processible = TestTransaction::new(tx_id, true);
    let expected_features = processible.extract_simple_features();
    
    // Use simple custom mock
    let mut scorer = MockScorer::new();
    scorer.expect_score()
        .returning(|_| vec![ScorerResult { name: "custom_score".to_string(), score: 75 }]);
    let storage = create_mock_common_storage(Some(tx_id), expected_features);
    
    // Create mockall-based processible storage
    let mut processible_storage = MockProcessibleStorage::new();
    processible_storage.expect_get_processible()
        .with(mockall::predicate::eq(tx_id))
        .returning(move |_| Ok(TestTransaction::new(tx_id, true)));
    processible_storage.expect_set_transaction_id()
        .with(mockall::predicate::eq(tx_id), mockall::predicate::eq(tx_id))
        .returning(|_, _| Ok(()));
    
    let mut queue = MockQueueService::new();
    queue.expect_fetch_next()
        .returning(move || Ok(Some(tx_id)));
    queue.expect_mark_processed()
        .with(mockall::predicate::eq(tx_id))
        .returning(|_| Ok(()));
    
    let mut failed_queue = MockQueueService::new();
    failed_queue.expect_fetch_next()
        .returning(move || Ok(Some(tx_id)));
    failed_queue.expect_mark_processed()
        .with(mockall::predicate::eq(tx_id))
        .returning(|_| Ok(()));
    
    let processor = Processor::new_raw(
        ProcessorConfig::default(),
        scorer,
        Arc::new(storage),
        Arc::new(processible_storage),
        Arc::new(queue),
        Arc::new(failed_queue),
    );
    
    let result = processor.process(tx_id).await?;
    assert!(result.is_some());
    
    Ok(())
} 