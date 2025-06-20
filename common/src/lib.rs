pub mod yaml_include;
pub mod config;

/// Common utilities shared across the Frida AI project
/// 
/// This crate provides shared functionality that can be used across different
/// modules of the Frida AI fraud detection system, including:
/// 
/// - Database connection management
/// - Shared test utilities and mocks
/// - Common data structures and utilities
/// - Test data factories and helpers

// Test helpers module - available for both development and test builds
#[cfg(any(test, feature = "test-helpers", feature = "async-sqlx"))]
pub mod test_helpers;

// Re-export commonly used test utilities for easier access
#[cfg(feature = "async-sqlx")]
pub use test_helpers::{
    generate_unique_id, 
    generate_unique_test_id, 
    get_test_database_url,
    setup_test_environment,
    create_test_pool,
    truncate_processing_tables,
    truncate_ecom_tables,
};

// Re-export centralized mocks for testing
#[cfg(feature = "async-sqlx")]
pub use test_helpers::{
    MockQueue,
    TestDataFactory,
    TestFeature,
    TestMatchingField,
    TestScorerResult,
    TestTransaction,
    MockScorer,
    MockStorage,
    MockProcessibleStorage,
};

// Re-export processing-specific mocks when the feature is enabled
#[cfg(feature = "processing-mocks")]
pub use test_helpers::{
    MockCommonStorage,
    MockProcessible,
    MockProcessibleStorage as ProcessingMockProcessibleStorage,
    MockQueueService,
    MockWebTransaction,
    MockWebStorage,
    TestConnectedTransaction,
    TestDirectConnection,
};

/// Documentation for Centralized Test Mocks
/// 
/// The common crate provides a comprehensive set of centralized mocks and test utilities
/// that can be used across different crates in the project. This eliminates code duplication
/// and ensures consistent testing patterns.
/// 
/// ## Basic Usage
/// 
/// Add the common crate to your test dependencies:
/// 
/// ```toml
/// [dev-dependencies]
/// common = { path = "../common", features = ["async-sqlx"] }
/// ```
/// 
/// For processing-specific mocks, also enable the processing-mocks feature:
/// 
/// ```toml
/// [dev-dependencies]
/// common = { path = "../common", features = ["async-sqlx", "processing-mocks"] }
/// ```
/// 
/// ## Test Data Factories
/// 
/// Use the `TestDataFactory` to create consistent test data:
/// 
/// ```rust
/// use common::TestDataFactory;
/// 
/// let feature = TestDataFactory::create_feature("test_feature", 42);
/// let transaction = TestDataFactory::create_transaction(123);
/// let high_value_tx = TestDataFactory::create_high_value_transaction(456);
/// ```
/// 
/// ## Mock Objects
/// 
/// Create mock objects with configurable behavior:
/// 
/// ```rust
/// use common::{MockStorage, MockScorer, TestDataFactory};
/// 
/// // Create a mock storage that tracks calls
/// let storage = MockStorage::new(123);
/// assert!(!storage.was_save_features_called());
/// 
/// // Create a mock scorer with specific results
/// let results = vec![TestDataFactory::create_scorer_result("test", 100)];
/// let scorer = MockScorer::new(results);
/// assert!(!scorer.was_called());
/// ```
/// 
/// ## Processing-Specific Mocks
/// 
/// Use processing-specific mocks that implement the actual traits:
/// 
/// ```rust
/// use common::{MockCommonStorage, MockProcessible, MockQueueService};
/// 
/// let storage = MockCommonStorage::new(123, vec![], vec![]);
/// let processible = MockProcessible::new(123);
/// let queue = MockQueueService::new(Some(123), 123);
/// ```
/// 
/// ## Database Test Utilities
/// 
/// Use centralized database setup and cleanup:
/// 
/// ```rust
/// use common::{setup_test_environment, truncate_processing_tables, create_test_pool};
/// 
/// #[tokio::test]
/// async fn my_test() -> Result<(), Box<dyn std::error::Error>> {
///     setup_test_environment().await?;
///     let pool = create_test_pool().await?;
///     
///     // Your test code here
///     
///     truncate_processing_tables(&pool).await?;
///     Ok(())
/// }
/// ```
pub mod docs {
    //! Documentation and examples for the centralized test utilities
}

