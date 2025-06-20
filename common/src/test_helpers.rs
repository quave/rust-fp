/// Shared Test Helpers for Cross-Crate Use
/// 
/// This module provides centralized test utilities that can be used across
/// both the `processing` and `ecom` crates to avoid code duplication.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use std::error::Error;

// Global counter for truly unique test identifiers across parallel tests
static GLOBAL_TEST_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Generate globally unique test identifiers that won't conflict across parallel tests
/// 
/// This creates IDs using timestamp + atomic counter to ensure uniqueness even when
/// running tests in parallel across multiple threads and crates.
/// 
/// # Arguments
/// * `prefix` - A string prefix to identify the test type (e.g., "SAVE-TX", "GET-TX")
/// 
/// # Returns
/// A unique string in the format: "{prefix}-{timestamp}-{counter}"
pub fn generate_unique_id(prefix: &str) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let counter = GLOBAL_TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("{}-{}-{}", prefix, timestamp, counter)
}

/// Generate a unique numeric test ID for ModelId usage
/// 
/// Combines thread information, timestamp, and atomic counter for maximum uniqueness.
/// Used primarily in processing tests where numeric IDs are required.
/// 
/// # Returns
/// A unique numeric ID suitable for use as ModelId
pub fn generate_unique_test_id() -> u64 {
    use std::thread;
    
    let thread_id = thread::current().id();
    let thread_hash = format!("{:?}", thread_id)
        .chars()
        .map(|c| c as u64)
        .sum::<u64>() % 10000;
    
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    
    let counter = GLOBAL_TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    
    // Combine all three to create a truly unique ID
    // Format: timestamp_low_bits + thread_hash_shifted + counter
    (timestamp % 100000) * 1_000_000 + thread_hash * 100 + counter
}

/// Get the test database URL from environment or default
/// 
/// This centralizes database URL configuration for all test suites.
/// 
/// # Returns
/// The database URL string for test connections
pub fn get_test_database_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5435/frida_test".to_string())
}

/// Get an in-memory SQLite database URL for unit tests
/// 
/// Used for tests that don't need a real PostgreSQL database.
/// 
/// # Returns
/// SQLite in-memory database URL string
pub fn get_test_in_memory_database_url() -> String {
    "sqlite::memory:".to_string()
}

// =============================================================================
// CENTRALIZED MOCK IMPLEMENTATIONS
// =============================================================================

#[cfg(feature = "async-sqlx")]
pub mod mocks {
    use std::collections::{VecDeque, HashMap};
    use std::sync::{Mutex, Arc};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::error::Error;

    // =============================================================================
    // QUEUE MOCKS
    // =============================================================================

    /// Simple mock queue implementation for testing
    /// 
    /// This provides a basic queue implementation that can be used across
    /// different crates for testing queue-based functionality.
    #[derive(Default)]
    pub struct MockQueue {
        queue: Mutex<VecDeque<u64>>,
        fetch_next_result: Option<u64>,
        should_fail: bool,
    }

    impl MockQueue {
        /// Create a new empty mock queue
        pub fn new() -> Self {
            Self::default()
        }

        /// Create a mock queue that will return a specific ID on fetch_next
        pub fn with_next_id(next_id: u64) -> Self {
            Self {
                queue: Mutex::new(VecDeque::new()),
                fetch_next_result: Some(next_id),
                should_fail: false,
            }
        }

        /// Create a mock queue that will fail on operations
        pub fn failing() -> Self {
            Self {
                queue: Mutex::new(VecDeque::new()),
                fetch_next_result: None,
                should_fail: true,
            }
        }

        /// Add an item to the queue
        pub async fn enqueue(&self, item: u64) -> Result<(), TestError> {
            if self.should_fail {
                return Err(TestError::mock_failure("Mock queue failure"));
            }
            self.queue.lock().unwrap().push_back(item);
            Ok(())
        }

        /// Get the next item from the queue
        pub async fn fetch_next(&self) -> Result<Option<u64>, TestError> {
            if self.should_fail {
                return Err(TestError::mock_failure("Mock queue failure"));
            }
            
            if let Some(result) = self.fetch_next_result {
                return Ok(Some(result));
            }
            
            Ok(self.queue.lock().unwrap().pop_front())
        }

        /// Mark an item as processed (mock implementation)
        pub async fn mark_processed(&self, _item: u64) -> Result<(), TestError> {
            if self.should_fail {
                return Err(TestError::mock_failure("Mock queue failure"));
            }
            // Mock implementation - just return success
            Ok(())
        }
    }

    // =============================================================================
    // STORAGE MOCKS
    // =============================================================================

    /// Test data factory for common model objects
    /// 
    /// This struct provides factory methods to create consistent test data
    /// across different test suites.
    pub struct TestDataFactory;

    impl TestDataFactory {
        /// Create a basic feature for testing
        pub fn create_feature(name: &str, value: i32) -> TestFeature {
            TestFeature {
                name: name.to_string(),
                int_value: Some(value),
                string_value: None,
            }
        }

        /// Create a string feature for testing
        pub fn create_string_feature(name: &str, value: &str) -> TestFeature {
            TestFeature {
                name: name.to_string(),
                int_value: None,
                string_value: Some(value.to_string()),
            }
        }

        /// Create a matching field for testing
        pub fn create_matching_field(matcher: &str, value: &str) -> TestMatchingField {
            TestMatchingField {
                matcher: matcher.to_string(),
                value: value.to_string(),
            }
        }

        /// Create a test scorer result
        pub fn create_scorer_result(name: &str, score: i32) -> TestScorerResult {
            TestScorerResult {
                name: name.to_string(),
                score,
            }
        }

        /// Create a test transaction
        pub fn create_transaction(id: u64) -> TestTransaction {
            TestTransaction {
                id,
                features: vec![
                    Self::create_feature("test_feature", 42),
                    Self::create_string_feature("test_string", "test_value"),
                ],
                matching_fields: vec![
                    Self::create_matching_field("test_matcher", "test_value"),
                ],
            }
        }

        /// Create a high-value test transaction
        pub fn create_high_value_transaction(id: u64) -> TestTransaction {
            TestTransaction {
                id,
                features: vec![
                    Self::create_feature("value", 10000),
                    Self::create_string_feature("type", "high_value"),
                ],
                matching_fields: vec![
                    Self::create_matching_field("value_matcher", "high"),
                ],
            }
        }

        /// Create a low-value test transaction
        pub fn create_low_value_transaction(id: u64) -> TestTransaction {
            TestTransaction {
                id,
                features: vec![
                    Self::create_feature("value", 100),
                    Self::create_string_feature("type", "low_value"),
                ],
                matching_fields: vec![
                    Self::create_matching_field("value_matcher", "low"),
                ],
            }
        }
    }

    // =============================================================================
    // TEST DATA STRUCTURES
    // =============================================================================

    /// Generic feature struct for testing
    #[derive(Debug, Clone, PartialEq)]
    pub struct TestFeature {
        pub name: String,
        pub int_value: Option<i32>,
        pub string_value: Option<String>,
    }

    /// Generic matching field struct for testing
    #[derive(Debug, Clone, PartialEq)]
    pub struct TestMatchingField {
        pub matcher: String,
        pub value: String,
    }

    /// Generic scorer result for testing
    #[derive(Debug, Clone, PartialEq)]
    pub struct TestScorerResult {
        pub name: String,
        pub score: i32,
    }

    /// Generic transaction struct for testing
    #[derive(Debug, Clone)]
    pub struct TestTransaction {
        pub id: u64,
        pub features: Vec<TestFeature>,
        pub matching_fields: Vec<TestMatchingField>,
    }

    impl TestTransaction {
        pub fn new(id: u64) -> Self {
            TestDataFactory::create_transaction(id)
        }

        pub fn high_value(id: u64) -> Self {
            TestDataFactory::create_high_value_transaction(id)
        }

        pub fn low_value(id: u64) -> Self {
            TestDataFactory::create_low_value_transaction(id)
        }

        pub fn with_features(id: u64, features: Vec<TestFeature>) -> Self {
            Self {
                id,
                features,
                matching_fields: vec![TestDataFactory::create_matching_field("default", "test")],
            }
        }

        pub fn with_matching_fields(id: u64, matching_fields: Vec<TestMatchingField>) -> Self {
            Self {
                id,
                features: vec![TestDataFactory::create_feature("default", 1)],
                matching_fields,
            }
        }
    }

    // =============================================================================
    // CONFIGURABLE MOCKS WITH TRACKING
    // =============================================================================

    /// Mock scorer with configurable behavior and call tracking
    #[derive(Debug, Clone)]
    pub struct MockScorer {
        pub called: Arc<AtomicBool>,
        pub expected_features: Vec<TestFeature>,
        pub results: Vec<TestScorerResult>,
        pub should_fail: bool,
    }

    impl MockScorer {
        pub fn new(results: Vec<TestScorerResult>) -> Self {
            Self {
                called: Arc::new(AtomicBool::new(false)),
                expected_features: Vec::new(),
                results,
                should_fail: false,
            }
        }

        pub fn with_expected_features(results: Vec<TestScorerResult>, expected_features: Vec<TestFeature>) -> Self {
            Self {
                called: Arc::new(AtomicBool::new(false)),
                expected_features,
                results,
                should_fail: false,
            }
        }

        pub fn failing() -> Self {
            Self {
                called: Arc::new(AtomicBool::new(false)),
                expected_features: Vec::new(),
                results: Vec::new(),
                should_fail: true,
            }
        }

        pub fn high_value_scorer() -> Self {
            Self::new(vec![TestDataFactory::create_scorer_result("high_value", 100)])
        }

        pub fn low_value_scorer() -> Self {
            Self::new(vec![TestDataFactory::create_scorer_result("low_value", 10)])
        }

        pub fn was_called(&self) -> bool {
            self.called.load(Ordering::Relaxed)
        }

        pub async fn score(&self, features: Vec<TestFeature>) -> Result<Vec<TestScorerResult>, TestError> {
            self.called.store(true, Ordering::Relaxed);
            
            if self.should_fail {
                return Err(TestError::mock_failure("Mock scorer failure"));
            }

            if !self.expected_features.is_empty() {
                test_assert_eq!(features, self.expected_features, "Features don't match expected");
            }

            Ok(self.results.clone())
        }
    }

    /// Mock storage with configurable behavior and comprehensive call tracking
    #[derive(Debug, Clone)]
    pub struct MockStorage {
        pub transaction_id: u64,
        pub features: Vec<TestFeature>,
        pub matching_fields: Vec<TestMatchingField>,
        pub scores: Vec<TestScorerResult>,
        
        // Call tracking
        pub save_features_called: Arc<AtomicBool>,
        pub save_scores_called: Arc<AtomicBool>,
        pub save_matching_fields_called: Arc<AtomicBool>,
        pub get_features_called: Arc<AtomicBool>,
        
        // Behavior configuration
        pub should_fail_save_features: bool,
        pub should_fail_save_scores: bool,
        pub should_fail_save_matching_fields: bool,
        pub should_fail_get_features: bool,
        
        // Custom configurations
        pub matcher_configs: HashMap<String, String>,
    }

    impl MockStorage {
        pub fn new(transaction_id: u64) -> Self {
            Self {
                transaction_id,
                features: vec![TestDataFactory::create_feature("default", 1)],
                matching_fields: vec![TestDataFactory::create_matching_field("default", "test")],
                scores: vec![TestDataFactory::create_scorer_result("default", 50)],
                save_features_called: Arc::new(AtomicBool::new(false)),
                save_scores_called: Arc::new(AtomicBool::new(false)),
                save_matching_fields_called: Arc::new(AtomicBool::new(false)),
                get_features_called: Arc::new(AtomicBool::new(false)),
                should_fail_save_features: false,
                should_fail_save_scores: false,
                should_fail_save_matching_fields: false,
                should_fail_get_features: false,
                matcher_configs: HashMap::new(),
            }
        }

        pub fn with_features(transaction_id: u64, features: Vec<TestFeature>) -> Self {
            let mut mock = Self::new(transaction_id);
            mock.features = features;
            mock
        }

        pub fn with_scores(transaction_id: u64, scores: Vec<TestScorerResult>) -> Self {
            let mut mock = Self::new(transaction_id);
            mock.scores = scores;
            mock
        }

        pub fn with_matcher_configs(transaction_id: u64, configs: HashMap<String, String>) -> Self {
            let mut mock = Self::new(transaction_id);
            mock.matcher_configs = configs;
            mock
        }

        pub fn failing_save_features(transaction_id: u64) -> Self {
            let mut mock = Self::new(transaction_id);
            mock.should_fail_save_features = true;
            mock
        }

        pub fn failing_save_scores(transaction_id: u64) -> Self {
            let mut mock = Self::new(transaction_id);
            mock.should_fail_save_scores = true;
            mock
        }

        // Call tracking methods
        pub fn was_save_features_called(&self) -> bool {
            self.save_features_called.load(Ordering::Relaxed)
        }

        pub fn was_save_scores_called(&self) -> bool {
            self.save_scores_called.load(Ordering::Relaxed)
        }

        pub fn was_save_matching_fields_called(&self) -> bool {
            self.save_matching_fields_called.load(Ordering::Relaxed)
        }

        pub fn was_get_features_called(&self) -> bool {
            self.get_features_called.load(Ordering::Relaxed)
        }

        // Storage operation methods (these would be adapted to specific storage traits)
        pub async fn save_features(&self, tx_id: u64, features: &[TestFeature]) -> Result<(), TestError> {
            self.save_features_called.store(true, Ordering::Relaxed);
            
            if self.should_fail_save_features {
                return Err(TestError::mock_failure("Mock storage save_features failure"));
            }

            test_assert_eq!(tx_id, self.transaction_id, "Transaction ID mismatch");
            
            if !self.features.is_empty() {
                test_assert_eq!(features, &self.features, "Features don't match expected");
            }
            
            Ok(())
        }

        pub async fn save_scores(&self, tx_id: u64, scores: &[TestScorerResult]) -> Result<(), TestError> {
            self.save_scores_called.store(true, Ordering::Relaxed);
            
            if self.should_fail_save_scores {
                return Err(TestError::mock_failure("Mock storage save_scores failure"));
            }

            test_assert_eq!(tx_id, self.transaction_id, "Transaction ID mismatch");
            test_assert_eq!(scores.len(), self.scores.len(), "Score count mismatch");
            
            Ok(())
        }

        pub async fn save_matching_fields(&self, tx_id: u64, fields: &[TestMatchingField]) -> Result<(), TestError> {
            self.save_matching_fields_called.store(true, Ordering::Relaxed);
            
            if self.should_fail_save_matching_fields {
                return Err(TestError::mock_failure("Mock storage save_matching_fields failure"));
            }

            test_assert_eq!(tx_id, self.transaction_id, "Transaction ID mismatch");
            
            if !fields.is_empty() {
                test_assert_eq!(fields, &self.matching_fields, "Matching fields don't match expected");
            }
            
            Ok(())
        }

        pub async fn get_features(&self, tx_id: u64) -> Result<Vec<TestFeature>, TestError> {
            self.get_features_called.store(true, Ordering::Relaxed);
            
            if self.should_fail_get_features {
                return Err(TestError::mock_failure("Mock storage get_features failure"));
            }

            test_assert_eq!(tx_id, self.transaction_id, "Transaction ID mismatch");
            Ok(self.features.clone())
        }
    }

    /// Mock processible storage for testing
    #[derive(Debug)]
    pub struct MockProcessibleStorage {
        pub transaction: Option<TestTransaction>,
        pub get_called: Arc<AtomicBool>,
        pub should_fail: bool,
    }

    impl MockProcessibleStorage {
        pub fn new(transaction: TestTransaction) -> Self {
            Self {
                transaction: Some(transaction),
                get_called: Arc::new(AtomicBool::new(false)),
                should_fail: false,
            }
        }

        pub fn empty() -> Self {
            Self {
                transaction: None,
                get_called: Arc::new(AtomicBool::new(false)),
                should_fail: false,
            }
        }

        pub fn failing() -> Self {
            Self {
                transaction: None,
                get_called: Arc::new(AtomicBool::new(false)),
                should_fail: true,
            }
        }

        pub fn was_get_called(&self) -> bool {
            self.get_called.load(Ordering::Relaxed)
        }

        pub async fn get_processible(&self, tx_id: u64) -> Result<TestTransaction, TestError> {
            self.get_called.store(true, Ordering::Relaxed);

            if self.should_fail {
                return Err(TestError::mock_failure("Mock processible storage failure"));
            }

            match &self.transaction {
                Some(tx) => {
                    test_assert_eq!(tx_id, tx.id, "Transaction ID mismatch");
                    Ok(tx.clone())
                }
                None => Err(TestError::transaction_not_found(tx_id as i64)),
            }
        }
    }
}

#[cfg(feature = "async-sqlx")]
pub use mocks::*;

// =============================================================================
// PROCESSING-SPECIFIC MOCKS
// =============================================================================

/// Processing-specific mock implementations that adapt to actual processing types
/// This module provides mocks that implement the actual traits used in the processing crate
#[cfg(feature = "processing-mocks")]
pub mod processing_mocks {
    use super::mocks::*;
    use std::sync::{Arc};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::collections::HashMap;
    use std::error::Error;

    // These would need to be imported from the processing crate when used
    // For now, we'll define minimal trait signatures that match the processing crate

    /// Mock implementation of CommonStorage trait from processing crate
    #[derive(Debug, Clone)]
    pub struct MockCommonStorage {
        pub transaction_id: i64,
        pub features: Vec<TestFeature>,
        pub matching_fields: Vec<TestMatchingField>,
        pub scores: Vec<TestScorerResult>,
        
        // Call tracking
        pub save_features_called: Arc<AtomicBool>,
        pub save_scores_called: Arc<AtomicBool>,
        pub save_matching_fields_called: Arc<AtomicBool>,
        pub get_features_called: Arc<AtomicBool>,
        
        // Behavior configuration
        pub should_fail_save_features: bool,
        pub should_fail_save_scores: bool,
        pub should_fail_save_matching_fields: bool,
        pub should_fail_get_features: bool,
        
        // Custom configurations
        pub matcher_configs: HashMap<String, String>,
    }

    impl MockCommonStorage {
        pub fn new(transaction_id: i64, features: Vec<TestFeature>, scores: Vec<TestScorerResult>) -> Self {
            Self {
                transaction_id,
                features,
                scores,
                matching_fields: vec![TestDataFactory::create_matching_field("default", "test")],
                save_features_called: Arc::new(AtomicBool::new(false)),
                save_scores_called: Arc::new(AtomicBool::new(false)),
                save_matching_fields_called: Arc::new(AtomicBool::new(false)),
                get_features_called: Arc::new(AtomicBool::new(false)),
                should_fail_save_features: false,
                should_fail_save_scores: false,
                should_fail_save_matching_fields: false,
                should_fail_get_features: false,
                matcher_configs: HashMap::new(),
            }
        }

        pub fn with_matcher_configs(
            transaction_id: i64, 
            features: Vec<TestFeature>, 
            scores: Vec<TestScorerResult>, 
            matcher_configs: HashMap<String, String>
        ) -> Self {
            let mut mock = Self::new(transaction_id, features, scores);
            mock.matcher_configs = matcher_configs;
            mock
        }

        pub fn empty(transaction_id: i64) -> Self {
            Self::new(transaction_id, Vec::new(), Vec::new())
        }

        // Call tracking methods
        pub fn was_save_features_called(&self) -> bool {
            self.save_features_called.load(Ordering::Relaxed)
        }

        pub fn was_save_scores_called(&self) -> bool {
            self.save_scores_called.load(Ordering::Relaxed)
        }

        pub fn was_save_matching_fields_called(&self) -> bool {
            self.save_matching_fields_called.load(Ordering::Relaxed)
        }

        pub fn was_get_features_called(&self) -> bool {
            self.get_features_called.load(Ordering::Relaxed)
        }
    }

    /// Mock implementation of Processible trait
    #[derive(Debug, Clone)]
    pub struct MockProcessible {
        pub id: i64,
        pub features: Vec<TestFeature>,
        pub matching_fields: Vec<TestMatchingField>,
    }

    impl MockProcessible {
        pub fn new(id: i64) -> Self {
            Self {
                id,
                features: vec![TestDataFactory::create_feature("test_feature", 42)],
                matching_fields: vec![TestDataFactory::create_matching_field("test_matcher", "test_value")],
            }
        }

        pub fn with_features(id: i64, features: Vec<TestFeature>) -> Self {
            Self {
                id,
                features,
                matching_fields: vec![TestDataFactory::create_matching_field("default", "test")],
            }
        }

        pub fn high_value(id: i64) -> Self {
            Self {
                id,
                features: vec![
                    TestDataFactory::create_feature("value", 10000),
                    TestDataFactory::create_string_feature("type", "high_value"),
                ],
                matching_fields: vec![TestDataFactory::create_matching_field("value_matcher", "high")],
            }
        }

        pub fn low_value(id: i64) -> Self {
            Self {
                id,
                features: vec![
                    TestDataFactory::create_feature("value", 100),
                    TestDataFactory::create_string_feature("type", "low_value"),
                ],
                matching_fields: vec![TestDataFactory::create_matching_field("value_matcher", "low")],
            }
        }

        pub fn tx_id(&self) -> i64 {
            self.id
        }

        pub fn get_id(&self) -> i64 {
            self.id
        }

        pub fn extract_simple_features(&self) -> Vec<TestFeature> {
            self.features.clone()
        }

        pub fn extract_graph_features(&self, _connected_transactions: &[TestConnectedTransaction], _direct_connections: &[TestDirectConnection]) -> Vec<TestFeature> {
            self.features.clone()
        }

        pub fn extract_matching_fields(&self) -> Vec<TestMatchingField> {
            self.matching_fields.clone()
        }
    }

    /// Mock QueueService implementation
    #[derive(Debug, Clone)]
    pub struct MockQueueService {
        pub fetch_next_called: Arc<AtomicBool>,
        pub mark_processed_called: Arc<AtomicBool>,
        pub enqueue_called: Arc<AtomicBool>,
        pub return_value: Option<i64>,
        pub expected_mark_processed_id: i64,
        pub should_fail: bool,
    }

    impl MockQueueService {
        pub fn new(return_value: Option<i64>, expected_mark_processed_id: i64) -> Self {
            Self {
                fetch_next_called: Arc::new(AtomicBool::new(false)),
                mark_processed_called: Arc::new(AtomicBool::new(false)),
                enqueue_called: Arc::new(AtomicBool::new(false)),
                return_value,
                expected_mark_processed_id,
                should_fail: false,
            }
        }

        pub fn empty() -> Self {
            Self::new(None, 0)
        }

        pub fn failing() -> Self {
            let mut service = Self::new(None, 0);
            service.should_fail = true;
            service
        }

        pub fn was_fetch_next_called(&self) -> bool {
            self.fetch_next_called.load(Ordering::Relaxed)
        }

        pub fn was_mark_processed_called(&self) -> bool {
            self.mark_processed_called.load(Ordering::Relaxed)
        }

        pub fn was_enqueue_called(&self) -> bool {
            self.enqueue_called.load(Ordering::Relaxed)
        }

        pub async fn fetch_next(&self) -> Result<Option<i64>, TestError> {
            self.fetch_next_called.store(true, Ordering::Relaxed);
            
            if self.should_fail {
                return Err(TestError::mock_failure("Mock queue service failure"));
            }
            
            Ok(self.return_value)
        }

        pub async fn mark_processed(&self, tx_id: i64) -> Result<(), TestError> {
            self.mark_processed_called.store(true, Ordering::Relaxed);
            
            if self.should_fail {
                return Err(TestError::mock_failure("Mock queue service failure"));
            }
            
            test_assert_eq!(tx_id, self.expected_mark_processed_id, "Mark processed ID mismatch");
            Ok(())
        }

        pub async fn enqueue(&self, _tx_id: i64) -> Result<(), TestError> {
            self.enqueue_called.store(true, Ordering::Relaxed);
            
            if self.should_fail {
                return Err(TestError::mock_failure("Mock queue service failure"));
            }
            
            Ok(())
        }
    }

    // Supporting test data structures for processing-specific mocks
    #[derive(Debug, Clone, PartialEq)]
    pub struct TestConnectedTransaction {
        pub id: i64,
        pub connection_type: String,
        pub confidence: i32,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct TestDirectConnection {
        pub id: i64,
        pub connection_type: String,
        pub strength: i32,
    }

    // Web-specific mocks for API testing
    #[derive(Debug, Clone, serde::Serialize)]
    pub struct MockWebTransaction {
        pub id: i64,
        pub label_id: Option<i64>,
    }

    impl MockWebTransaction {
        pub fn new(id: i64) -> Self {
            Self { id, label_id: None }
        }

        pub fn with_label(id: i64, label_id: i64) -> Self {
            Self { id, label_id: Some(label_id) }
        }

        pub fn get_id(&self) -> i64 {
            self.id
        }
    }

    #[derive(Debug)]
    pub struct MockWebStorage {
        pub transactions: Vec<MockWebTransaction>,
        pub get_transaction_called: Arc<AtomicBool>,
        pub get_transactions_called: Arc<AtomicBool>,
    }

    impl MockWebStorage {
        pub fn new(transactions: Vec<MockWebTransaction>) -> Self {
            Self {
                transactions,
                get_transaction_called: Arc::new(AtomicBool::new(false)),
                get_transactions_called: Arc::new(AtomicBool::new(false)),
            }
        }

        pub fn empty() -> Self {
            Self::new(Vec::new())
        }

        pub fn was_get_transaction_called(&self) -> bool {
            self.get_transaction_called.load(Ordering::Relaxed)
        }

        pub fn was_get_transactions_called(&self) -> bool {
            self.get_transactions_called.load(Ordering::Relaxed)
        }

        pub async fn get_transaction(&self, transaction_id: i64) -> Result<MockWebTransaction, TestError> {
            self.get_transaction_called.store(true, Ordering::Relaxed);
            
            self.transactions
                .iter()
                .find(|t| t.id == transaction_id)
                .cloned()
                .ok_or_else(|| TestError::transaction_not_found(transaction_id))
        }

        pub async fn get_transactions(&self) -> Result<Vec<MockWebTransaction>, TestError> {
            self.get_transactions_called.store(true, Ordering::Relaxed);
            Ok(self.transactions.clone())
        }
    }
}

#[cfg(feature = "processing-mocks")]
pub use processing_mocks::*;

#[cfg(feature = "async-sqlx")]
mod sqlx_helpers {
    use super::*;
    use sqlx::PgPool;
    use std::path::PathBuf;
    use std::error::Error;

    /// Create a database connection pool for testing
    /// 
    /// # Returns
    /// A Result containing the PgPool or an error
    pub async fn create_test_pool() -> Result<PgPool, Box<dyn Error + Send + Sync>> {
        let database_url = get_test_database_url();
        let pool = PgPool::connect(&database_url).await?;
        Ok(pool)
    }

    /// Global test setup function that should be called before running any tests
    /// 
    /// This initializes the test environment including database schema setup.
    pub async fn setup_test_environment() -> Result<(), Box<dyn Error + Send + Sync>> {
        let pool = create_test_pool().await?;
        initialize_test_schema(&pool).await?;
        Ok(())
    }

    /// Initialize database schema for testing using SQLx migrations
    /// 
    /// This sets up the database schema by running migrations and verifying
    /// that required tables exist.
    /// 
    /// # Arguments
    /// * `pool` - The database connection pool
    /// 
    /// # Returns
    /// Result indicating success or failure of schema initialization
    pub async fn initialize_test_schema(pool: &PgPool) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Get the absolute path to the migrations directory
        let workspace_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
        
        let migrations_dir = workspace_dir.join("migrations");
        
        // Run migrations if the directory exists
        if migrations_dir.exists() {
            println!("Running migrations from: {:?}", migrations_dir);
            sqlx::migrate::Migrator::new(migrations_dir)
                .await?
                .run(pool)
                .await?;
        } else {
            println!("Migrations directory not found at: {:?}, skipping migrations", migrations_dir);
        }
        
        // Verify core tables exist
        verify_core_tables(pool).await?;
        
        Ok(())
    }
    
    /// Verify that core tables required for testing exist
    async fn verify_core_tables(pool: &PgPool) -> Result<(), Box<dyn Error + Send + Sync>> {
        // This is a basic check - can be expanded based on actual schema requirements
        let table_check = sqlx::query("SELECT 1")
            .fetch_optional(pool)
            .await;
        
        match table_check {
            Ok(_) => {
                println!("Database connection verified");
                Ok(())
            }
            Err(e) => {
                println!("Database verification failed: {}", e);
                Err(e.into())
            }
        }
    }

    /// Clean up test data - truncate processing tables
    /// 
    /// This function truncates tables used in processing tests to ensure
    /// test isolation.
    /// 
    /// # Arguments
    /// * `pool` - The database connection pool
    /// 
    /// # Returns
    /// Result indicating success or failure of cleanup
    pub async fn truncate_processing_tables(pool: &PgPool) -> Result<(), Box<dyn Error + Send + Sync>> {
        // List of tables to truncate for processing tests (includes ecom tables since they share the same database)
        let tables = vec![
            // Ecom tables (must come first due to foreign key dependencies)
            "order_items",
            "customers", 
            "billing_data",
            "orders",
            // Match node tables (must come before match_node due to foreign key)
            "match_node_transactions", 
            "match_node",
            // Core processing tables
            "transactions",
            "features", 
            "scores",
            "matching_fields",
            "connected_transactions",
            "direct_connections",
            "channels",
            "scoring_events",
            "triggered_rules",
            "labels"
        ];

        for table in tables {
            let query = format!("TRUNCATE TABLE {} RESTART IDENTITY CASCADE", table);
            match sqlx::query(&query).execute(pool).await {
                Ok(_) => println!("Truncated table: {}", table),
                Err(e) => {
                    // Don't fail if table doesn't exist, just log it
                    println!("Could not truncate table {}: {} (table may not exist)", table, e);
                }
            }
        }

        Ok(())
    }

    /// Clean up test data - truncate ecom tables
    /// 
    /// This function truncates tables used in ecom tests to ensure
    /// test isolation.
    /// 
    /// # Arguments
    /// * `pool` - The database connection pool
    /// 
    /// # Returns
    /// Result indicating success or failure of cleanup
    pub async fn truncate_ecom_tables(pool: &PgPool) -> Result<(), Box<dyn Error + Send + Sync>> {
        // List of tables to truncate for ecom tests
        let tables = vec![
            "ecom_transactions",
            "ecom_queue",
            "ecom_import_jobs"
        ];

        for table in tables {
            let query = format!("TRUNCATE TABLE {} RESTART IDENTITY CASCADE", table);
            match sqlx::query(&query).execute(pool).await {
                Ok(_) => println!("Truncated table: {}", table),
                Err(e) => {
                    // Don't fail if table doesn't exist, just log it
                    println!("Could not truncate table {}: {} (table may not exist)", table, e);
                }
            }
        }

        Ok(())
    }
}

// Export sqlx helper functions at the module level
#[cfg(feature = "async-sqlx")]
pub use sqlx_helpers::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_generate_unique_id() {
        // Generate multiple IDs and ensure they're unique
        let mut ids = HashSet::new();
        for i in 0..1000 {
            let id = generate_unique_id(&format!("TEST-{}", i));
            assert!(ids.insert(id.clone()), "Duplicate ID generated: {}", id);
        }
    }

    #[test]
    fn test_generate_unique_test_id() {
        // Generate multiple numeric IDs and ensure they're unique
        let mut ids = HashSet::new();
        for _ in 0..1000 {
            let id = generate_unique_test_id();
            assert!(ids.insert(id), "Duplicate numeric ID generated: {}", id);
        }
    }

    #[test]
    fn test_database_url_configuration() {
        // Test that database URL returns a valid string
        let url = get_test_database_url();
        assert!(url.starts_with("postgres://"));
        
        let in_memory_url = get_test_in_memory_database_url();
        assert_eq!(in_memory_url, "sqlite::memory:");
    }

    #[cfg(feature = "async-sqlx")]
    #[tokio::test]
    async fn test_mock_queue() {
        let queue = MockQueue::new();
        
        // Test enqueue and fetch
        queue.enqueue(42).await.unwrap();
        let item = queue.fetch_next().await.unwrap();
        assert_eq!(item, Some(42));
        
        // Test empty queue
        let empty = queue.fetch_next().await.unwrap();
        assert_eq!(empty, None);
    }
}

// =============================================================================
// UNIFIED TEST ERROR HANDLING
// =============================================================================

/// Unified error type for all test failures
/// 
/// This provides a consistent error interface across all test suites,
/// making debugging easier and error handling more predictable.
#[derive(Debug, thiserror::Error)]
pub enum TestError {
    #[error("Mock failure: {message}")]
    MockFailure { message: String },
    
    #[error("Assertion failed: {message}")]
    AssertionFailure { message: String },
    
    #[error("Setup error: {message}")]
    SetupError { message: String },
    
    #[error("Database error: {source}")]
    DatabaseError { #[from] source: sqlx::Error },
    
    #[error("Serialization error: {source}")]
    SerializationError { #[from] source: serde_json::Error },
    
    #[error("HTTP error: {source}")]
    HttpError { #[from] source: http::Error },
    
    #[error("Request error: {source}")]
    RequestError { #[from] source: reqwest::Error },
    
    #[error("Validation error: {message}")]
    ValidationError { message: String },
    
    #[error("Feature type mismatch: expected {expected}, got {actual}")]
    FeatureTypeMismatch { expected: String, actual: String },
    
    #[error("Transaction not found: {id}")]
    TransactionNotFound { id: i64 },
    
    #[error("Test timeout: {operation} took longer than {timeout_ms}ms")]
    Timeout { operation: String, timeout_ms: u64 },
    
    #[error("Generic test error: {message}")]
    Generic { message: String },
}

impl TestError {
    /// Create a mock failure error
    pub fn mock_failure(message: impl Into<String>) -> Self {
        Self::MockFailure { message: message.into() }
    }
    
    /// Create an assertion failure error
    pub fn assertion_failure(message: impl Into<String>) -> Self {
        Self::AssertionFailure { message: message.into() }
    }
    
    /// Create a setup error
    pub fn setup_error(message: impl Into<String>) -> Self {
        Self::SetupError { message: message.into() }
    }
    
    /// Create a validation error
    pub fn validation_error(message: impl Into<String>) -> Self {
        Self::ValidationError { message: message.into() }
    }
    
    /// Create a feature type mismatch error
    pub fn feature_type_mismatch(expected: impl Into<String>, actual: impl Into<String>) -> Self {
        Self::FeatureTypeMismatch { 
            expected: expected.into(), 
            actual: actual.into() 
        }
    }
    
    /// Create a transaction not found error
    pub fn transaction_not_found(id: i64) -> Self {
        Self::TransactionNotFound { id }
    }
    
    /// Create a timeout error
    pub fn timeout(operation: impl Into<String>, timeout_ms: u64) -> Self {
        Self::Timeout { 
            operation: operation.into(), 
            timeout_ms 
        }
    }
    
    /// Create a generic error
    pub fn generic(message: impl Into<String>) -> Self {
        Self::Generic { message: message.into() }
    }
}

/// Alias for the standard test result type
pub type TestResult<T = ()> = Result<T, TestError>;

/// Helper macro for test assertions that return TestError instead of panicking
#[macro_export]
macro_rules! test_assert {
    ($condition:expr) => {
        if !($condition) {
            return Err($crate::test_helpers::TestError::assertion_failure(
                format!("assertion failed: {}", stringify!($condition))
            ));
        }
    };
    ($condition:expr, $message:expr $(, $arg:expr)*) => {
        if !($condition) {
            return Err($crate::test_helpers::TestError::assertion_failure(
                format!($message $(, $arg)*)
            ));
        }
    };
}

/// Helper macro for test assertions with equality
#[macro_export]
macro_rules! test_assert_eq {
    ($left:expr, $right:expr) => {
        match (&$left, &$right) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    return Err($crate::test_helpers::TestError::assertion_failure(
                        format!("assertion failed: `(left == right)`\n  left: `{:?}`,\n right: `{:?}`", 
                                left_val, right_val)
                    ));
                }
            }
        }
    };
    ($left:expr, $right:expr, $message:expr $(, $arg:expr)*) => {
        match (&$left, &$right) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    return Err($crate::test_helpers::TestError::assertion_failure(
                        format!($message $(, $arg)*)
                    ));
                }
            }
        }
    };
}

/// Helper macro for checking feature value types
#[macro_export]
macro_rules! test_feature_value {
    ($feature_value:expr, $expected_type:ident($expected_val:expr)) => {
        match $feature_value {
            processing::model::FeatureValue::$expected_type(val) => {
                if *val != $expected_val {
                    return Err($crate::test_helpers::TestError::assertion_failure(
                        format!("Feature value mismatch: expected {}, got {}", $expected_val, val)
                    ));
                }
            },
            other => {
                return Err($crate::test_helpers::TestError::feature_type_mismatch(
                    stringify!($expected_type),
                    format!("{:?}", other)
                ));
            }
        }
    };
    ($feature_value:expr, $expected_type:ident) => {
        match $feature_value {
            processing::model::FeatureValue::$expected_type(_) => {},
            other => {
                return Err($crate::test_helpers::TestError::feature_type_mismatch(
                    stringify!($expected_type),
                    format!("{:?}", other)
                ));
            }
        }
    };
}

/// Utility functions for common test operations
pub mod test_utils {
    use super::*;
    
    /// Safe HTTP request builder that returns TestError
    pub fn build_request(method: &str, uri: &str, body: Option<String>) -> TestResult<http::Request<String>> {
        let mut builder = http::Request::builder()
            .uri(uri)
            .method(method);
            
        if body.is_some() {
            builder = builder.header("Content-Type", "application/json");
        }
        
        let request = builder
            .body(body.unwrap_or_default())
            .map_err(TestError::from)?;
            
        Ok(request)
    }
    
    /// Safe JSON serialization that returns TestError
    pub fn serialize_json<T: serde::Serialize>(value: &T) -> TestResult<String> {
        serde_json::to_string(value).map_err(TestError::from)
    }
    
    /// Safe response status check
    pub fn check_status_code(actual: http::StatusCode, expected: http::StatusCode) -> TestResult<()> {
        if actual != expected {
            return Err(TestError::assertion_failure(
                format!("Status code mismatch: expected {}, got {}", expected, actual)
            ));
        }
        Ok(())
    }
    
    /// Safe error containment check
    pub fn check_error_contains(error: &dyn std::error::Error, expected_substring: &str) -> TestResult<()> {
        let error_msg = error.to_string();
        if !error_msg.contains(expected_substring) {
            return Err(TestError::assertion_failure(
                format!("Error message '{}' does not contain '{}'", error_msg, expected_substring)
            ));
        }
        Ok(())
    }
} 