use chrono::TimeZone;
use chrono::Utc;
use processing::model::{FraudLevel, LabelSource};
use std::error::Error;

use super::setup::get_test_storage;

// =============================================================================
// STORAGE LAYER TESTS (Integration tests with real database)
// =============================================================================

// Test saving and retrieving a label
#[tokio::test]
#[serial_test::serial]
async fn test_save_and_get_label() -> Result<(), Box<dyn Error + Send + Sync>> {
    let storage = get_test_storage().await?;

    // Create test label
    let test_label = entities::label::Model {
        id: 0, // Will be assigned by database
        fraud_level: FraudLevel::Fraud,
        fraud_category: "Test Fraud".to_string(),
        label_source: LabelSource::Manual,
        labeled_by: "test_user".to_string(),
        created_at: Utc::now().naive_utc(),
    };

    // Save label via SeaORM directly (trait no longer exposes save_label)
    use processing::model::sea_orm_storage_model as entities;
    use sea_orm::{ActiveModelTrait, Set};
    let db = &storage.db;
    let label_am = entities::label::ActiveModel {
        id: sea_orm::NotSet,
        fraud_level: Set(test_label.fraud_level),
        fraud_category: Set(test_label.fraud_category.clone()),
        label_source: Set(test_label.label_source),
        labeled_by: Set(test_label.labeled_by.clone()),
        created_at: Set(test_label.created_at),
    };
    let label_model = label_am.insert(db).await?;
    let label_id = label_model.id;

    // Verify label_id is non-zero
    assert!(label_id > 0, "Expected non-zero label ID");

    // Retrieve label via SeaORM directly
    use sea_orm::EntityTrait;
    let retrieved_label = entities::label::Entity::find_by_id(label_id)
        .one(db)
        .await?
        .unwrap();

    // Verify label fields
    assert_eq!(retrieved_label.id, label_id);
    assert_eq!(retrieved_label.fraud_level, FraudLevel::Fraud);
    assert_eq!(retrieved_label.fraud_category, "Test Fraud");
    assert_eq!(retrieved_label.label_source, LabelSource::Manual);
    assert_eq!(retrieved_label.labeled_by, "test_user");

    // Created_at should be within a small time window of the original
    let time_diff = (Utc.from_utc_datetime(&retrieved_label.created_at)
        - Utc.from_utc_datetime(&test_label.created_at))
    .num_seconds()
    .unsigned_abs() as i64;
    assert!(
        time_diff < 10,
        "Created_at timestamp differs by more than 10 seconds"
    );

    Ok(())
}

// Test saving a label with different fraud levels
#[tokio::test]
#[serial_test::serial]
async fn test_save_label_with_different_fraud_levels() -> Result<(), Box<dyn Error + Send + Sync>> {
    let storage = get_test_storage().await?;

    // Test each fraud level
    let fraud_levels = vec![
        FraudLevel::Fraud,
        FraudLevel::NoFraud,
        FraudLevel::BlockedAutomatically,
        FraudLevel::AccountTakeover,
        FraudLevel::NotCreditWorthy,
    ];

    for fraud_level in fraud_levels {
        // Clone the fraud_level for use in multiple places
        let fraud_level_clone = fraud_level.clone();

        let test_label = entities::label::Model {
            id: 0,
            fraud_level,
            fraud_category: format!("Test {:?}", fraud_level_clone),
            label_source: LabelSource::Manual,
            labeled_by: "test_user".to_string(),
            created_at: Utc::now().naive_utc(),
        };

        // Save label
        use processing::model::sea_orm_storage_model as entities;
        use sea_orm::{ActiveModelTrait, Set};
        let db = &storage.db;
        let am = entities::label::ActiveModel {
            id: sea_orm::NotSet,
            fraud_level: Set(test_label.fraud_level),
            fraud_category: Set(test_label.fraud_category.clone()),
            label_source: Set(test_label.label_source),
            labeled_by: Set(test_label.labeled_by.clone()),
            created_at: Set(test_label.created_at),
        };
        let model = am.insert(db).await?;
        let label_id = model.id;

        // Retrieve label via SeaORM
        use sea_orm::EntityTrait;
        let retrieved_label = entities::label::Entity::find_by_id(label_id)
            .one(db)
            .await?
            .unwrap();

        // Verify fraud level is stored correctly
        assert_eq!(retrieved_label.fraud_level, test_label.fraud_level);
    }

    Ok(())
}

// Test saving a label with different label sources
#[tokio::test]
#[serial_test::serial]
async fn test_save_label_with_different_sources() -> Result<(), Box<dyn Error + Send + Sync>> {
    let storage = get_test_storage().await?;

    // Test each label source
    let label_sources = vec![LabelSource::Manual, LabelSource::Api];

    for label_source in label_sources {
        let test_label = entities::label::Model {
            id: 0,
            fraud_level: FraudLevel::Fraud,
            fraud_category: "Test Category".to_string(),
            label_source,
            labeled_by: "test_user".to_string(),
            created_at: Utc::now().naive_utc(),
        };

        // Save label via SeaORM
        use processing::model::sea_orm_storage_model as entities;
        use sea_orm::{ActiveModelTrait, Set};
        let db = &storage.db;
        let am = entities::label::ActiveModel {
            id: sea_orm::NotSet,
            fraud_level: Set(test_label.fraud_level),
            fraud_category: Set(test_label.fraud_category.clone()),
            label_source: Set(test_label.label_source),
            labeled_by: Set(test_label.labeled_by.clone()),
            created_at: Set(test_label.created_at),
        };
        let model = am.insert(db).await?;
        let label_id = model.id;

        // Retrieve label via SeaORM
        use sea_orm::EntityTrait;
        let retrieved_label = entities::label::Entity::find_by_id(label_id)
            .one(db)
            .await?
            .unwrap();

        // Verify label source is stored correctly
        assert_eq!(retrieved_label.label_source, test_label.label_source);
    }

    Ok(())
}
