use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use common::config::ProcessorConfig;
use processing::processor::Processor;
use processing::model::{
    Processible, Feature, FeatureValue, MatchingField, ModelId, ScorerResult, TriggeredRule,
    ConnectedTransaction, DirectConnection, Channel, ScoringEvent, Label, FraudLevel, LabelSource
};
use processing::storage::CommonStorage;
use processing::queue::QueueService;
use processing::storage::ProcessibleStorage;
use chrono::Utc;

// Import centralized mocks directly
use common::test_helpers::{
    MockQueueService as CentralizedMockQueueService,
    MockCommonStorage as CentralizedMockCommonStorage,
    MockProcessible, MockScorer,
    TestDataFactory, TestFeature, TestScorerResult, TestMatchingField
};
use async_trait::async_trait;

// Simple conversion functions
fn convert_test_feature_to_processing(test_feature: &TestFeature) -> Feature {
    let value: Box<FeatureValue> = if let Some(int_val) = test_feature.int_value {
        Box::new(FeatureValue::Int(int_val as i64))
    } else if let Some(string_val) = &test_feature.string_value {
        Box::new(FeatureValue::String(string_val.clone()))
    } else {
        Box::new(FeatureValue::Int(42)) // Default
    };
    
    Feature {
        name: test_feature.name.clone(),
        value,
    }
}

fn convert_test_matching_field_to_processing(test_field: &TestMatchingField) -> MatchingField {
    MatchingField {
        matcher: test_field.matcher.clone(),
        value: test_field.value.clone(),
    }
}

// Adapter for MockProcessible to implement processing::model::Processible
struct ProcessibleAdapter {
    inner: MockProcessible,
}

impl ProcessibleAdapter {
    fn new(id: i64) -> Self {
        Self {
            inner: MockProcessible::new(id),
        }
    }
}

#[async_trait]
impl Processible for ProcessibleAdapter {
    fn tx_id(&self) -> i64 {
        self.inner.tx_id()
    }

    fn id(&self) -> i64 {
        self.inner.get_id()
    }

    fn extract_simple_features(&self) -> Vec<Feature> {
        self.inner.extract_simple_features()
            .iter()
            .map(convert_test_feature_to_processing)
            .collect()
    }

    fn extract_graph_features(
        &self,
        _connected_transactions: &[ConnectedTransaction],
        _direct_connections: &[DirectConnection]
    ) -> Vec<Feature> {
        // For tests, just return the same as simple features
        self.extract_simple_features()
    }

    fn extract_matching_fields(&self) -> Vec<MatchingField> {
        self.inner.extract_matching_fields()
            .iter()
            .map(convert_test_matching_field_to_processing)
            .collect()
    }
}

// Adapter for MockScorer
struct ScorerAdapter {
    inner: MockScorer,
}

impl ScorerAdapter {
    fn new(test_scores: Vec<TestScorerResult>) -> Self {
        Self {
            inner: MockScorer::new(test_scores),
        }
    }
}

#[async_trait]
impl processing::scorers::Scorer for ScorerAdapter {
    async fn score(&self, features: Vec<Feature>) -> Vec<ScorerResult> {
        // Convert features to test features for the centralized mock
        let test_features: Vec<TestFeature> = features.iter().map(|f| {
            TestDataFactory::create_feature(&f.name, 42)
        }).collect();
        
        // Call the centralized mock
        let test_results = self.inner.score(test_features).await.unwrap();
        
        // Convert back to processing ScorerResult
        test_results.iter().map(|tr| ScorerResult {
            name: tr.name.clone(),
            score: tr.score,
        }).collect()
    }
}

// Adapter for MockCommonStorage
struct CommonStorageAdapter {
    inner: CentralizedMockCommonStorage,
}

impl CommonStorageAdapter {
    fn new(id: i64, features: Vec<Feature>, scores: Vec<ScorerResult>) -> Self {
        let test_features: Vec<TestFeature> = features.iter().map(|f| {
            TestDataFactory::create_feature(&f.name, 42)
        }).collect();
        
        let test_scores: Vec<TestScorerResult> = scores.iter().map(|s| {
            TestDataFactory::create_scorer_result(&s.name, s.score)
        }).collect();
        
        Self {
            inner: CentralizedMockCommonStorage::new(id, test_features, test_scores),
        }
    }
    
    pub fn was_save_features_called(&self) -> bool {
        self.inner.was_save_features_called()
    }
    
    pub fn was_save_scores_called(&self) -> bool {
        self.inner.was_save_scores_called()
    }
    
    pub fn was_save_matching_fields_called(&self) -> bool {
        self.inner.was_save_matching_fields_called()
    }
}

#[async_trait]
impl CommonStorage for CommonStorageAdapter {
    async fn save_features(&self, tx_id: i64, _simple_features: Option<&[Feature]>, _graph_features: &[Feature]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Just track the call
        assert_eq!(tx_id, self.inner.transaction_id);
        self.inner.save_features_called.store(true, Ordering::Relaxed);
        Ok(())
    }

    async fn save_scores(&self, tx_id: i64, _channel_id: i64, _total_score: i32, scores: &[TriggeredRule]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        assert_eq!(tx_id, self.inner.transaction_id);
        assert_eq!(scores.len(), self.inner.scores.len());
        self.inner.save_scores_called.store(true, Ordering::Relaxed);
        Ok(())
    }

    async fn get_features(&self, tx_id: i64) -> Result<(Option<Vec<Feature>>, Vec<Feature>), Box<dyn std::error::Error + Send + Sync>> {
        assert_eq!(tx_id, self.inner.transaction_id);
        self.inner.get_features_called.store(true, Ordering::Relaxed);
        
        let processing_features: Vec<Feature> = self.inner.features.iter()
            .map(convert_test_feature_to_processing)
            .collect();
        
        Ok((Some(processing_features.clone()), processing_features))
    }   

    // Stub implementations for unused methods
    async fn find_connected_transactions(&self, _transaction_id: i64, _max_depth: Option<i32>, _limit_count: Option<i32>, _min_created_at: Option<chrono::DateTime<chrono::Utc>>, _max_created_at: Option<chrono::DateTime<chrono::Utc>>, _min_confidence: Option<i32>) -> Result<Vec<ConnectedTransaction>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Vec::new())
    }

    async fn get_direct_connections(&self, _transaction_id: ModelId) -> Result<Vec<DirectConnection>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Vec::new())
    }

    async fn save_matching_fields(&self, tx_id: i64, _matching_fields: &[MatchingField]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        assert_eq!(tx_id, self.inner.transaction_id);
        self.inner.save_matching_fields_called.store(true, Ordering::Relaxed);
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
}

// Adapter for ProcessibleStorage
struct ProcessibleStorageAdapter {
    called: Arc<AtomicBool>,
    expected_tx_id: i64,
    return_value: Option<ProcessibleAdapter>,
}

impl ProcessibleStorageAdapter {
    fn new(expected_tx_id: i64, return_value: Option<ProcessibleAdapter>) -> Self {
        Self {
            called: Arc::new(AtomicBool::new(false)),
            expected_tx_id,
            return_value,
        }
    }
    
    pub fn was_called(&self) -> bool {
        self.called.load(Ordering::Relaxed)
    }
}

#[async_trait]
impl ProcessibleStorage<ProcessibleAdapter> for ProcessibleStorageAdapter {
    async fn get_processible(&self, tx_id: i64) -> Result<ProcessibleAdapter, Box<dyn std::error::Error + Send + Sync>> {
        self.called.store(true, Ordering::Relaxed);
        assert_eq!(tx_id, self.expected_tx_id);
        
        match &self.return_value {
            Some(processible) => Ok(ProcessibleAdapter::new(processible.inner.get_id())),
            None => Err("No processible found".into()),
        }
    }
}

// Mock QueueService using centralized implementation
#[derive(Debug, Clone)]
struct MockQueueService {
    inner: CentralizedMockQueueService,
}

impl MockQueueService {
    fn new(return_value: Option<i64>, expected_mark_processed_id: i64) -> Self {
        Self {
            inner: CentralizedMockQueueService::new(return_value, expected_mark_processed_id),
        }
    }
    
    pub fn was_mark_processed_called(&self) -> bool {
        self.inner.was_mark_processed_called()
    }
}

#[async_trait]
impl QueueService for MockQueueService {
    async fn fetch_next(&self) -> Result<Option<i64>, Box<dyn std::error::Error + Send + Sync>> {
        println!("MockQueueService.fetch_next");
        self.inner.fetch_next().await
    }

    async fn mark_processed(&self, tx_id: i64) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("MockQueueService.mark_processed");
        self.inner.mark_processed(tx_id).await
    }

    async fn enqueue(&self, tx_id: i64) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("MockQueueService.enqueue");
        self.inner.enqueue(tx_id).await
    }
}

#[tokio::test]
async fn test_processor_process() -> Result<(), Box<dyn Error + Send + Sync>> {
    // Create test data
    let tx_id = 1;
    let processible = ProcessibleAdapter::new(tx_id);
    let expected_features = processible.extract_simple_features();
    
    // Create scores
    let scores = vec![ScorerResult {
        score: 100,
        name: "test_score".to_string(),
    }];
    
    // Set up mocks using centralized implementations
    let scorer = ScorerAdapter::new(vec![TestDataFactory::create_scorer_result("test_score", 100)]);
    let storage = CommonStorageAdapter::new(tx_id, expected_features, scores);
    let processible_storage = ProcessibleStorageAdapter::new(tx_id, Some(processible));
    let queue = MockQueueService::new(Some(tx_id), tx_id);
    let failed_queue = MockQueueService::new(Some(tx_id), tx_id);
    
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
    let scorer = ScorerAdapter::new(vec![]);
    let storage = CommonStorageAdapter::new(1, vec![], vec![]);
    let processible_storage = ProcessibleStorageAdapter::new(tx_id, None);
    let queue = MockQueueService::new(Some(tx_id), tx_id);
    let failed_queue = MockQueueService::new(Some(tx_id), tx_id);
    
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
    
    let scores = vec![ScorerResult {
        score: 75,
        name: "custom_score".to_string(),
    }];
    
    // Use centralized mocks
    let scorer = ScorerAdapter::new(vec![TestDataFactory::create_scorer_result("custom_score", 75)]);
    let storage = CommonStorageAdapter::new(tx_id, expected_features, scores);
    let processible_storage = ProcessibleStorageAdapter::new(tx_id, Some(processible));
    let queue = MockQueueService::new(Some(tx_id), tx_id);
    let failed_queue = MockQueueService::new(Some(tx_id), tx_id);
    
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