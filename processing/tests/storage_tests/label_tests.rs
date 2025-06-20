use chrono::Utc;
use processing::{
    model::{Label, FraudLevel, LabelSource},
    storage::CommonStorage,
};
use std::error::Error;

use super::setup::{get_test_storage, get_unique_model_id};

// =============================================================================
// STORAGE LAYER TESTS (Integration tests with real database)
// =============================================================================

// Test saving and retrieving a label
#[tokio::test]
#[serial_test::serial]
async fn test_save_and_get_label() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (_pool, storage) = get_test_storage().await?;
    
    // Create test label
    let test_label = Label {
        id: 0, // Will be assigned by database
        fraud_level: FraudLevel::Fraud,
        fraud_category: "Test Fraud".to_string(),
        label_source: LabelSource::Manual,
        labeled_by: "test_user".to_string(),
        created_at: Utc::now(),
    };
    
    // Save label
    let label_id = storage.save_label(&test_label).await?;
    
    // Verify label_id is non-zero
    assert!(label_id > 0, "Expected non-zero label ID");
    
    // Retrieve label
    let retrieved_label = storage.get_label(label_id).await?;
    
    // Verify label fields
    assert_eq!(retrieved_label.id, label_id);
    assert!(matches!(retrieved_label.fraud_level, FraudLevel::Fraud));
    assert_eq!(retrieved_label.fraud_category, "Test Fraud");
    assert!(matches!(retrieved_label.label_source, LabelSource::Manual));
    assert_eq!(retrieved_label.labeled_by, "test_user");
    
    // Created_at should be within a small time window of the original
    let time_diff = (retrieved_label.created_at - test_label.created_at).num_seconds().abs();
    assert!(time_diff < 10, "Created_at timestamp differs by more than 10 seconds");
    
    Ok(())
}

// Test saving a label with different fraud levels
#[tokio::test]
#[serial_test::serial]
async fn test_save_label_with_different_fraud_levels() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (_pool, storage) = get_test_storage().await?;
    
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
        
        let test_label = Label {
            id: 0,
            fraud_level,
            fraud_category: format!("Test {:?}", fraud_level_clone),
            label_source: LabelSource::Manual,
            labeled_by: "test_user".to_string(),
            created_at: Utc::now(),
        };
        
        // Save label
        let label_id = storage.save_label(&test_label).await?;
        
        // Retrieve label
        let retrieved_label = storage.get_label(label_id).await?;
        
        // Verify fraud level is stored correctly
        assert!(
            std::mem::discriminant(&retrieved_label.fraud_level) == std::mem::discriminant(&test_label.fraud_level),
            "Fraud level not stored correctly: expected {:?}, got {:?}", 
            test_label.fraud_level, 
            retrieved_label.fraud_level
        );
    }
    
    Ok(())
}

// Test saving a label with different label sources
#[tokio::test]
#[serial_test::serial]
async fn test_save_label_with_different_sources() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (_pool, storage) = get_test_storage().await?;
    
    // Test each label source
    let label_sources = vec![
        LabelSource::Manual,
        LabelSource::Api,
    ];
    
    for label_source in label_sources {
        let test_label = Label {
            id: 0,
            fraud_level: FraudLevel::Fraud,
            fraud_category: "Test Category".to_string(),
            label_source,
            labeled_by: "test_user".to_string(),
            created_at: Utc::now(),
        };
        
        // Save label
        let label_id = storage.save_label(&test_label).await?;
        
        // Retrieve label
        let retrieved_label = storage.get_label(label_id).await?;
        
        // Verify label source is stored correctly
        assert!(
            std::mem::discriminant(&retrieved_label.label_source) == std::mem::discriminant(&test_label.label_source),
            "Label source not stored correctly: expected {:?}, got {:?}", 
            test_label.label_source, 
            retrieved_label.label_source
        );
    }
    
    Ok(())
}

// Test updating transaction label
#[tokio::test]
#[serial_test::serial]
async fn test_update_transaction_label() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (pool, storage) = get_test_storage().await?;
    
    let transaction_id = get_unique_model_id();
    
    // Create a transaction
    sqlx::query!(
        r#"
        INSERT INTO transactions (id, created_at) 
        VALUES ($1, '2024-01-01')
        "#,
        transaction_id
    )
    .execute(&pool)
    .await?;
    
    // Create test label
    let test_label = Label {
        id: 0,
        fraud_level: FraudLevel::Fraud,
        fraud_category: "Test Fraud".to_string(),
        label_source: LabelSource::Manual,
        labeled_by: "test_user".to_string(),
        created_at: Utc::now(),
    };
    
    // Save label
    let label_id = storage.save_label(&test_label).await?;
    
    // Update transaction label
    storage.update_transaction_label(transaction_id, label_id).await?;
    
    // Verify label was set correctly
    let row = sqlx::query!(
        r#"
        SELECT label_id FROM transactions
        WHERE id = $1
        "#,
        transaction_id
    )
    .fetch_one(&pool)
    .await?;
    
    assert_eq!(row.label_id, Some(label_id));
    
    Ok(())
}

// Test updating transaction label for non-existent transaction
#[tokio::test]
#[serial_test::serial]
async fn test_update_nonexistent_transaction_label() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (_pool, storage) = get_test_storage().await?;
    
    // Create test label
    let test_label = Label {
        id: 0,
        fraud_level: FraudLevel::Fraud,
        fraud_category: "Test Fraud".to_string(),
        label_source: LabelSource::Manual,
        labeled_by: "test_user".to_string(),
        created_at: Utc::now(),
    };
    
    // Save label
    let label_id = storage.save_label(&test_label).await?;
    
    // Try to update non-existent transaction - this might actually succeed if the implementation
    // doesn't verify transaction existence
    let result = storage.update_transaction_label(999, label_id).await;
    
    // Verify the query execution succeeds even though no rows are affected
    assert!(result.is_ok(), "The update should succeed even for non-existent transactions");
    
    // Verify the transaction wasn't actually labeled by checking the database
    let row = sqlx::query!(
        r#"
        SELECT label_id FROM transactions
        WHERE id = 999
        "#
    )
    .fetch_optional(&_pool)
    .await?;
    
    // The query should return no rows since the transaction doesn't exist
    assert!(row.is_none(), "Non-existent transaction should not be found in the database");
    
    Ok(())
}

// Test updating transaction label multiple times
#[tokio::test]
#[serial_test::serial]
async fn test_update_transaction_label_multiple_times() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (pool, storage) = get_test_storage().await?;
    
    let transaction_id = get_unique_model_id();
    
    // Create a transaction
    sqlx::query!(
        r#"
        INSERT INTO transactions (id, created_at) 
        VALUES ($1, '2024-01-01')
        "#,
        transaction_id
    )
    .execute(&pool)
    .await?;
    
    // Create first label
    let first_label = Label {
        id: 0,
        fraud_level: FraudLevel::Fraud,
        fraud_category: "First Label".to_string(),
        label_source: LabelSource::Manual,
        labeled_by: "test_user".to_string(),
        created_at: Utc::now(),
    };
    
    // Save first label
    let first_label_id = storage.save_label(&first_label).await?;
    
    // Update transaction with first label
    storage.update_transaction_label(transaction_id, first_label_id).await?;
    
    // Verify first label was set
    let row1 = sqlx::query!(
        r#"
        SELECT label_id FROM transactions
        WHERE id = $1
        "#,
        transaction_id
    )
    .fetch_one(&pool)
    .await?;
    
    assert_eq!(row1.label_id, Some(first_label_id));
    
    // Create second label
    let second_label = Label {
        id: 0,
        fraud_level: FraudLevel::NoFraud,
        fraud_category: "Second Label".to_string(),
        label_source: LabelSource::Api,
        labeled_by: "api".to_string(),
        created_at: Utc::now(),
    };
    
    // Save second label
    let second_label_id = storage.save_label(&second_label).await?;
    
    // Update transaction with second label
    storage.update_transaction_label(transaction_id, second_label_id).await?;
    
    // Verify second label replaced first label
    let row2 = sqlx::query!(
        r#"
        SELECT label_id FROM transactions
        WHERE id = $1
        "#,
        transaction_id
    )
    .fetch_one(&pool)
    .await?;
    
    assert_eq!(row2.label_id, Some(second_label_id));
    
    Ok(())
}

// =============================================================================
// BUSINESS LOGIC TESTS (Unit tests with unified mocks)
// =============================================================================

// TODO: Re-implement these tests with centralized MockCommonStorage from common crate
// 
// This section previously contained comprehensive business logic unit tests for 
// transaction labeling functionality including:
// - test_label_transactions_complete_success
// - test_label_transactions_save_label_failure  
// - test_label_transactions_partial_success
// - test_label_transactions_complete_failure
// - test_label_transactions_empty_list
//
// These tests have been temporarily commented out during the test helper 
// centralization effort. When re-implementing, they should use the centralized
// MockCommonStorage from the common crate's test_helpers module. 