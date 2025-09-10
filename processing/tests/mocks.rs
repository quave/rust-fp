use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use chrono::Utc;
use async_trait::async_trait;
use seaography::Builder;
use sea_orm::DatabaseConnection;
use processing::{
    model::*,
    storage::{CommonStorage, WebStorage, ProcessibleStorage}, 
    queue::QueueService,
};

// Test transaction struct for processor tests
#[derive(Debug, Clone)]
pub struct TestTransaction {
    pub id: ModelId,
    pub is_high_value: bool,
    pub created_at: chrono::DateTime<Utc>,
    // Add connection verification fields
    pub expected_connected_ids: Option<Vec<ModelId>>,
    pub expected_direct_ids: Option<Vec<ModelId>>,
    pub verification_passed: Option<Arc<AtomicBool>>,
}

impl TestTransaction {
    pub fn new(id: ModelId, is_high_value: bool) -> Self {
        Self {
            id,
            is_high_value,
            created_at: Utc::now(),
            expected_connected_ids: None,
            expected_direct_ids: None,
            verification_passed: None,
        }
    }

    pub fn high_value() -> Self {
        Self::new(1, true)
    }

    pub fn low_value() -> Self {
        Self::new(2, false)
    }
    
    // Add constructor for connection verification
    pub fn connection_verifying(
        id: ModelId, 
        expected_connected_ids: Vec<ModelId>,
        expected_direct_ids: Vec<ModelId>
    ) -> Self {
        Self {
            id,
            is_high_value: false,
            created_at: Utc::now(),
            expected_connected_ids: Some(expected_connected_ids),
            expected_direct_ids: Some(expected_direct_ids),
            verification_passed: Some(Arc::new(AtomicBool::new(false))),
        }
    }
    
    pub fn verification_passed(&self) -> Option<Arc<AtomicBool>> {
        self.verification_passed.clone()
    }
}

#[async_trait]
impl Processible for TestTransaction {
    fn extract_simple_features(&self) -> Vec<Feature> {
        let mut features = Vec::new();
        
        features.push(Feature {
            name: "is_high_value".to_string(),
            value: Box::new(FeatureValue::Bool(self.is_high_value)),
        });
        
        features
    }

    fn extract_graph_features(
        &self,
        connected_transactions: &[ConnectedTransaction],
        direct_connections: &[DirectConnection]
    ) -> Vec<Feature> {
        let mut features = Vec::new();
        
        // Handle connection verification if enabled
        if let (Some(expected_connected_ids), Some(expected_direct_ids), Some(verification_passed)) = 
            (&self.expected_connected_ids, &self.expected_direct_ids, &self.verification_passed) {
            
            // Verify that we received the expected connected transaction IDs
            let connected_ids: Vec<ModelId> = connected_transactions.iter()
                .map(|ct| ct.transaction_id)
                .collect();
            
            // Verify that we received the expected direct connection IDs
            let direct_ids: Vec<ModelId> = direct_connections.iter()
                .map(|dc| dc.transaction_id)
                .collect();
            
            // Check if all expected connected IDs are present
            let all_connected_present = expected_connected_ids.iter()
                .all(|id| connected_ids.contains(id));
                
            // Check if all expected direct IDs are present
            let all_direct_present = expected_direct_ids.iter()
                .all(|id| direct_ids.contains(id));
            
            // Set verification passed flag if everything matches
            if all_connected_present && all_direct_present {
                verification_passed.store(true, Ordering::Relaxed);
            }
            
            // Add test feature for verification transactions
            features.push(Feature {
                name: "test_feature".to_string(),
                value: Box::new(FeatureValue::Int(42)),
            });
        }
        
        // Regular transaction features
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

    fn id(&self) -> ModelId {
        self.id
    }

    fn extract_matching_fields(&self) -> Vec<MatchingField> {
        if self.expected_connected_ids.is_some() {
            // Connection verification transaction
            vec![
                MatchingField {
                    matcher: "test_matcher".to_string(),
                    value: "test_value".to_string(),
                }
            ]
        } else {
            // Regular transaction
            vec![
                MatchingField {
                    matcher: "customer.email".to_string(),
                    value: "test@example.com".to_string(),
                },
                MatchingField {
                    matcher: "payment_details".to_string(),
                    value: "1234".to_string(),
                },
            ]
        }
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

mock! {
    pub CommonStorage {}

    #[async_trait]
    impl CommonStorage for CommonStorage {
        async fn save_transaction(&self) -> Result<ModelId, Box<dyn Error + Send + Sync>>;
        async fn mark_transaction_processed(&self, transaction_id: ModelId) -> Result<(), Box<dyn Error + Send + Sync>>;
        async fn save_features<'a>(
            &self,
            transaction_id: ModelId,
            simple_features: &'a Option<&'a[Feature]>,
            graph_features: &'a [Feature],
        ) -> Result<(), Box<dyn Error + Send + Sync>>;
        
        async fn get_features(
            &self,
            _transaction_id: i64,
        ) -> Result<(Option<Vec<Feature>>, Vec<Feature>), Box<dyn Error + Send + Sync>>;
    
        async fn save_scores(
            &self,
            _transaction_id: i64,
            _channel_id: i64,
            _total_score: i32,
            _triggered_rules: &[TriggeredRule],
        ) -> Result<(), Box<dyn Error + Send + Sync>>;
    
        async fn find_connected_transactions(
            &self,
            _transaction_id: i64,
            _max_depth: Option<i32>,
            _limit_count: Option<i32>,
            _min_created_at: Option<chrono::DateTime<chrono::Utc>>,
            _max_created_at: Option<chrono::DateTime<chrono::Utc>>,
            _min_confidence: Option<i32>,
        ) -> Result<Vec<ConnectedTransaction>, Box<dyn Error + Send + Sync>>;
    
        async fn get_direct_connections(
            &self,
            _transaction_id: ModelId
        ) -> Result<Vec<DirectConnection>, Box<dyn Error + Send + Sync>>;
    
        async fn save_matching_fields(
            &self,
            _transaction_id: i64,
            _matching_fields: &[MatchingField],
        ) -> Result<(), Box<dyn Error + Send + Sync>>;
    
        async fn get_channels(
            &self,
            _model_id: ModelId,
        ) -> Result<Vec<Channel>, Box<dyn Error + Send + Sync>>;
    
        async fn get_scoring_events(
            &self,
            _transaction_id: ModelId,
        ) -> Result<Vec<ScoringEvent>, Box<dyn Error + Send + Sync>>;
    
        async fn get_triggered_rules(
            &self,
            _scoring_event_id: ModelId,
        ) -> Result<Vec<TriggeredRule>, Box<dyn Error + Send + Sync>>;
    
        async fn update_transaction_label(
            &self,
            _transaction_id: ModelId,
            _label_id: ModelId,
        ) -> Result<(), Box<dyn Error + Send + Sync>>;
    
        async fn label_transactions(
            &self, 
            _transaction_ids: &[ModelId], 
            _fraud_level: FraudLevel,
            _fraud_category: String,
            _label_source: LabelSource,
            _labeled_by: String
        ) -> Result<(), Box<dyn Error + Send + Sync>>;
    }
}

pub fn create_mock_common_storage(tx_id: Option<i64>, features: Vec<Feature>) -> MockCommonStorage {
    let mut mock = MockCommonStorage::new();
    let default_tx_id = 1;
    
    mock.expect_save_transaction().returning(move|| Ok(tx_id.unwrap_or(default_tx_id)));
    mock.expect_mark_transaction_processed().returning(|_| Ok(()));
    mock.expect_save_features().returning(|_, _, _| Ok(()));
    mock.expect_get_features().returning( move |arg_tx_id|  {
        assert_eq!(tx_id.unwrap_or(default_tx_id), arg_tx_id);
        Ok((Some(features.clone()), features.clone()))
    });
    mock.expect_save_scores().returning(|_, _, _, _| Ok(()));
    mock.expect_find_connected_transactions().returning(|_, _, _, _, _, _| Ok(vec![]));
    mock.expect_get_direct_connections().returning(|_| Ok(vec![]));
    mock.expect_save_matching_fields().returning(|_, _| Ok(()));
    mock.expect_get_channels().returning(|_| Ok(vec![]));
    mock.expect_get_scoring_events().returning(|_| Ok(vec![]));
    mock.expect_get_triggered_rules().returning(|_| Ok(vec![]));
    mock.expect_update_transaction_label().returning(|_, _| Ok(()));
    
    mock
}

// Import and re-export centralized mocks for use in API tests
// Simple local mock implementations - TODO: Move to common when centralized
#[derive(Clone, serde::Serialize)]
pub struct MockWebTransaction {
    pub id: ModelId,
}

impl MockWebTransaction {
    pub fn new(id: ModelId) -> Self {
        Self { id }
    }
    
    pub fn get_id(&self) -> ModelId {
        self.id
    }
}

// Implement the required traits for MockWebTransaction
#[async_trait]
impl WebTransaction for MockWebTransaction {
    fn id(&self) -> ModelId {
        self.get_id()
    }
}



mock!{
    pub WebStorage {}

    #[async_trait]
    impl WebStorage<MockWebTransaction> for WebStorage {
        fn register_seaography_entities(&self, builder: Builder) -> Builder;
        fn get_connection(&self) -> &DatabaseConnection;
        async fn get_web_transaction(&self, transaction_id: ModelId) -> Result<MockWebTransaction, Box<dyn Error + Send + Sync>>;
    }
}

pub fn create_mock_web_storage(transactions: Vec<MockWebTransaction>) -> MockWebStorage {
    let mut mock_web_storage = MockWebStorage::new();
    mock_web_storage.expect_get_web_transaction().returning(move |transaction_id| {
        transactions.iter()
            .find(|t| t.id == transaction_id)
            .cloned()
            .ok_or_else(|| format!("Transaction {} not found", transaction_id).into())
    });
    mock_web_storage
}

// Connection tracking storage for testing
#[derive(Debug, Clone)]
pub struct ConnectionTrackingStorage {
    pub fetch_connected_called: Arc<AtomicBool>,
    pub fetch_direct_called: Arc<AtomicBool>,
    pub save_features_called: Arc<AtomicBool>,
    pub connected_transactions: Vec<ConnectedTransaction>,
    pub direct_connections: Vec<DirectConnection>,
}

impl ConnectionTrackingStorage {
    pub fn new(connected_transactions: Vec<ConnectedTransaction>, direct_connections: Vec<DirectConnection>) -> Self {
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
    async fn save_transaction(&self) -> Result<ModelId, Box<dyn Error + Send + Sync>> { Ok(1) }
    async fn mark_transaction_processed(&self, _transaction_id: ModelId) -> Result<(), Box<dyn Error + Send + Sync>> { Ok(()) }

    async fn save_features<'a>(
        &self,
        _transaction_id: i64,
        _simple_features: &'a Option<&'a [Feature]>,
        graph_features: &'a [Feature],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.save_features_called.store(true, Ordering::Relaxed);
        
        // Verify that connection counts are included in graph features
        let has_connected_count = graph_features.iter()
            .any(|f| f.name == "connected_transaction_count");
        let has_direct_count = graph_features.iter()
            .any(|f| f.name == "direct_connection_count");
        
        assert!(has_connected_count, "Graph features should include connected_transaction_count");
        assert!(has_direct_count, "Graph features should include direct_connection_count");
        
        Ok(())
    }

    async fn get_features(
        &self,
        _transaction_id: i64,
    ) -> Result<(Option<Vec<Feature>>, Vec<Feature>), Box<dyn Error + Send + Sync>> {
        Ok((None, Vec::new()))
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
        Ok(Vec::new())
    }

    async fn get_scoring_events(
        &self,
        _transaction_id: ModelId,
    ) -> Result<Vec<ScoringEvent>, Box<dyn Error + Send + Sync>> {
        Ok(Vec::new())
    }

    async fn get_triggered_rules(
        &self,
        _scoring_event_id: ModelId,
    ) -> Result<Vec<TriggeredRule>, Box<dyn Error + Send + Sync>> {
        Ok(Vec::new())
    }

    // removed save_label/get_label from trait

    async fn update_transaction_label(
        &self,
        _transaction_id: ModelId,
        _label_id: ModelId,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(())
    }

    async fn label_transactions(&self, _transaction_ids: &[ModelId], _fraud_level: FraudLevel, _fraud_category: String, _label_source: LabelSource, _labeled_by: String) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(())
    }
}

// Mock ProcessibleStorage using mockall - much cleaner than custom adapter!
mock! {
    pub ProcessibleStorage {}

    #[async_trait]
    impl ProcessibleStorage<TestTransaction> for ProcessibleStorage {
        async fn get_processible(&self, transaction_id: i64) -> Result<TestTransaction, Box<dyn std::error::Error + Send + Sync>>;
        async fn set_transaction_id(&self, processible_id: i64, transaction_id: i64) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    }
}

pub fn create_mock_processible_storage(transaction: Option<TestTransaction>) -> MockProcessibleStorage {
    let mut mock_processible_storage = MockProcessibleStorage::new();
        mock_processible_storage.expect_get_processible().returning( move |_| match transaction.clone() {
            Some(tx) => Ok(tx.clone()),
            None => Err("Transaction not found".into()),
        }
    );
    mock_processible_storage.expect_set_transaction_id().returning(|_, _| Ok(()));
    mock_processible_storage
}

// Direct MockQueueService implementation using mockall - OPTIMAL FIRST approach
use mockall::mock;

mock! {
    pub QueueService {}

    #[async_trait]
    impl QueueService for QueueService {
        async fn fetch_next(&self) -> Result<Option<ModelId>, Box<dyn Error + Send + Sync>>;
        async fn mark_processed(&self, id: ModelId) -> Result<(), Box<dyn Error + Send + Sync>>;
        async fn enqueue(&self, id: ModelId) -> Result<(), Box<dyn Error + Send + Sync>>;
    }
}

// Direct MockScorer implementation using mockall - OPTIMAL FIRST approach
mock! {
    pub Scorer {}

    #[async_trait]
    impl processing::scorers::Scorer for Scorer {
        async fn score(&self, features: Vec<Feature>) -> Vec<ScorerResult>;
    }
}

// Helper functions for common scorer configurations
pub fn create_high_value_scorer() -> MockScorer {
    let mut scorer = MockScorer::new();
    scorer.expect_score()
        .returning(|_| vec![
            ScorerResult { score: 85, name: "high_amount_score".to_string() },
            ScorerResult { score: 70, name: "premium_category_score".to_string() },
        ]);
    scorer
}

pub fn create_low_value_scorer() -> MockScorer {
    let mut scorer = MockScorer::new();
    scorer.expect_score()
        .returning(|_| vec![
            ScorerResult { score: 25, name: "low_amount_score".to_string() },
        ]);
    scorer
}

pub fn create_empty_scorer() -> MockScorer {
    let mut scorer = MockScorer::new();
    scorer.expect_score()
        .returning(|_| vec![]);
    scorer
} 
