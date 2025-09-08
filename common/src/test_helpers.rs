/// Shared Test Helpers for Cross-Crate Use
/// 
/// This module provides centralized test utilities that can be used across
/// both the `processing` and `ecom` crates to avoid code duplication.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};



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

pub mod mocks {
    use std::collections::VecDeque;
    use std::sync::{Mutex, Arc};
    use std::sync::atomic::{AtomicBool, Ordering};
    use crate::test_helpers::*;
    use crate::test_assert_eq;

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
        pub async fn enqueue(&self, item: u64) -> TestResult<()> {
            if self.should_fail {
                return Err(TestError::mock_failure("Mock queue failure"));
            }
            self.queue.lock().unwrap().push_back(item);
            Ok(())
        }

        /// Get the next item from the queue
        pub async fn fetch_next(&self) -> TestResult<Option<u64>> {
            if self.should_fail {
                return Err(TestError::mock_failure("Mock queue failure"));
            }
            
            if let Some(result) = self.fetch_next_result {
                return Ok(Some(result));
            }
            
            Ok(self.queue.lock().unwrap().pop_front())
        }

        /// Mark an item as processed (mock implementation)
        pub async fn mark_processed(&self, _item: u64) -> TestResult<()> {
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



    // =============================================================================
    // HELPER FUNCTIONS (previously TestDataFactory methods)
    // =============================================================================

    pub fn create_feature(name: &str, value: i32) -> TestFeature {
        TestFeature {
            name: name.to_string(),
            int_value: Some(value),
            string_value: None,
        }
    }

    pub fn create_string_feature(name: &str, value: &str) -> TestFeature {
        TestFeature {
            name: name.to_string(),
            int_value: None,
            string_value: Some(value.to_string()),
        }
    }

    pub fn create_matching_field(matcher: &str, value: &str) -> TestMatchingField {
        TestMatchingField {
            matcher: matcher.to_string(),
            value: value.to_string(),
        }
    }

    pub fn create_scorer_result(name: &str, score: i32) -> TestScorerResult {
        TestScorerResult {
            name: name.to_string(),
            score,
        }
    }

    pub fn create_transaction(id: u64) -> TestTransaction {
        TestTransaction {
            id,
            features: vec![
                create_feature("test_feature", 42),
                create_string_feature("test_string", "test_value"),
            ],
            matching_fields: vec![
                create_matching_field("test_matcher", "test_value"),
            ],
        }
    }

    pub fn create_high_value_transaction(id: u64) -> TestTransaction {
        TestTransaction {
            id,
            features: vec![
                create_feature("value", 10000),
                create_string_feature("type", "high_value"),
            ],
            matching_fields: vec![
                create_matching_field("value_matcher", "high"),
            ],
        }
    }

    pub fn create_low_value_transaction(id: u64) -> TestTransaction {
        TestTransaction {
            id,
            features: vec![
                create_feature("value", 100),
                create_string_feature("type", "low_value"),
            ],
            matching_fields: vec![
                create_matching_field("value_matcher", "low"),
            ],
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
            create_transaction(id)
        }

        pub fn high_value(id: u64) -> Self {
            create_high_value_transaction(id)
        }

        pub fn low_value(id: u64) -> Self {
            create_low_value_transaction(id)
        }

        pub fn with_features(id: u64, features: Vec<TestFeature>) -> Self {
            Self {
                id,
                features,
                matching_fields: vec![create_matching_field("default", "test")],
            }
        }

        pub fn with_matching_fields(id: u64, matching_fields: Vec<TestMatchingField>) -> Self {
            Self {
                id,
                features: vec![create_feature("default", 1)],
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
            Self::new(vec![create_scorer_result("high_value", 100)])
        }

        pub fn low_value_scorer() -> Self {
            Self::new(vec![create_scorer_result("low_value", 10)])
        }

        pub fn was_called(&self) -> bool {
            self.called.load(Ordering::Relaxed)
        }

        pub async fn score(&self, features: Vec<TestFeature>) -> TestResult<Vec<TestScorerResult>> {
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

    /// Simple mock storage for basic testing scenarios
    /// For complex storage testing, use trait-specific mocks with mockall
    #[derive(Debug, Clone)]
    pub struct MockStorage {
        pub transaction_id: u64,
        pub features: Vec<TestFeature>,
    }

    impl MockStorage {
        pub fn new(transaction_id: u64) -> Self {
            Self {
                transaction_id,
                features: vec![create_feature("default", 1)],
            }
        }

        pub fn with_features(transaction_id: u64, features: Vec<TestFeature>) -> Self {
            Self {
                transaction_id,
                features,
            }
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

        pub async fn get_processible(&self, tx_id: u64) -> TestResult<TestTransaction> {
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

pub use mocks::*;

// =============================================================================
// PROCESSING-SPECIFIC MOCKS AND ADAPTERS
// =============================================================================

/// Processing-specific mock implementations that adapt to actual processing types
/// This module provides mocks that implement the actual traits used in the processing crate
// Removed empty processing_mocks module - YAGNI violation eliminated

mod sqlx_helpers {
    use super::*;
    use sqlx::PgPool;
    use std::path::PathBuf;
    use std::error::Error;
    use serde_json::Value;

    /// Create a database connection pool for testing
    pub async fn create_test_pool() -> Result<PgPool, Box<dyn Error + Send + Sync>> {
        let database_url = get_test_database_url();
        let pool = PgPool::connect(&database_url).await?;
        Ok(pool)
    }

    /// Global test setup function that should be called before running any tests
    pub async fn setup_test_environment() -> Result<(), Box<dyn Error + Send + Sync>> {
        let pool = create_test_pool().await?;
        initialize_test_schema(&pool).await?;
        Ok(())
    }

    /// Initialize database schema for testing using SQLx migrations
    pub async fn initialize_test_schema(pool: &PgPool) -> Result<(), Box<dyn Error + Send + Sync>> {
        let workspace_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
        
        let migrations_dir = workspace_dir.join("migrations");
        
        if migrations_dir.exists() {
            println!("Running migrations from: {:?}", migrations_dir);
            sqlx::migrate::Migrator::new(migrations_dir)
                .await?
                .run(pool)
                .await?;
        }
        
        // Basic connection verification
        sqlx::query("SELECT 1").fetch_optional(pool).await?;
        println!("Database connection verified");
        Ok(())
    }

    /// Generic table truncation with dependency ordering
    pub async fn truncate_tables(pool: &PgPool, tables: &[&str]) -> Result<(), Box<dyn Error + Send + Sync>> {
        for table in tables {
            let query = format!("TRUNCATE TABLE {} RESTART IDENTITY CASCADE", table);
            match sqlx::query(&query).execute(pool).await {
                Ok(_) => println!("Truncated table: {}", table),
                Err(e) => println!("Could not truncate table {}: {} (table may not exist)", table, e),
            }
        }
        Ok(())
    }

    /// Truncate processing tables in dependency order
    pub async fn truncate_processing_tables(pool: &PgPool) -> Result<(), Box<dyn Error + Send + Sync>> {
        let tables = &[
            "order_items", "customers", "billing_data", "transactions", "orders",
            "match_node_transactions", "match_node", "features",
            "channels", "scoring_events", "triggered_rules", "labels"
        ];
        truncate_tables(pool, tables).await
    }

    /// Truncate connection test tables
    pub async fn truncate_connection_test_tables(pool: &PgPool) -> Result<(), Box<dyn Error + Send + Sync>> {
        let tables = &["match_node_transactions", "match_node", "transactions"];
        truncate_tables(pool, tables).await
    }

    /// Create a test model and return its ID  
    pub async fn create_test_model(pool: &PgPool, name: &str) -> Result<i64, Box<dyn Error + Send + Sync>> {
        let row = sqlx::query!("INSERT INTO models (name, features_schema_version_major, features_schema_version_minor) VALUES ($1, 1, 0) RETURNING id", name)
            .fetch_one(pool).await?;
        Ok(row.id)
    }
    
    /// Create a test channel and return its ID
    pub async fn create_test_channel(pool: &PgPool, name: &str, model_id: i64) -> Result<i64, Box<dyn Error + Send + Sync>> {
        let row = sqlx::query!("INSERT INTO channels (name, model_id) VALUES ($1, $2) RETURNING id", name, model_id)
            .fetch_one(pool).await?;
        Ok(row.id)
    }
    
    /// Create a test match node and return its ID
    pub async fn create_test_match_node(pool: &PgPool, matcher: &str, value: &str, confidence: i32, importance: i32) -> Result<i64, Box<dyn Error + Send + Sync>> {
        let row = sqlx::query!("INSERT INTO match_node (matcher, value, confidence, importance) VALUES ($1, $2, $3, $4) RETURNING id", matcher, value, confidence, importance)
            .fetch_one(pool).await?;
        Ok(row.id)
    }

    /// Create a test scoring rule (complex case that needs manual implementation)
    pub async fn create_test_scoring_rule(
        pool: &PgPool, 
        model_id: i64, 
        name: &str, 
        description: &str, 
        rule: Value, 
        score: i32
    ) -> Result<i64, Box<dyn Error + Send + Sync>> {
        let row = sqlx::query!(
            "INSERT INTO scoring_rules (model_id, name, description, rule, score) VALUES ($1, $2, $3, $4, $5) RETURNING id",
            model_id, name, description, rule, score
        ).fetch_one(pool).await?;
        Ok(row.id)
    }

    /// Create a transaction with unique test ID
    pub async fn create_test_transaction(pool: &PgPool) -> Result<i64, Box<dyn Error + Send + Sync>> {
        let unique_id = generate_unique_test_id() as i64;
        sqlx::query!("INSERT INTO transactions (id) VALUES ($1)", unique_id)
            .execute(pool).await?;
        Ok(unique_id)
    }

    /// Generic batch insert for transactions
    pub async fn create_test_transactions_batch(
        pool: &PgPool, 
        transaction_data: &[(i64, &str)]
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        for (id, created_at) in transaction_data {
            // Use raw SQL to avoid datetime type issues
            let query = format!("INSERT INTO transactions (id, created_at) VALUES ($1, '{} 00:00:00')", created_at);
            sqlx::query(&query)
                .bind(id)
                .execute(pool).await?;
        }
        Ok(())
    }

    /// Generic batch insert for match nodes
    pub async fn create_match_nodes_batch(
        pool: &PgPool, 
        nodes: &[(i64, &str, &str, i32, i32)]
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        for (id, matcher, value, confidence, importance) in nodes {
            sqlx::query!(
                "INSERT INTO match_node (id, matcher, value, confidence, importance) VALUES ($1, $2, $3, $4, $5)",
                id, matcher, value, confidence, importance
            ).execute(pool).await?;
        }
        Ok(())
    }

    /// Link a transaction to a match node
    pub async fn link_transaction_to_match_node(pool: &PgPool, node_id: i64, transaction_id: i64) -> Result<(), Box<dyn Error + Send + Sync>> {
        sqlx::query!("INSERT INTO match_node_transactions (node_id, transaction_id) VALUES ($1, $2)", node_id, transaction_id)
            .execute(pool).await?;
        Ok(())
    }

    /// Count match node transactions for a specific transaction
    pub async fn count_match_node_transactions(pool: &PgPool, transaction_id: i64) -> Result<i64, Box<dyn Error + Send + Sync>> {
        let row = sqlx::query!("SELECT COUNT(*) as count FROM match_node_transactions WHERE transaction_id = $1", transaction_id)
            .fetch_one(pool).await?;
        Ok(row.count.unwrap_or(0))
    }

    /// Count triggered rules for a scoring event
    pub async fn count_triggered_rules_for_scoring_event(pool: &PgPool, scoring_event_id: i64) -> Result<i64, Box<dyn Error + Send + Sync>> {
        let row = sqlx::query!("SELECT COUNT(*) as count FROM triggered_rules WHERE scoring_events_id = $1", scoring_event_id)
            .fetch_one(pool).await?;
        Ok(row.count.unwrap_or(0))
    }

    /// Get scoring event by transaction ID
    pub async fn get_scoring_event_by_transaction(pool: &PgPool, transaction_id: i64) -> Result<(i64, i64, i64, i32), Box<dyn Error + Send + Sync>> {
        let row = sqlx::query!("SELECT id, transaction_id, channel_id, total_score FROM scoring_events WHERE transaction_id = $1", transaction_id)
            .fetch_one(pool).await?;
        Ok((row.id, row.transaction_id, row.channel_id, row.total_score))
    }

    /// Get triggered rules for a scoring event
    pub async fn get_triggered_rules_for_scoring_event(pool: &PgPool, scoring_event_id: i64) -> Result<Vec<i64>, Box<dyn Error + Send + Sync>> {
        let rows = sqlx::query!("SELECT rule_id FROM triggered_rules WHERE scoring_events_id = $1", scoring_event_id)
            .fetch_all(pool).await?;
        Ok(rows.into_iter().map(|row| row.rule_id).collect())
    }

    /// Clean up ecom test data for a specific transaction
    pub async fn cleanup_ecom_transaction(pool: &PgPool, transaction_id: i64) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut tx = pool.begin().await?;
        
        // Delete in dependency order
        for query in &[
            "DELETE FROM order_items WHERE order_id IN (SELECT id FROM orders WHERE id = $1)",
            "DELETE FROM customers WHERE order_id IN (SELECT id FROM orders WHERE id = $1)",
            "DELETE FROM billing_data WHERE order_id IN (SELECT id FROM orders WHERE id = $1)",
            "DELETE FROM transactions WHERE id = $1",
            "DELETE FROM orders WHERE id = $1"
        ] {
            sqlx::query(query).bind(transaction_id).execute(&mut *tx).await?;
        }
        
        tx.commit().await?;
        Ok(())
    }

    // Generic getters for common queries
    pub async fn get_all_match_nodes(pool: &PgPool) -> Result<Vec<(String, String, i32, i32)>, Box<dyn Error + Send + Sync>> {
        let rows = sqlx::query!("SELECT matcher, value, confidence, importance FROM match_node")
            .fetch_all(pool).await?;
        Ok(rows.into_iter().map(|row| (row.matcher, row.value, row.confidence, row.importance)).collect())
    }

    pub async fn get_match_node_id(pool: &PgPool, matcher: &str, value: &str) -> Result<i64, Box<dyn Error + Send + Sync>> {
        let row = sqlx::query!("SELECT id FROM match_node WHERE matcher = $1 AND value = $2", matcher, value)
            .fetch_one(pool).await?;
        Ok(row.id)
    }

    pub async fn get_transactions_for_match_node(pool: &PgPool, node_id: i64) -> Result<Vec<i64>, Box<dyn Error + Send + Sync>> {
        let rows = sqlx::query!("SELECT transaction_id FROM match_node_transactions WHERE node_id = $1", node_id)
            .fetch_all(pool).await?;
        Ok(rows.into_iter().map(|row| row.transaction_id).collect())
    }

    pub async fn count_match_nodes(pool: &PgPool) -> Result<i64, Box<dyn Error + Send + Sync>> {
        let row = sqlx::query!("SELECT COUNT(*) as count FROM match_node").fetch_one(pool).await?;
        Ok(row.count.unwrap_or(0))
    }

    pub async fn count_all_match_node_transactions(pool: &PgPool) -> Result<i64, Box<dyn Error + Send + Sync>> {
        let row = sqlx::query!("SELECT COUNT(*) as count FROM match_node_transactions").fetch_one(pool).await?;
        Ok(row.count.unwrap_or(0))
    }

    // Aliases for backward compatibility
    pub async fn create_ecom_test_transaction(pool: &PgPool) -> Result<i64, Box<dyn Error + Send + Sync>> {
        create_test_transaction(pool).await
    }

    pub async fn create_test_transaction_with_unique_id(pool: &PgPool) -> Result<i64, Box<dyn Error + Send + Sync>> {
        create_test_transaction(pool).await
    }
}

// Export sqlx helper functions at the module level
pub use sqlx_helpers::*;

// Export the new sqlx helpers for easy access
pub use sqlx_helpers::{
    create_test_model, create_test_channel, create_test_scoring_rule, 
    create_test_match_node, link_transaction_to_match_node,
    get_scoring_event_by_transaction, get_triggered_rules_for_scoring_event,
    count_triggered_rules_for_scoring_event, cleanup_ecom_transaction,
    truncate_connection_test_tables, create_test_transactions_batch,
    create_match_nodes_batch, get_all_match_nodes, count_match_node_transactions,
    get_match_node_id, get_transactions_for_match_node, count_match_nodes,
    count_all_match_node_transactions, create_ecom_test_transaction,
    create_test_transaction_with_unique_id, create_test_transaction,
    truncate_tables
};

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
    
    #[error("Database error: {source}")]
    DatabaseError { #[from] source: sqlx::Error },
    
    #[error("Serialization error: {source}")]
    SerializationError { #[from] source: serde_json::Error },
    
    #[error("HTTP error: {source}")]
    HttpError { #[from] source: http::Error },
    
    #[error("Feature type mismatch: expected {expected}, got {actual}")]
    FeatureTypeMismatch { expected: String, actual: String },
    
    #[error("Transaction not found: {id}")]
    TransactionNotFound { id: i64 },
    
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