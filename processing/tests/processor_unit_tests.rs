use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use chrono::Utc;
use async_trait::async_trait;

use processing::{
    model::{
        Feature, FeatureValue, ModelId, Processible, MatchingField, ScorerResult, ConnectedTransaction, DirectConnection, 
        TriggeredRule, Channel, ScoringEvent, Label, FraudLevel, LabelSource
    },
    processor::Processor,
    scorers::Scorer,
    queue::QueueService,
    storage::{ProcessibleStorage, CommonStorage},
};

// Define a test transaction struct to use in tests
#[derive(Debug, Clone)]
struct TestTransaction {
    id: ModelId,
    transaction_id: ModelId,
    is_high_value: bool,
    created_at: chrono::DateTime<Utc>,
}

impl TestTransaction {
    fn new(id: ModelId, transaction_id: ModelId, is_high_value: bool) -> Self {
        Self {
            id,
            transaction_id,
            is_high_value,
            created_at: Utc::now(),
        }
    }

    fn high_value() -> Self {
        Self::new(1, 1, true)
    }

    fn low_value() -> Self {
        Self::new(2, 2, false)
    }
}

#[async_trait]
impl Processible for TestTransaction {
    fn extract_features(
        &self,
        connected_transactions: &[ConnectedTransaction],
        direct_connections: &[DirectConnection]
    ) -> Vec<Feature> {
        let mut features = Vec::new();
        
        features.push(Feature {
            name: "is_high_value".to_string(),
            value: Box::new(FeatureValue::Bool(self.is_high_value)),
        });
        
        // Add features for connection counts
        features.push(Feature {
            name: "connected_transaction_count".to_string(),
            value: Box::new(FeatureValue::Int(connected_transactions.len() as i64)),
        });
        
        features.push(Feature {
            name: "direct_connection_count".to_string(),
            value: Box::new(FeatureValue::Int(direct_connections.len() as i64)),
        });
        
        if self.is_high_value {
            features.push(Feature {
                name: "amount".to_string(),
                value: Box::new(FeatureValue::Double(1500.0)),
            });
            
            features.push(Feature {
                name: "amounts".to_string(),
                value: Box::new(FeatureValue::DoubleList(vec![500.0, 1000.0])),
            });
            
            features.push(Feature {
                name: "categories".to_string(),
                value: Box::new(FeatureValue::StringList(vec!["electronics".to_string(), "accessories".to_string()])),
            });
            
            features.push(Feature {
                name: "created_at".to_string(),
                value: Box::new(FeatureValue::DateTime(self.created_at)),
            });
        }
        
        features
    }

    fn tx_id(&self) -> ModelId {
        self.transaction_id
    }

    fn id(&self) -> ModelId {
        self.id
    }

    fn extract_matching_fields(&self) -> Vec<MatchingField> {
        vec![
            MatchingField {
                matcher: "customer.email".to_string(),
                value: "test@example.com".to_string(),
            },
            MatchingField {
                matcher: "payment_details".to_string(),
                value: "1234".to_string(),
            }
        ]
    }
}

// Define a mock common storage implementation
#[derive(Debug, Default)]
struct MockCommonStorage {
    features: Vec<Feature>,
}

#[async_trait]
impl CommonStorage for MockCommonStorage {
    async fn save_features(
        &self,
        _transaction_id: i64,
        _features: &[Feature],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(())
    }
    
    async fn get_features(
        &self,
        _transaction_id: i64,
    ) -> Result<Vec<Feature>, Box<dyn Error + Send + Sync>> {
        Ok(self.features.clone())
    }
    
    async fn save_scores(
        &self,
        _transaction_id: i64,
        _channel_id: i64,
        _total_score: i32,
        _triggered_rules: &[TriggeredRule],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(())
    }
    
    async fn find_connected_transactions(
        &self,
        _transaction_id: i64,
        _max_depth: Option<i32>,
        _limit_count: Option<i32>,
        _min_created_at: Option<chrono::DateTime<chrono::Utc>>,
        _max_created_at: Option<chrono::DateTime<chrono::Utc>>,
        _min_confidence: Option<i32>,
    ) -> Result<Vec<ConnectedTransaction>, Box<dyn Error + Send + Sync>> {
        Ok(vec![])
    }
    
    async fn get_direct_connections(
        &self,
        _transaction_id: ModelId
    ) -> Result<Vec<DirectConnection>, Box<dyn Error + Send + Sync>> {
        Ok(vec![])
    }
    
    async fn save_matching_fields(
        &self,
        _transaction_id: i64,
        _matching_fields: &[MatchingField],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(())
    }
    
    async fn get_channels(
        &self,
        _model_id: ModelId,
    ) -> Result<Vec<Channel>, Box<dyn Error + Send + Sync>> {
        Ok(vec![])
    }
    
    async fn get_scoring_events(
        &self,
        _transaction_id: ModelId,
    ) -> Result<Vec<ScoringEvent>, Box<dyn Error + Send + Sync>> {
        Ok(vec![])
    }
    
    async fn get_triggered_rules(
        &self,
        _scoring_event_id: ModelId,
    ) -> Result<Vec<TriggeredRule>, Box<dyn Error + Send + Sync>> {
        Ok(vec![])
    }
    
    async fn save_label(
        &self,
        _label: &Label,
    ) -> Result<ModelId, Box<dyn Error + Send + Sync>> {
        Ok(1) // Return a dummy label ID
    }
    
    async fn get_label(
        &self,
        _label_id: ModelId,
    ) -> Result<Label, Box<dyn Error + Send + Sync>> {
        Ok(Label {
            id: 1,
            fraud_level: FraudLevel::NoFraud,
            fraud_category: "Test".to_string(),
            label_source: LabelSource::Manual,
            labeled_by: "test".to_string(),
            created_at: Utc::now(),
        })
    }
    
    async fn update_transaction_label(
        &self,
        _transaction_id: ModelId,
        _label_id: ModelId,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(())
    }
}

// Create a mock common storage that tracks method calls and returns predefined values
#[derive(Debug)]
struct ConnectionTrackingStorage {
    fetch_connected_called: Arc<AtomicBool>,
    fetch_direct_called: Arc<AtomicBool>,
    save_features_called: Arc<AtomicBool>,
    connected_transactions: Vec<ConnectedTransaction>,
    direct_connections: Vec<DirectConnection>,
}

impl ConnectionTrackingStorage {
    fn new(connected_transactions: Vec<ConnectedTransaction>, direct_connections: Vec<DirectConnection>) -> Self {
        Self {
            fetch_connected_called: Arc::new(AtomicBool::new(false)),
            fetch_direct_called: Arc::new(AtomicBool::new(false)),
            save_features_called: Arc::new(AtomicBool::new(false)),
            connected_transactions,
            direct_connections,
        }
    }
}

#[async_trait]
impl CommonStorage for ConnectionTrackingStorage {
    async fn save_features(
        &self,
        _transaction_id: i64,
        features: &[Feature],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.save_features_called.store(true, Ordering::Relaxed);
        
        // Verify that the features include connection counts
        let has_connected_count = features.iter().any(|f| f.name == "connected_transaction_count");
        let has_direct_count = features.iter().any(|f| f.name == "direct_connection_count");
        
        assert!(has_connected_count, "Features should include connected_transaction_count");
        assert!(has_direct_count, "Features should include direct_connection_count");
        
        Ok(())
    }
    
    async fn get_features(
        &self,
        _transaction_id: i64,
    ) -> Result<Vec<Feature>, Box<dyn Error + Send + Sync>> {
        Ok(vec![])
    }
    
    async fn save_scores(
        &self,
        _transaction_id: i64,
        _channel_id: i64,
        _total_score: i32,
        _triggered_rules: &[TriggeredRule],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(())
    }
    
    async fn find_connected_transactions(
        &self,
        _transaction_id: i64,
        _max_depth: Option<i32>,
        _limit_count: Option<i32>,
        _min_created_at: Option<chrono::DateTime<chrono::Utc>>,
        _max_created_at: Option<chrono::DateTime<chrono::Utc>>,
        _min_confidence: Option<i32>,
    ) -> Result<Vec<ConnectedTransaction>, Box<dyn Error + Send + Sync>> {
        self.fetch_connected_called.store(true, Ordering::Relaxed);
        Ok(self.connected_transactions.clone())
    }
    
    async fn get_direct_connections(
        &self,
        _transaction_id: ModelId
    ) -> Result<Vec<DirectConnection>, Box<dyn Error + Send + Sync>> {
        self.fetch_direct_called.store(true, Ordering::Relaxed);
        Ok(self.direct_connections.clone())
    }
    
    async fn save_matching_fields(
        &self,
        _transaction_id: i64,
        _matching_fields: &[MatchingField],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(())
    }
    
    async fn get_channels(
        &self,
        _model_id: ModelId,
    ) -> Result<Vec<Channel>, Box<dyn Error + Send + Sync>> {
        Ok(vec![])
    }
    
    async fn get_scoring_events(
        &self,
        _transaction_id: ModelId,
    ) -> Result<Vec<ScoringEvent>, Box<dyn Error + Send + Sync>> {
        Ok(vec![])
    }
    
    async fn get_triggered_rules(
        &self,
        _scoring_event_id: ModelId,
    ) -> Result<Vec<TriggeredRule>, Box<dyn Error + Send + Sync>> {
        Ok(vec![])
    }
    
    async fn save_label(
        &self,
        _label: &Label,
    ) -> Result<ModelId, Box<dyn Error + Send + Sync>> {
        Ok(1) // Return a dummy label ID
    }
    
    async fn get_label(
        &self,
        _label_id: ModelId,
    ) -> Result<Label, Box<dyn Error + Send + Sync>> {
        Ok(Label {
            id: 1,
            fraud_level: FraudLevel::NoFraud,
            fraud_category: "Test".to_string(),
            label_source: LabelSource::Manual,
            labeled_by: "test".to_string(),
            created_at: Utc::now(),
        })
    }
    
    async fn update_transaction_label(
        &self,
        _transaction_id: ModelId,
        _label_id: ModelId,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(())
    }
}

// Mock ProcessibleStorage implementation
#[derive(Debug, Default)]
struct MockProcessibleStorage {
    transaction: Option<TestTransaction>,
}

impl MockProcessibleStorage {
    fn new(transaction: TestTransaction) -> Self {
        Self { transaction: Some(transaction) }
    }
}

#[async_trait]
impl ProcessibleStorage<TestTransaction> for MockProcessibleStorage {
    async fn get_processible(
        &self,
        _transaction_id: ModelId,
    ) -> Result<TestTransaction, Box<dyn Error + Send + Sync>> {
        match &self.transaction {
            Some(tx) => Ok(tx.clone()),
            None => Err("Transaction not found".into()),
        }
    }
}

// Mock queue implementation
#[derive(Debug, Default)]
struct MockQueue {
    next_id: Option<ModelId>,
}

impl MockQueue {
    fn new(next_id: Option<ModelId>) -> Self {
        Self { 
            next_id,
        }
    }
}

#[async_trait]
impl QueueService for MockQueue {
    async fn fetch_next(&self) -> Result<Option<ModelId>, Box<dyn Error + Send + Sync>> {
        Ok(self.next_id)
    }
    
    async fn mark_processed(&self, _id: ModelId) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(())
    }
    
    async fn enqueue(&self, _id: ModelId) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(())
    }
}

// Mock scorer implementation
#[derive(Debug, Default)]
struct MockScorer {
    results: Vec<ScorerResult>,
}

impl MockScorer {
    fn new(results: Vec<ScorerResult>) -> Self {
        Self { results }
    }
    
    fn high_value_scorer() -> Self {
        Self::new(vec![
            ScorerResult {
                name: "High value order".to_string(),
                score: 70,
            },
        ])
    }
    
    fn low_value_scorer() -> Self {
        Self::new(vec![])
    }
}

#[async_trait]
impl Scorer for MockScorer {
    async fn score(&self, _features: Vec<Feature>) -> Vec<ScorerResult> {
        self.results.clone()
    }
}

#[tokio::test]
async fn test_processor_with_high_value_transaction() -> Result<(), Box<dyn Error + Send + Sync>> {
    // Setup
    let transaction = TestTransaction::high_value();
    let transaction_id = transaction.tx_id();
    
    let common_storage = Arc::new(MockCommonStorage::default());
    let processible_storage = Arc::new(MockProcessibleStorage::new(transaction.clone()));
    let queue = Arc::new(MockQueue::new(Some(transaction_id)));
    let scorer = MockScorer::high_value_scorer();
    
    // Create processor
    let processor = Processor::new(
        scorer,
        common_storage,
        processible_storage,
        queue,
    );
    
    // Process the transaction
    let result = processor.process().await?;
    
    // Verify result
    assert!(result.is_some());
    let processible = result.unwrap();
    assert_eq!(processible.tx_id(), transaction_id);
    
    Ok(())
}

#[tokio::test]
async fn test_processor_with_low_value_transaction() -> Result<(), Box<dyn Error + Send + Sync>> {
    // Setup
    let transaction = TestTransaction::low_value();
    let transaction_id = transaction.tx_id();
    
    let common_storage = Arc::new(MockCommonStorage::default());
    let processible_storage = Arc::new(MockProcessibleStorage::new(transaction.clone()));
    let queue = Arc::new(MockQueue::new(Some(transaction_id)));
    let scorer = MockScorer::low_value_scorer();
    
    // Create processor
    let processor = Processor::new(
        scorer,
        common_storage,
        processible_storage,
        queue,
    );
    
    // Process the transaction
    let result = processor.process().await?;
    
    // Verify result
    assert!(result.is_some());
    let processible = result.unwrap();
    assert_eq!(processible.tx_id(), transaction_id);
    
    Ok(())
}

#[tokio::test]
async fn test_processor_with_empty_queue() -> Result<(), Box<dyn Error + Send + Sync>> {
    // Setup with empty queue
    let common_storage = Arc::new(MockCommonStorage::default());
    let processible_storage = Arc::new(MockProcessibleStorage::default());
    let queue = Arc::new(MockQueue::new(None)); // No transaction in queue
    let scorer = MockScorer::high_value_scorer();
    
    // Create processor
    let processor = Processor::new(
        scorer,
        common_storage,
        processible_storage,
        queue,
    );
    
    // Process - should return None since queue is empty
    let result = processor.process().await?;
    
    // Verify result
    assert!(result.is_none());
    
    Ok(())
}

// Add a test for the connection handling
#[tokio::test]
async fn test_processor_with_connections() -> Result<(), Box<dyn Error + Send + Sync>> {
    // Setup test data
    let transaction = TestTransaction::high_value();
    let transaction_id = transaction.tx_id();
    
    // Create some sample connections
    let connected_transactions = vec![
        ConnectedTransaction {
            transaction_id: 100,
            path_matchers: vec!["email".to_string()],
            path_values: vec!["test@example.com".to_string()],
            depth: 1,
            confidence: 90,
            importance: 80,
            created_at: Utc::now(),
        },
        ConnectedTransaction {
            transaction_id: 101,
            path_matchers: vec!["phone".to_string()],
            path_values: vec!["1234567890".to_string()],
            depth: 2,
            confidence: 85,
            importance: 75,
            created_at: Utc::now(),
        }
    ];
    
    let direct_connections = vec![
        DirectConnection {
            transaction_id: 100,
            matcher: "email".to_string(),
            confidence: 90,
            importance: 80,
            created_at: Utc::now(),
        },
        DirectConnection {
            transaction_id: 200,
            matcher: "device_id".to_string(),
            confidence: 95,
            importance: 85,
            created_at: Utc::now(),
        }
    ];
    
    // Create mocks
    let common_storage = Arc::new(ConnectionTrackingStorage::new(
        connected_transactions,
        direct_connections
    ));
    let processible_storage = Arc::new(MockProcessibleStorage::new(transaction.clone()));
    let queue = Arc::new(MockQueue::new(Some(transaction_id)));
    let scorer = MockScorer::high_value_scorer();
    
    // Create processor
    let processor = Processor::new(
        scorer,
        common_storage.clone(),
        processible_storage,
        queue,
    );
    
    // Process the transaction
    let result = processor.process().await?;
    
    // Verify result
    assert!(result.is_some());
    let processible = result.unwrap();
    assert_eq!(processible.tx_id(), transaction_id);
    
    // Verify the storage methods were called
    assert!(common_storage.fetch_connected_called.load(Ordering::Relaxed), 
            "find_connected_transactions should have been called");
    assert!(common_storage.fetch_direct_called.load(Ordering::Relaxed), 
            "get_direct_connections should have been called");
    assert!(common_storage.save_features_called.load(Ordering::Relaxed), 
            "save_features should have been called with connection-aware features");
    
    Ok(())
}

// Create a processible that verifies connection content
#[derive(Debug, Clone)]
struct ConnectionVerifyingTransaction {
    id: ModelId,
    transaction_id: ModelId,
    expected_connected_ids: Vec<ModelId>,
    expected_direct_ids: Vec<ModelId>,
    verification_passed: Arc<AtomicBool>,
}

impl ConnectionVerifyingTransaction {
    fn new(
        id: ModelId, 
        transaction_id: ModelId,
        expected_connected_ids: Vec<ModelId>,
        expected_direct_ids: Vec<ModelId>
    ) -> Self {
        Self {
            id,
            transaction_id,
            expected_connected_ids,
            expected_direct_ids,
            verification_passed: Arc::new(AtomicBool::new(false)),
        }
    }
}

#[async_trait]
impl Processible for ConnectionVerifyingTransaction {
    fn extract_features(
        &self,
        connected_transactions: &[ConnectedTransaction],
        direct_connections: &[DirectConnection]
    ) -> Vec<Feature> {
        // Verify that we received the expected connected transaction IDs
        let connected_ids: Vec<ModelId> = connected_transactions.iter()
            .map(|ct| ct.transaction_id)
            .collect();
        
        // Verify that we received the expected direct connection IDs
        let direct_ids: Vec<ModelId> = direct_connections.iter()
            .map(|dc| dc.transaction_id)
            .collect();
        
        // Check if all expected connected IDs are present
        let all_connected_present = self.expected_connected_ids.iter()
            .all(|id| connected_ids.contains(id));
            
        // Check if all expected direct IDs are present
        let all_direct_present = self.expected_direct_ids.iter()
            .all(|id| direct_ids.contains(id));
        
        // Set verification passed flag if everything matches
        if all_connected_present && all_direct_present {
            self.verification_passed.store(true, Ordering::Relaxed);
        }
        
        // Return base features plus connection counts
        vec![
            Feature {
                name: "test_feature".to_string(),
                value: Box::new(FeatureValue::Int(42)),
            },
            Feature {
                name: "connected_transaction_count".to_string(),
                value: Box::new(FeatureValue::Int(connected_transactions.len() as i64)),
            },
            Feature {
                name: "direct_connection_count".to_string(),
                value: Box::new(FeatureValue::Int(direct_connections.len() as i64)),
            }
        ]
    }

    fn tx_id(&self) -> ModelId {
        self.transaction_id
    }

    fn id(&self) -> ModelId {
        self.id
    }

    fn extract_matching_fields(&self) -> Vec<MatchingField> {
        vec![
            MatchingField {
                matcher: "test_matcher".to_string(),
                value: "test_value".to_string(),
            }
        ]
    }
}

// Mock storage for the verification transaction
#[derive(Debug)]
struct MockConnectionStorage {
    connected_transactions: Vec<ConnectedTransaction>,
    direct_connections: Vec<DirectConnection>,
}

impl MockConnectionStorage {
    fn new(connected_transactions: Vec<ConnectedTransaction>, direct_connections: Vec<DirectConnection>) -> Self {
        Self {
            connected_transactions,
            direct_connections,
        }
    }
}

#[async_trait]
impl CommonStorage for MockConnectionStorage {
    async fn save_features(
        &self,
        _transaction_id: i64,
        _features: &[Feature],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(())
    }
    
    async fn get_features(
        &self,
        _transaction_id: i64,
    ) -> Result<Vec<Feature>, Box<dyn Error + Send + Sync>> {
        Ok(vec![])
    }
    
    async fn save_scores(
        &self,
        _transaction_id: i64,
        _channel_id: i64,
        _total_score: i32,
        _triggered_rules: &[TriggeredRule],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(())
    }
    
    async fn find_connected_transactions(
        &self,
        _transaction_id: i64,
        _max_depth: Option<i32>,
        _limit_count: Option<i32>,
        _min_created_at: Option<chrono::DateTime<chrono::Utc>>,
        _max_created_at: Option<chrono::DateTime<chrono::Utc>>,
        _min_confidence: Option<i32>,
    ) -> Result<Vec<ConnectedTransaction>, Box<dyn Error + Send + Sync>> {
        Ok(self.connected_transactions.clone())
    }
    
    async fn get_direct_connections(
        &self,
        _transaction_id: ModelId
    ) -> Result<Vec<DirectConnection>, Box<dyn Error + Send + Sync>> {
        Ok(self.direct_connections.clone())
    }
    
    async fn save_matching_fields(
        &self,
        _transaction_id: i64,
        _matching_fields: &[MatchingField],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(())
    }
    
    async fn get_channels(
        &self,
        _model_id: ModelId,
    ) -> Result<Vec<Channel>, Box<dyn Error + Send + Sync>> {
        Ok(vec![])
    }
    
    async fn get_scoring_events(
        &self,
        _transaction_id: ModelId,
    ) -> Result<Vec<ScoringEvent>, Box<dyn Error + Send + Sync>> {
        Ok(vec![])
    }
    
    async fn get_triggered_rules(
        &self,
        _scoring_event_id: ModelId,
    ) -> Result<Vec<TriggeredRule>, Box<dyn Error + Send + Sync>> {
        Ok(vec![])
    }
    
    async fn save_label(
        &self,
        _label: &Label,
    ) -> Result<ModelId, Box<dyn Error + Send + Sync>> {
        Ok(1) // Return a dummy label ID
    }
    
    async fn get_label(
        &self,
        _label_id: ModelId,
    ) -> Result<Label, Box<dyn Error + Send + Sync>> {
        Ok(Label {
            id: 1,
            fraud_level: FraudLevel::NoFraud,
            fraud_category: "Test".to_string(),
            label_source: LabelSource::Manual,
            labeled_by: "test".to_string(),
            created_at: Utc::now(),
        })
    }
    
    async fn update_transaction_label(
        &self,
        _transaction_id: ModelId,
        _label_id: ModelId,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(())
    }
}

#[derive(Debug, Default)]
struct MockProcessibleConnectionStorage<P: Processible> {
    transaction: Option<P>,
}

impl<P: Processible + Clone> MockProcessibleConnectionStorage<P> {
    fn new(transaction: P) -> Self {
        Self { transaction: Some(transaction) }
    }
}

#[async_trait]
impl<P: Processible + Clone + Send + Sync + 'static> ProcessibleStorage<P> for MockProcessibleConnectionStorage<P> {
    async fn get_processible(
        &self,
        _transaction_id: ModelId,
    ) -> Result<P, Box<dyn Error + Send + Sync>> {
        match &self.transaction {
            Some(tx) => Ok(tx.clone()),
            None => Err("Transaction not found".into()),
        }
    }
}

#[tokio::test]
async fn test_processor_connection_verification() -> Result<(), Box<dyn Error + Send + Sync>> {
    // Create test transactions with specific IDs
    let connected_transactions = vec![
        ConnectedTransaction {
            transaction_id: 100,
            path_matchers: vec!["email".to_string()],
            path_values: vec!["test@example.com".to_string()],
            depth: 1,
            confidence: 90,
            importance: 80,
            created_at: Utc::now(),
        },
        ConnectedTransaction {
            transaction_id: 101,
            path_matchers: vec!["phone".to_string()],
            path_values: vec!["1234567890".to_string()],
            depth: 2,
            confidence: 85,
            importance: 75,
            created_at: Utc::now(),
        }
    ];
    
    let direct_connections = vec![
        DirectConnection {
            transaction_id: 100,
            matcher: "email".to_string(),
            confidence: 90,
            importance: 80,
            created_at: Utc::now(),
        },
        DirectConnection {
            transaction_id: 200,
            matcher: "device_id".to_string(),
            confidence: 95,
            importance: 85,
            created_at: Utc::now(),
        }
    ];
    
    // Create a transaction that will verify it receives these specific connection IDs
    let transaction = ConnectionVerifyingTransaction::new(
        1, 1,
        vec![100, 101], // expected connected transaction IDs
        vec![100, 200]  // expected direct connection IDs
    );
    let verification_passed = transaction.verification_passed.clone();
    
    // Create mocks
    let common_storage = Arc::new(MockConnectionStorage::new(
        connected_transactions,
        direct_connections
    ));
    let processible_storage = Arc::new(MockProcessibleConnectionStorage::new(transaction));
    let queue = Arc::new(MockQueue::new(Some(1)));
    let scorer = MockScorer::high_value_scorer();
    
    // Create processor
    let processor = Processor::new(
        scorer,
        common_storage,
        processible_storage,
        queue,
    );
    
    // Process the transaction
    let result = processor.process().await?;
    
    // Verify result
    assert!(result.is_some());
    
    // Verify the transaction actually received and verified the expected connections
    assert!(verification_passed.load(Ordering::Relaxed), 
            "Transaction should have received and verified the expected connections");
    
    Ok(())
} 