use processing::{
    model::ModelId,
    storage::ProdCommonStorage,
};
use common::test_helpers::{setup_test_environment, create_test_pool, generate_unique_test_id};
use serde_json::Value;
use sqlx::PgPool;
use std::error::Error;
use tracing::debug;
use tokio::sync::OnceCell;

/// Convert the centralized test ID to ModelId
pub fn get_unique_model_id() -> ModelId {
    generate_unique_test_id() as ModelId
}

/// Create a transaction with a unique test ID to avoid conflicts
pub async fn create_test_transaction(storage: &ProdCommonStorage) -> Result<ModelId, Box<dyn Error + Send + Sync>> {
    let unique_id = get_unique_model_id();
    
    // Insert transaction with explicit ID to avoid auto-increment conflicts
    sqlx::query!(
        r#"
        INSERT INTO transactions (id, created_at)
        VALUES ($1, NOW())
        "#,
        unique_id
    )
    .execute(&storage.pool)
    .await?;
    
    Ok(unique_id)
}

// Global async schema setup: runs only once per test process
static SETUP: OnceCell<()> = OnceCell::const_new();

/// Ensures the test DB schema is initialized only once per test run.
pub async fn ensure_setup() {
    SETUP.get_or_init(|| async {
        setup_test_environment().await.expect("Failed to setup test environment");
    }).await;
}

// Helper to create a DB pool and storage, plus reset tables
pub async fn get_test_storage() -> Result<(PgPool, ProdCommonStorage), Box<dyn Error + Send + Sync>> {
    ensure_setup().await;
    let pool = create_test_pool().await?;
    let storage = create_test_common_storage().await?;
    // Note: No longer dropping tables here since it interferes with parallel tests
    // The schema is set up once in ensure_setup() and shared across all tests
    Ok((pool, storage))
}

/// Create a common storage instance for testing
async fn create_test_common_storage() -> Result<ProdCommonStorage, Box<dyn Error + Send + Sync>> {
    use common::test_helpers::get_test_database_url;
    let database_url = get_test_database_url();
    let storage = ProdCommonStorage::new(&database_url).await?;
    Ok(storage)
}

// Helper function to save raw features for testing
pub async fn save_raw_features(
    storage: &ProdCommonStorage,
    transaction_id: ModelId,
    features_json: Value,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    ensure_setup().await;
    // Validate features against schema
    debug!("Raw features JSON: {}", serde_json::to_string_pretty(&features_json)?);
    let validation_result = jsonschema::validate(&storage.get_features_schema(), &features_json);
    if let Err(errors) = validation_result {
        debug!("Validation error details: {:?}", errors);
        return Err(format!("Feature validation failed: {:?}", errors).into());
    }

    // Use the production storage method instead of raw SQL
    use processing::model::Feature;
    
    // Convert the JSON to Feature structs - this would be better done through 
    // proper deserialization, but for test purposes we'll use the storage layer directly
    let empty_simple_features: Option<&[Feature]> = None;
    let empty_graph_features: Vec<Feature> = vec![];
    
    // For now, we'll still use the raw approach since we're testing raw feature insertion
    // In a production refactor, this would be replaced with proper Feature objects
    let mut tx = storage.pool.begin().await?;

    sqlx::query(
        r#"
        INSERT INTO features (
            transaction_id, 
            transaction_version,
            schema_version_major, 
            schema_version_minor, 
            simple_features,
            graph_features
        ) VALUES ($1, $2, $3, $4, $5, $6)
        "#
    )
    .bind(transaction_id)
    .bind(1i32)
    .bind(1i32)
    .bind(0i32)
    .bind(None::<Value>) // simple_features as NULL
    .bind(features_json) // graph_features
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
} 