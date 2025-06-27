use std::error::Error;
use std::sync::Arc;
use common::config::ProcessorConfig;
use processing::processor::Processor;
use processing::model::{
    Processible, Feature, FeatureValue, MatchingField, ModelId, ScorerResult, TriggeredRule,
    ConnectedTransaction, DirectConnection, Channel, ScoringEvent, Label, FraudLevel, LabelSource, LabelingResult
};
use processing::storage::CommonStorage;
use processing::queue::QueueService;
use processing::storage::ProcessibleStorage;
use chrono::Utc;


use async_trait::async_trait;
use mockall::mock;



// Adapter for MockProcessible to implement processing::model::Processible
struct ProcessibleAdapter {
    id: i64,
}

impl ProcessibleAdapter {
    fn new(id: i64) -> Self {
        Self { id }
    }
}

#[async_trait]
impl Processible for ProcessibleAdapter {
    fn tx_id(&self) -> i64 {
        self.id
    }

    fn id(&self) -> i64 {
        self.id
    }

    fn extract_simple_features(&self) -> Vec<Feature> {
        vec![Feature {
            name: "test_feature".to_string(),
            value: Box::new(FeatureValue::Int(42)),
        }]
    }

    fn extract_graph_features(
        &self,
        _connected_transactions: &[ConnectedTransaction],
        _direct_connections: &[DirectConnection]
    ) -> Vec<Feature> {
        vec![Feature {
            name: "graph_feature".to_string(),
            value: Box::new(FeatureValue::String("test".to_string())),
        }]
    }

    fn extract_matching_fields(&self) -> Vec<MatchingField> {
        vec![MatchingField {
            matcher: "test_field".to_string(),
            value: "test_value".to_string(),
        }]
    }
}

// Direct Scorer implementation using mockall - OPTIMAL FIRST approach
mock! {
    ScorerService {}

    #[async_trait]
    impl processing::scorers::Scorer for ScorerService {
        async fn score(&self, features: Vec<Feature>) -> Vec<ScorerResult>;
    }
}

// Mock CommonStorage - for complex traits like this, we'll keep a simple custom mock
// since mockall has issues with complex lifetime parameters in traits
#[derive(Debug, Clone)]
struct MockCommonStorage {
    tx_id: i64,
    features: Vec<Feature>,
}

impl MockCommonStorage {
    fn new(tx_id: i64, features: Vec<Feature>) -> Self {
        Self { tx_id, features }
    }
}

#[async_trait]
impl CommonStorage for MockCommonStorage {
    async fn save_features(&self, tx_id: i64, _simple_features: Option<&[Feature]>, _graph_features: &[Feature]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        assert_eq!(tx_id, self.tx_id);
        Ok(())
    }

    async fn save_scores(&self, tx_id: i64, _channel_id: i64, _total_score: i32, _scores: &[TriggeredRule]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        assert_eq!(tx_id, self.tx_id);
        Ok(())
    }

    async fn get_features(&self, tx_id: i64) -> Result<(Option<Vec<Feature>>, Vec<Feature>), Box<dyn std::error::Error + Send + Sync>> {
        assert_eq!(tx_id, self.tx_id);
        Ok((Some(self.features.clone()), self.features.clone()))
    }

    async fn find_connected_transactions(&self, _transaction_id: i64, _max_depth: Option<i32>, _limit_count: Option<i32>, _min_created_at: Option<chrono::DateTime<chrono::Utc>>, _max_created_at: Option<chrono::DateTime<chrono::Utc>>, _min_confidence: Option<i32>) -> Result<Vec<ConnectedTransaction>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Vec::new())
    }

    async fn get_direct_connections(&self, _transaction_id: ModelId) -> Result<Vec<DirectConnection>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Vec::new())
    }

    async fn save_matching_fields(&self, tx_id: i64, _matching_fields: &[MatchingField]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        assert_eq!(tx_id, self.tx_id);
        Ok(())
    }

    async fn get_channels(&self, _model_id: ModelId) -> Result<Vec<Channel>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Vec::new())
    }

    async fn get_scoring_events(&self, _transaction_id: ModelId) -> Result<Vec<ScoringEvent>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Vec::new())
    }

    async fn get_triggered_rules(&self, _scoring_event_id: ModelId) -> Result<Vec<TriggeredRule>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Vec::new())
    }

    async fn save_label(&self, _label: &Label) -> Result<ModelId, Box<dyn Error + Send + Sync>> {
        Ok(1)
    }

    async fn get_label(&self, _label_id: ModelId) -> Result<Label, Box<dyn Error + Send + Sync>> {
        Ok(Label {
            id: 1,
            fraud_level: FraudLevel::NoFraud,
            fraud_category: "Test".to_string(),
            label_source: LabelSource::Manual,
            labeled_by: "test".to_string(),
            created_at: Utc::now(),
        })
    }

    async fn update_transaction_label(&self, _transaction_id: ModelId, _label_id: ModelId) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(())
    }

    async fn label_transactions(&self, _transaction_ids: &[ModelId], _fraud_level: FraudLevel, _fraud_category: String, _labeled_by: String) -> Result<LabelingResult, Box<dyn Error + Send + Sync>> {
        Ok(LabelingResult {
            label_id: 1,
            success_count: 1,
            failed_transaction_ids: Vec::new(),
        })
    }
}

// Mock ProcessibleStorage using mockall - much cleaner than custom adapter!
mock! {
    ProcessibleStorage {}

    #[async_trait]
    impl ProcessibleStorage<ProcessibleAdapter> for ProcessibleStorage {
        async fn get_processible(&self, transaction_id: i64) -> Result<ProcessibleAdapter, Box<dyn std::error::Error + Send + Sync>>;
    }
}

mock! {
    QueueService {}

    #[async_trait]
    impl QueueService for QueueService {
        async fn fetch_next(&self) -> Result<Option<i64>, Box<dyn std::error::Error + Send + Sync>>;
        async fn mark_processed(&self, tx_id: i64) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
        async fn enqueue(&self, tx_id: i64) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    }
}

#[tokio::test]
async fn test_processor_process() -> Result<(), Box<dyn Error + Send + Sync>> {
    // Create test data
    let tx_id = 1;
    
    // Set up mocks using mockall - OPTIMAL FIRST approach
    let mut scorer = MockScorerService::new();
    scorer.expect_score()
        .returning(|_| vec![ScorerResult { name: "test_score".to_string(), score: 100 }]);
    
    // Create features for the mock storage
    let features = vec![
        Feature {
            name: "test_feature".to_string(),
            value: Box::new(FeatureValue::Int(42)),
        }
    ];
    
    let storage = MockCommonStorage::new(tx_id, features);
    
    // Create mockall-based processible storage
    let mut processible_storage = MockProcessibleStorage::new();
    processible_storage.expect_get_processible()
        .with(mockall::predicate::eq(tx_id))
        .returning(move |_| Ok(ProcessibleAdapter::new(tx_id)));
    
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
    let processor = Processor::new(
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
    let processed = result.unwrap();
    assert_eq!(processed.tx_id(), tx_id);
    
    Ok(())
}

#[tokio::test]
async fn test_processor_process_with_nonexistent_transaction() -> Result<(), Box<dyn Error + Send + Sync>> {
    let tx_id = 999;
    
    // Set up mocks - processible storage returns None
    let mut scorer = MockScorerService::new();
    scorer.expect_score()
        .returning(|_| vec![]);
    let storage = MockCommonStorage::new(1, vec![]);
    
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
    
    let processor = Processor::new(
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
    let processible = ProcessibleAdapter::new(tx_id);
    let expected_features = processible.extract_simple_features();
    
    // Use simple custom mock
    let mut scorer = MockScorerService::new();
    scorer.expect_score()
        .returning(|_| vec![ScorerResult { name: "custom_score".to_string(), score: 75 }]);
    let storage = MockCommonStorage::new(tx_id, expected_features);
    
    // Create mockall-based processible storage
    let mut processible_storage = MockProcessibleStorage::new();
    processible_storage.expect_get_processible()
        .with(mockall::predicate::eq(tx_id))
        .returning(move |_| Ok(ProcessibleAdapter::new(tx_id)));
    
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
    
    let processor = Processor::new(
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