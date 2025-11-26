use chrono::Utc;
use common::test_helpers::setup_test_environment;
use processing::{
    model::{ModelId, ProcessibleSerde, sea_orm_storage_model as entities},
    storage::ProdCommonStorage,
};
use sea_orm::ActiveModelTrait;
use sea_orm::ActiveValue::{NotSet, Set};
use serde_json::Value;
use std::error::Error;
use tokio::sync::OnceCell;
use tracing::debug;

use crate::mocks::TestPayload;

// Global async schema setup: runs only once per test process
static SETUP: OnceCell<()> = OnceCell::const_new();

/// Ensures the test DB schema is initialized only once per test run.
pub async fn ensure_setup() {
    SETUP
        .get_or_init(|| async {
            setup_test_environment()
                .await
                .expect("Failed to setup test environment");
        })
        .await;
}

// Helper to create a storage instance backed by the test database
pub async fn get_test_storage() -> Result<ProdCommonStorage<TestPayload>, Box<dyn Error + Send + Sync>> {
    ensure_setup().await;
    create_test_common_storage::<TestPayload>().await
}

/// Create a common storage instance for testing
async fn create_test_common_storage<P: ProcessibleSerde>()
-> Result<ProdCommonStorage<P>, Box<dyn Error + Send + Sync>> {
    use common::test_helpers::get_test_database_url;
    let database_url = get_test_database_url();
    let storage = ProdCommonStorage::new(&database_url).await?;
    Ok(storage)
}

// Helper function to save raw features for testing
pub async fn save_raw_features<P: ProcessibleSerde>(
    storage: &ProdCommonStorage<P>,
    transaction_id: ModelId,
    features_json: Value,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    ensure_setup().await;
    // Validate features against schema
    debug!(
        "Raw features JSON: {}",
        serde_json::to_string_pretty(&features_json)?
    );
    let validation_result = jsonschema::validate(&storage.get_features_schema(), &features_json);
    if let Err(errors) = validation_result {
        debug!("Validation error details: {:?}", errors);
        return Err(format!("Feature validation failed: {:?}", errors).into());
    }

    entities::feature::ActiveModel {
        id: NotSet,
        transaction_id: Set(transaction_id),
        schema_version_major: Set(1),
        schema_version_minor: Set(0),
        simple_features: Set(None),
        graph_features: Set(features_json),
        created_at: Set(Utc::now().naive_utc()),
    }
    .insert(&storage.db)
    .await?;
    Ok(())
}
