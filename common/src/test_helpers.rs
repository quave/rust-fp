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

use sqlx::PgPool;
use std::path::PathBuf;
use std::error::Error;

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
        "transactions",
        "match_node_transactions", "match_node", "features",
        "channels", "scoring_events", "triggered_rules", "labels"
    ];
    truncate_tables(pool, tables).await
}

/// Create a transaction with unique test ID
pub async fn create_test_transaction(pool: &PgPool) -> Result<i64, Box<dyn Error + Send + Sync>> {
    let unique_id = generate_unique_test_id() as i64;
    let unique_payload_number = generate_unique_id("TEST123");
    sqlx::query!("INSERT INTO transactions (id, payload_number, payload, schema_version_major, schema_version_minor) VALUES ($1, $2, '{}', 1, 0)", unique_id, unique_payload_number)
        .execute(pool).await?;
    Ok(unique_id)
}
