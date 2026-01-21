use super::super::mocks::{MockQueueService, MockScorer, TestPayload, create_mock_common_storage};
use common::config::ProcessorConfig;
use processing::model::*;
use processing::processor::Processor;
use std::sync::Arc;

#[tokio::test]
async fn test_processor_process() {
    // Create test data
    let tx_id = 1;

    // Set up mocks using mockall - OPTIMAL FIRST approach
    let mut scorer = MockScorer::new();
    scorer
        .expect_score_and_save_result()
        .returning(|_, _, _| Ok(()));
    scorer.expect_channel_id().return_const(1);
    scorer
        .expect_scorer_type()
        .returning(|| ScoringModelType::RuleBased);

    // Create features for the mock storage
    let features = vec![Feature {
        name: "test_feature".to_string(),
        value: Box::new(FeatureValue::Int(42)),
    }];

    let processible = TestPayload::new(tx_id, true);
    let storage = create_mock_common_storage(Some(tx_id), Some(processible), features);

    // Create mockall-based queue services
    let mut queue = MockQueueService::new();
    queue
        .expect_fetch_next()
        .returning(move |_| Ok(vec![(tx_id, tx_id)]));
    queue
        .expect_mark_processed()
        .with(mockall::predicate::eq(tx_id))
        .returning(|_| Ok(()));
    queue.expect_enqueue().returning(|_| Ok(()));

    let mut failed_queue = MockQueueService::new();
    failed_queue
        .expect_fetch_next()
        .returning(move |_| Ok(vec![(tx_id, tx_id)]));
    failed_queue
        .expect_mark_processed()
        .with(mockall::predicate::eq(tx_id))
        .returning(|_| Ok(()));
    failed_queue.expect_enqueue().returning(|_| Ok(()));

    // Create processor
    let processor = Processor::<TestPayload, MockScorer>::new_raw(
        Arc::new(ProcessorConfig::default()),
        Arc::new(scorer),
        Arc::new(storage),
        Arc::new(queue),
        Arc::new(failed_queue),
    );

    // Process
    let result = processor.process(tx_id).await;

    // Verify result
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_processor_process_with_nonexistent_transaction() {
    let tx_id = 999;

    // Set up mocks - processible storage returns None
    let mut scorer = MockScorer::new();
    scorer
        .expect_score_and_save_result()
        .returning(|_, _, _| Ok(()));
    scorer.expect_channel_id().return_const(1);
    scorer
        .expect_scorer_type()
        .returning(|| ScoringModelType::RuleBased);
    let storage = create_mock_common_storage(Some(tx_id), None, vec![]);

    let mut queue = MockQueueService::new();
    queue
        .expect_fetch_next()
        .returning(move |_| Ok(vec![(tx_id, tx_id)]));
    queue.expect_mark_processed().returning(|_| Ok(()));
    queue.expect_enqueue().returning(|_| Ok(()));

    let mut failed_queue = MockQueueService::new();
    failed_queue
        .expect_fetch_next()
        .returning(move |_| Ok(vec![(tx_id, tx_id)]));
    failed_queue.expect_mark_processed().returning(|_| Ok(()));
    failed_queue.expect_enqueue().returning(|_| Ok(()));

    let processor = Processor::<TestPayload, MockScorer>::new_raw(
        Arc::new(ProcessorConfig::default()),
        Arc::new(scorer),
        Arc::new(storage),
        Arc::new(queue),
        Arc::new(failed_queue),
    );

    // Should handle the error gracefully and return None or an Err
    let result = processor.process(tx_id).await;
    // Either Ok(None) or Err - both are acceptable for nonexistent transactions
    assert!(result.is_err());
}

#[tokio::test]
async fn test_processor_with_custom_matching_config() {
    let tx_id = 1;
    let processible = TestPayload::new(tx_id, true);
    let expected_features = processible.extract_simple_features();

    // Use simple custom mock
    let mut scorer = MockScorer::new();
    scorer
        .expect_score_and_save_result()
        .returning(|_, _, _| Ok(()));
    scorer.expect_channel_id().return_const(1);
    scorer
        .expect_scorer_type()
        .returning(|| ScoringModelType::RuleBased);
    let storage = create_mock_common_storage(Some(tx_id), Some(processible), expected_features);

    // Create mockall-based processible storage
    let mut queue = MockQueueService::new();
    queue
        .expect_fetch_next()
        .returning(move |_| Ok(vec![(tx_id, tx_id)]));
    queue
        .expect_mark_processed()
        .with(mockall::predicate::eq(tx_id))
        .returning(|_| Ok(()));
    queue.expect_enqueue().returning(|_| Ok(()));

    let mut failed_queue = MockQueueService::new();
    failed_queue
        .expect_fetch_next()
        .returning(move |_| Ok(vec![(tx_id, tx_id)]));
    failed_queue
        .expect_mark_processed()
        .with(mockall::predicate::eq(tx_id))
        .returning(|_| Ok(()));
    failed_queue.expect_enqueue().returning(|_| Ok(()));

    let processor = Processor::<TestPayload, MockScorer>::new_raw(
        Arc::new(ProcessorConfig::default()),
        Arc::new(scorer),
        Arc::new(storage),
        Arc::new(queue),
        Arc::new(failed_queue),
    );

    let result = processor.process(tx_id).await;
    assert!(result.is_ok());
}
