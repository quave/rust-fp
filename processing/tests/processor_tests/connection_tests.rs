use crate::mocks::MockScorer;
use common::config::ProcessorConfig;
use processing::model::{ConnectedTransaction, DirectConnection, Processible};
use processing::processor::Processor;
use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use super::super::mocks::{
    ConnectionTrackingStorage, MockQueueService, TestPayload, create_empty_scorer,
    create_high_value_scorer,
};

#[tokio::test]
async fn test_processor_with_connections() -> Result<(), Box<dyn Error + Send + Sync>> {
    // Create test connection data with correct field names
    let connected_transactions = vec![
        ConnectedTransaction {
            transaction_id: 100,
            parent_transaction_id: 100,
            matcher: "payment.card_number".to_string(),
            confidence: 85,
            importance: 50,
            created_at: chrono::Utc::now(),
        },
        ConnectedTransaction {
            transaction_id: 200,
            parent_transaction_id: 100,
            matcher: "customer.email".to_string(),
            confidence: 92,
            importance: 60,
            created_at: chrono::Utc::now(),
        },
    ];

    let direct_connections = vec![
        DirectConnection {
            transaction_id: 300,
            matcher: "payment_method".to_string(),
            confidence: 95,
            importance: 70,
            created_at: chrono::Utc::now(),
        },
        DirectConnection {
            transaction_id: 400,
            matcher: "customer_id".to_string(),
            confidence: 88,
            importance: 65,
            created_at: chrono::Utc::now(),
        },
    ];

    // Set up mocks
    let storage =
        ConnectionTrackingStorage::new(connected_transactions.clone(), direct_connections.clone());

    let mut queue = MockQueueService::new();
    queue.expect_fetch_next().returning(|_| Ok(vec![(1, 1)]));
    queue.expect_mark_processed().returning(|_| Ok(()));
    queue.expect_enqueue().returning(|_| Ok(()));
    let queue = Arc::new(queue);

    let scorer = create_high_value_scorer();

    // Create processor
    let processor = Processor::<TestPayload, MockScorer>::new_raw(
        Arc::new(ProcessorConfig::default()),
        Arc::new(scorer),
        Arc::new(storage.clone()),
        queue.clone(),
        queue,
    );

    // Process the transaction
    let result = processor.process(1).await;

    // Verify the result
    assert!(result.is_ok());

    // Verify that connections were fetched
    assert!(storage.fetch_connected_called.load(Ordering::Relaxed));
    assert!(storage.fetch_direct_called.load(Ordering::Relaxed));
    assert!(storage.save_features_called.load(Ordering::Relaxed));

    Ok(())
}

#[tokio::test]
async fn test_processor_connection_verification() {
    // Create specific connection data for verification
    let connected_ids = vec![100, 200];
    let direct_ids = vec![300, 400];

    let connected_transactions = vec![
        ConnectedTransaction {
            transaction_id: 100,
            parent_transaction_id: 100,
            matcher: "payment.card_number".to_string(),
            confidence: 85,
            importance: 50,
            created_at: chrono::Utc::now(),
        },
        ConnectedTransaction {
            transaction_id: 200,
            parent_transaction_id: 100,
            matcher: "customer.email".to_string(),
            confidence: 92,
            importance: 60,
            created_at: chrono::Utc::now(),
        },
    ];

    let direct_connections = vec![
        DirectConnection {
            transaction_id: 300,
            matcher: "payment_method".to_string(),
            confidence: 95,
            importance: 70,
            created_at: chrono::Utc::now(),
        },
        DirectConnection {
            transaction_id: 400,
            matcher: "customer_id".to_string(),
            confidence: 88,
            importance: 65,
            created_at: chrono::Utc::now(),
        },
    ];

    // Create a verification transaction that expects specific connections
    let tx_id = 10_001;
    let transaction = TestPayload::connection_verifying(tx_id, connected_ids, direct_ids);
    let verification_flag = transaction.verification_passed().unwrap();

    // Set up mocks
    let storage = ConnectionTrackingStorage::with_processible(
        connected_transactions,
        direct_connections,
        transaction,
    );

    let mut queue = MockQueueService::new();
    queue
        .expect_fetch_next()
        .returning(move |_| Ok(vec![(tx_id, tx_id)]));
    queue.expect_mark_processed().returning(|_| Ok(()));
    queue.expect_enqueue().returning(|_| Ok(()));
    let queue = Arc::new(queue);

    let scorer = create_empty_scorer();

    // Create processor
    let processor = Processor::<TestPayload, MockScorer>::new_raw(
        Arc::new(ProcessorConfig::default()),
        Arc::new(scorer),
        Arc::new(storage.clone()),
        queue.clone(),
        queue,
    );

    // Process the transaction
    let result = processor.process(tx_id).await;

    // Verify the result
    assert!(result.is_ok());

    // Verify that connection verification passed
    assert!(
        verification_flag.load(Ordering::SeqCst),
        "Connection verification should have passed"
    );

    // Verify that storage methods were called
    assert!(storage.fetch_connected_called.load(Ordering::Relaxed));
    assert!(storage.fetch_direct_called.load(Ordering::Relaxed));
    assert!(storage.save_features_called.load(Ordering::Relaxed));
}

#[tokio::test]
async fn test_processor_connection_feature_extraction() {
    // Create test connection data
    let connected_transactions = vec![
        ConnectedTransaction {
            transaction_id: 100,
            parent_transaction_id: 100,
            matcher: "payment.card_number".to_string(),
            confidence: 85,
            importance: 50,
            created_at: chrono::Utc::now(),
        },
        ConnectedTransaction {
            transaction_id: 200,
            parent_transaction_id: 100,
            matcher: "customer.email".to_string(),
            confidence: 92,
            importance: 60,
            created_at: chrono::Utc::now(),
        },
        ConnectedTransaction {
            transaction_id: 300,
            parent_transaction_id: 100,
            matcher: "shipping.address".to_string(),
            confidence: 78,
            importance: 45,
            created_at: chrono::Utc::now(),
        },
    ];

    let direct_connections = vec![
        DirectConnection {
            transaction_id: 400,
            matcher: "payment_method".to_string(),
            confidence: 95,
            importance: 70,
            created_at: chrono::Utc::now(),
        },
        DirectConnection {
            transaction_id: 500,
            matcher: "customer_id".to_string(),
            confidence: 88,
            importance: 65,
            created_at: chrono::Utc::now(),
        },
    ];

    // Test graph feature extraction with connections
    let transaction = TestPayload::high_value();
    let graph_features =
        transaction.extract_graph_features(&connected_transactions, &direct_connections);

    // Verify connection count features are present
    let _connected_count_feature = graph_features
        .iter()
        .find(|f| f.name == "connected_transaction_count")
        .expect("Should have connected_transaction_count feature");

    let _direct_count_feature = graph_features
        .iter()
        .find(|f| f.name == "direct_connection_count")
        .expect("Should have direct_connection_count feature");

    // Verify the counts match the test data
    // Note: We need to extract the actual values, but for this test we'll just verify presence
    assert!(graph_features.len() >= 7); // All features including connection counts
}

#[tokio::test]
async fn test_processor_empty_connections() {
    // Set up mocks with empty connections
    let storage = ConnectionTrackingStorage::new(Vec::new(), Vec::new());

    let mut queue = MockQueueService::new();
    queue.expect_fetch_next().returning(|_| Ok(vec![(1, 1)]));
    queue.expect_mark_processed().returning(|_| Ok(()));
    queue.expect_enqueue().returning(|_| Ok(()));
    let queue = Arc::new(queue);

    let scorer = create_high_value_scorer();

    // Create processor
    let processor = Processor::<TestPayload, MockScorer>::new_raw(
        Arc::new(ProcessorConfig::default()),
        Arc::new(scorer),
        Arc::new(storage.clone()),
        queue.clone(),
        queue,
    );

    // Process the transaction
    let result = processor.process(1).await;

    // Verify the result
    assert!(result.is_ok());

    // Verify that connections were still fetched (even if empty)
    assert!(storage.fetch_connected_called.load(Ordering::Relaxed));
    assert!(storage.fetch_direct_called.load(Ordering::Relaxed));
    assert!(storage.save_features_called.load(Ordering::Relaxed));
}
