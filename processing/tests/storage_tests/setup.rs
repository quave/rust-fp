use processing::{
    model::ModelId,
    storage::ProdCommonStorage,
    test_helpers::{setup_test_environment, create_test_pool, create_test_common_storage},
};
use serde_json::Value;
use sqlx::PgPool;
use std::error::Error;
use tracing::debug;

// Ensure DB migrations are applied before tests
#[tokio::test(flavor = "multi_thread")]
#[serial_test::serial]
pub async fn setup() -> Result<(), Box<dyn Error + Send + Sync>> {
    setup_test_environment().await?;
    Ok(())
}

// Reset tables to ensure clean state
pub async fn reset_test_tables(pool: &PgPool) -> Result<(), Box<dyn Error + Send + Sync>> {
    sqlx::query("TRUNCATE TABLE match_node_transactions CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("TRUNCATE TABLE match_node CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("TRUNCATE TABLE transactions CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("TRUNCATE TABLE features CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("TRUNCATE TABLE triggered_rules CASCADE")
        .execute(pool)
        .await?;
    
    Ok(())
}

// Helper to create a DB pool and storage, plus reset tables
pub async fn get_test_storage() -> Result<(PgPool, ProdCommonStorage), Box<dyn Error + Send + Sync>> {
    let pool = create_test_pool().await?;
    let storage = create_test_common_storage().await?;
    
    // Reset tables to ensure clean state
    reset_test_tables(&pool).await?;
    
    Ok((pool, storage))
}

// Helper function to save raw features for testing
pub async fn save_raw_features(
    storage: &ProdCommonStorage,
    transaction_id: ModelId,
    features_json: Value,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    // Validate features against schema
    debug!("Raw features JSON: {}", serde_json::to_string_pretty(&features_json)?);
    let validation_result = jsonschema::validate(&storage.get_features_schema(), &features_json);
    if let Err(errors) = validation_result {
        debug!("Validation error details: {:?}", errors);
        return Err(format!("Feature validation failed: {:?}", errors).into());
    }

    let mut tx = storage.pool.begin().await?;

    // Use dynamic SQL query instead of macro to avoid compile-time checking
    sqlx::query(
        r#"
        INSERT INTO features (
            transaction_id, schema_version_major, schema_version_minor, payload
        ) VALUES ($1, $2, $3, $4)
        "#
    )
    .bind(transaction_id)
    .bind(1i32)
    .bind(0i32)
    .bind(features_json)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
} 