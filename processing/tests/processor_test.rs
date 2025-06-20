use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::collections::HashMap;
use async_trait::async_trait;

use processing::{
    model::{Feature, FeatureValue, ModelId, Processible, MatchingField, ScorerResult, ConnectedTransaction, DirectConnection},
    processor::Processor,
    queue::QueueService,
    storage::{CommonStorage, ProcessibleStorage, MatcherConfig},
    scorers::Scorer,
};

// Mock Processible implementation
#[derive(Debug, Clone)]
struct MockProcessible {
    id: i64,
    features: Vec<Feature>,
    matching_fields: Vec<MatchingField>
}

impl MockProcessible {
    fn new(id: i64) -> Self {
        println!("MockProcessible.new");
        Self {
            id,
            features: vec![
                Feature {
                    name: "test_feature".to_string(),
                    value: Box::new(FeatureValue::Int(42)),
                },
            ],
            matching_fields: vec![
                MatchingField {
                    matcher: "test_matcher".to_string(),
                    value: "test_value".to_string(),
                },
            ],
        }
    }
}

#[async_trait]
impl Processible for MockProcessible {
    fn tx_id(&self) -> i64 {
        println!("MockProcessible.tx_id");
        self.id
    }

    fn id(&self) -> i64 {
        println!("MockProcessible.id");
        self.id
    }

    fn extract_features(
        &self,
        _connected_transactions: &[ConnectedTransaction],
        _direct_connections: &[DirectConnection]
    ) -> Vec<Feature> {
        println!("MockProcessible.extract_features");
        self.features.clone()
    }

    fn extract_matching_fields(&self) -> Vec<MatchingField> {
        println!("MockProcessible.extract_matching_fields");
        self.matching_fields.clone()
    }
}

// Mock Scorer
#[derive(Debug, Clone)]
struct MockScorer {
    called: Arc<AtomicBool>,
    expected_features: Vec<Feature>,
}

impl MockScorer {
    fn new(expected_features: Vec<Feature>) -> Self {
        println!("MockScorer.new");
        Self {
            called: Arc::new(AtomicBool::new(false)),
            expected_features,
        }
    }
}

#[async_trait]
impl Scorer for MockScorer {
    async fn score(&self, features: Vec<Feature>) -> Vec<ScorerResult> {
        println!("MockScorer.score");
        self.called.store(true, Ordering::Relaxed);
        assert_eq!(features, self.expected_features);
        vec![ScorerResult {
            score: 100,
            name: "test_score".to_string(),
        }]
    }
}

// Mock CommonStorage
#[derive(Debug, Clone)]
struct MockCommonStorage {
    id: i64,
    features: Vec<Feature>,
    scores: Vec<ScorerResult>,
    save_features_called: Arc<AtomicBool>,
    save_scores_called: Arc<AtomicBool>,
    save_matching_fields_called: Arc<AtomicBool>,
    matcher_configs: HashMap<String, MatcherConfig>,
}

impl MockCommonStorage {
    fn new(id: i64, features: Vec<Feature>, scores: Vec<ScorerResult>) -> Self {
        println!("MockCommonStorage.new");
        Self {
            id,
            features,
            scores,
            save_features_called: Arc::new(AtomicBool::new(false)),
            save_scores_called: Arc::new(AtomicBool::new(false)),
            save_matching_fields_called: Arc::new(AtomicBool::new(false)),
            matcher_configs: HashMap::new(),
        }
    }
    
    fn with_matcher_configs(id: i64, features: Vec<Feature>, scores: Vec<ScorerResult>, matcher_configs: HashMap<String, MatcherConfig>) -> Self {
        println!("MockCommonStorage.with_matcher_configs");
        Self {
            id,
            features,
            scores,
            save_features_called: Arc::new(AtomicBool::new(false)),
            save_scores_called: Arc::new(AtomicBool::new(false)),
            save_matching_fields_called: Arc::new(AtomicBool::new(false)),
            matcher_configs,
        }
    }
}

#[async_trait]
impl CommonStorage for MockCommonStorage {
    async fn save_features(&self, tx_id: i64, features: &[Feature]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("MockCommonStorage.save_features");
        self.save_features_called.store(true, Ordering::Relaxed);
        assert_eq!(tx_id, self.id);
        assert_eq!(features, self.features);
        Ok(())
    }

    async fn save_scores(&self, tx_id: i64, scores: &[ScorerResult]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("MockCommonStorage.save_scores");
        self.save_scores_called.store(true, Ordering::Relaxed);
        assert_eq!(tx_id, self.id);
        assert_eq!(scores, self.scores);
        Ok(())
    }

    async fn get_features(&self, tx_id: i64) -> Result<Vec<Feature>, Box<dyn std::error::Error + Send + Sync>> {
        println!("MockCommonStorage.get_features");
        assert_eq!(tx_id, self.id);
        Ok(self.features.clone())
    }   

    async fn find_connected_transactions(
        &self,
        _transaction_id: i64,
        _max_depth: Option<i32>,
        _limit_count: Option<i32>,
        _min_created_at: Option<chrono::DateTime<chrono::Utc>>,
        _max_created_at: Option<chrono::DateTime<chrono::Utc>>,
        _min_confidence: Option<i32>
    ) -> Result<Vec<ConnectedTransaction>, Box<dyn std::error::Error + Send + Sync>> {
        // Just return an empty list for the mock implementation
        Ok(Vec::new())
    }
    
    async fn get_direct_connections(
        &self,
        _transaction_id: ModelId
    ) -> Result<Vec<DirectConnection>, Box<dyn std::error::Error + Send + Sync>> {
        // Just return an empty list for the mock implementation
        Ok(Vec::new())
    }
    
    async fn save_matching_fields(
        &self,
        tx_id: i64,
        _matching_fields: &[MatchingField],
        _matcher_configs: &HashMap<String, MatcherConfig>
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("MockCommonStorage.save_matching_fields");
        self.save_matching_fields_called.store(true, Ordering::Relaxed);
        assert_eq!(tx_id, self.id);
        Ok(())
    }

    fn get_matcher_configs(&self) -> HashMap<String, MatcherConfig> {
        self.matcher_configs.clone()
    }
}

// Mock ProcessibleStorage
#[derive(Debug, Clone)]
struct MockProcessibleStorage {
    called: Arc<AtomicBool>,
    expected_tx_id: i64,
    return_value: Option<MockProcessible>,
}

impl MockProcessibleStorage {
    fn new(expected_tx_id: i64, return_value: Option<MockProcessible>) -> Self {
        println!("MockProcessibleStorage.new");
        Self {
            called: Arc::new(AtomicBool::new(false)),
            expected_tx_id,
            return_value,
        }
    }
}

#[async_trait]
impl ProcessibleStorage<MockProcessible> for MockProcessibleStorage {
    async fn get_processible(&self, tx_id: i64) -> Result<MockProcessible, Box<dyn std::error::Error + Send + Sync>> {
        println!("MockProcessibleStorage.get_processible");
        self.called.store(true, Ordering::Relaxed);
        assert_eq!(tx_id, self.expected_tx_id);
        self.return_value.clone().ok_or_else(|| "No processible found".into())
    }
}

// Mock QueueService
#[derive(Debug, Clone)]
struct MockQueueService {
    fetch_next_called: Arc<AtomicBool>,
    mark_processed_called: Arc<AtomicBool>,
    return_value: Option<i64>,
    expected_mark_processed_id: i64,
}

impl MockQueueService {
    fn new(return_value: Option<i64>, expected_mark_processed_id: i64) -> Self {
        Self {
            fetch_next_called: Arc::new(AtomicBool::new(false)),
            mark_processed_called: Arc::new(AtomicBool::new(false)),
            return_value,
            expected_mark_processed_id,
        }
    }
}

#[async_trait]
impl QueueService for MockQueueService {
    async fn fetch_next(&self) -> Result<Option<i64>, Box<dyn std::error::Error + Send + Sync>> {
        println!("MockQueueService.fetch_next");
        self.fetch_next_called.store(true, Ordering::Relaxed);
        Ok(self.return_value)
    }

    async fn mark_processed(&self, tx_id: i64) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("MockQueueService.mark_processed");
        self.mark_processed_called.store(true, Ordering::Relaxed);
        assert_eq!(tx_id, self.expected_mark_processed_id);
        Ok(())
    }

    async fn enqueue(&self, _tx_id: i64) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("MockQueueService.enqueue");
        Ok(())
    }
}

#[tokio::test]
async fn test_processor_process() {
    // Setup test data
    let tx_id = 123;
    let processible = MockProcessible::new(tx_id);
    let empty_connected: Vec<ConnectedTransaction> = Vec::new();
    let empty_direct: Vec<DirectConnection> = Vec::new();
    let features = processible.extract_features(&empty_connected, &empty_direct);
    let expected_scores = vec![ScorerResult {
        score: 100,
        name: "test_score".to_string(),
    }];

    // Create mocks
    let scorer = MockScorer::new(features.clone());
    let common_storage = MockCommonStorage::new(tx_id, features.clone(), expected_scores.clone());
    let processible_storage = MockProcessibleStorage::new(tx_id, Some(processible.clone()));
    let queue = MockQueueService::new(Some(tx_id), tx_id);

    // Create processor
    let processor = Processor::new(
        scorer.clone(),
        Arc::new(common_storage.clone()),
        Arc::new(processible_storage.clone()),
        Arc::new(queue.clone()),
    );

    // Run process
    let result = processor.process().await.unwrap();

    // Verify results
    assert!(result.is_some());
    assert_eq!(result.unwrap().tx_id(), tx_id);

    // Verify mock calls
    assert!(scorer.called.load(Ordering::Relaxed));
    assert!(common_storage.save_features_called.load(Ordering::Relaxed));
    assert!(common_storage.save_scores_called.load(Ordering::Relaxed));
    assert!(common_storage.save_matching_fields_called.load(Ordering::Relaxed));
    assert!(processible_storage.called.load(Ordering::Relaxed));
    assert!(queue.fetch_next_called.load(Ordering::Relaxed));
    assert!(queue.mark_processed_called.load(Ordering::Relaxed));
}

#[tokio::test]
async fn test_processor_process_empty_queue() {
    // Create mocks
    let scorer = MockScorer::new(vec![]);
    let common_storage = MockCommonStorage::new(0, vec![], vec![]);
    let processible_storage = MockProcessibleStorage::new(0, None);
    let queue = MockQueueService::new(None, 0);

    // Create processor
    let processor = Processor::new(
        scorer.clone(),
        Arc::new(common_storage.clone()),
        Arc::new(processible_storage.clone()),
        Arc::new(queue.clone()),
    );

    // Run process
    let result = processor.process().await.unwrap();

    // Verify results
    assert!(result.is_none());

    // Verify mock calls
    assert!(!scorer.called.load(Ordering::Relaxed));
    assert!(!common_storage.save_features_called.load(Ordering::Relaxed));
    assert!(!common_storage.save_scores_called.load(Ordering::Relaxed));
    assert!(!common_storage.save_matching_fields_called.load(Ordering::Relaxed));
    assert!(!processible_storage.called.load(Ordering::Relaxed));
    assert!(queue.fetch_next_called.load(Ordering::Relaxed));
    assert!(!queue.mark_processed_called.load(Ordering::Relaxed));
}

#[tokio::test]
async fn test_processor_with_custom_matching_config() {
    // Setup test data
    let tx_id = 123;
    let processible = MockProcessible::new(tx_id);
    let empty_connected: Vec<ConnectedTransaction> = Vec::new();
    let empty_direct: Vec<DirectConnection> = Vec::new();
    let features = processible.extract_features(&empty_connected, &empty_direct);
    let expected_scores = vec![ScorerResult {
        score: 100,
        name: "test_score".to_string(),
    }];

    // Create custom matcher configs
    let mut matcher_configs = HashMap::new();
    matcher_configs.insert("test_matcher".to_string(), (99, 99));
    matcher_configs.insert("custom_matcher".to_string(), (75, 50));

    // Create mocks with custom matcher configs
    let scorer = MockScorer::new(features.clone());
    
    // Pass the custom matcher configs to the storage
    let common_storage = MockCommonStorage::with_matcher_configs(
        tx_id, 
        features.clone(), 
        expected_scores.clone(), 
        matcher_configs
    );
    
    let processible_storage = MockProcessibleStorage::new(tx_id, Some(processible.clone()));
    let queue = MockQueueService::new(Some(tx_id), tx_id);

    // Create processor
    let processor = Processor::new(
        scorer.clone(),
        Arc::new(common_storage.clone()),
        Arc::new(processible_storage.clone()),
        Arc::new(queue.clone()),
    );

    // Run process
    let result = processor.process().await.unwrap();

    // Verify results
    assert!(result.is_some());
    assert_eq!(result.unwrap().tx_id(), tx_id);

    // Verify all expected mock calls
    assert!(scorer.called.load(Ordering::Relaxed));
    assert!(common_storage.save_features_called.load(Ordering::Relaxed));
    assert!(common_storage.save_scores_called.load(Ordering::Relaxed));
    assert!(common_storage.save_matching_fields_called.load(Ordering::Relaxed));
    assert!(processible_storage.called.load(Ordering::Relaxed));
    assert!(queue.fetch_next_called.load(Ordering::Relaxed));
    assert!(queue.mark_processed_called.load(Ordering::Relaxed));
} 