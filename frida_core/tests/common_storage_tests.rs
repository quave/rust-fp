use chrono::Utc;
use frida_core::{
    model::{Feature, FeatureValue, ModelId, ScorerResult},
    storage::{CommonStorage, ProdCommonStorage},
    test_utils::{setup_test_environment, create_test_pool, create_test_common_storage},
};
use serde_json::{json, Value};
use sqlx::PgPool;
use std::error::Error;
use log::debug;

// Global test setup
#[tokio::test(flavor = "multi_thread")]
async fn setup() -> Result<(), Box<dyn Error + Send + Sync>> {
    setup_test_environment().await
}

async fn get_test_storage() -> Result<(PgPool, ProdCommonStorage), Box<dyn Error + Send + Sync>> {
    let pool = create_test_pool().await?;
    let storage = create_test_common_storage().await?;
    Ok((pool, storage))
}

#[tokio::test]
async fn test_save_and_get_features() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (_, storage) = get_test_storage().await?;
    
    // Create a real transaction
    let transaction_id = storage.insert_transaction().await?;
    
    // Create test features
    let features = vec![
        Feature {
            name: "amount".to_string(),
            value: Box::new(FeatureValue::Double(100.0)),
        },
        Feature {
            name: "is_high_value".to_string(),
            value: Box::new(FeatureValue::Bool(true)),
        },
    ];
    
    // Save features
    storage
        .save_features(transaction_id, &features)
        .await
        .map_err(|e| "save_features ".to_string() + &e.to_string())?;
    
    // Retrieve features
    let retrieved_features = 
        storage
            .get_features(transaction_id)
            .await
            .map_err(|e| "get_features ".to_string() + &e.to_string())?;
    
    // Verify features
    assert_eq!(retrieved_features.len(), 2);
    assert_eq!(retrieved_features[0].name, "amount");
    assert_eq!(retrieved_features[1].name, "is_high_value");
    
    // Verify values
    match *retrieved_features[0].value {
        FeatureValue::Double(amount) => assert_eq!(amount, 100.0),
        _ => panic!("Expected Double value"),
    }
    
    match *retrieved_features[1].value {
        FeatureValue::Bool(is_high_value) => assert!(is_high_value),
        _ => panic!("Expected Bool value"),
    }
    
    Ok(())
}

#[tokio::test]
async fn test_save_scores() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (pool, storage) = get_test_storage().await?;
    
    // Create a real transaction
    let transaction_id = storage.insert_transaction().await?;

    // Create test scores
    let scores = vec![
        ScorerResult {
            name: "fraud_score".to_string(),
            score: 85,
        },
        ScorerResult {
            name: "risk_score".to_string(),
            score: 65,
        },
    ];
    
    // Save scores
    storage.save_scores(transaction_id, &scores).await?;
    
    // Verify scores were saved by querying the database directly
    let saved_scores = sqlx::query!(
        r#"
        SELECT rule_name, rule_score
        FROM triggered_rules
        WHERE transaction_id = $1
        ORDER BY rule_name
        "#,
        transaction_id
    )
    .fetch_all(&pool)
    .await?;
    
    assert_eq!(saved_scores.len(), 2);
    assert_eq!(saved_scores[0].rule_name, "fraud_score");
    assert_eq!(saved_scores[0].rule_score, 85);
    assert_eq!(saved_scores[1].rule_name, "risk_score");
    assert_eq!(saved_scores[1].rule_score, 65);
    
    Ok(())
}

#[tokio::test]
async fn test_get_features_nonexistent() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (_pool, storage) = get_test_storage().await?;
    
    // Try to get features for a nonexistent transaction
    let nonexistent_id: ModelId = 999;
    let result = storage.get_features(nonexistent_id).await;
    
    // Should return a RowNotFound error
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("RowNotFound") || err.to_string().contains("no rows returned"));
    
    Ok(())
}

#[tokio::test]
async fn test_save_features_with_complex_values() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (_, storage) = get_test_storage().await?;
    
    // Create a real transaction
    let transaction_id = storage.insert_transaction().await?;
    let now = Utc::now();
    
    let features = vec![
        Feature {
            name: "amounts".to_string(),
            value: Box::new(FeatureValue::DoubleList(vec![100.0, 200.0, 300.0])),
        },
        Feature {
            name: "created_at".to_string(),
            value: Box::new(FeatureValue::DateTime(now)),
        },
        Feature {
            name: "categories".to_string(),
            value: Box::new(FeatureValue::StringList(vec!["electronics".to_string(), "clothing".to_string()])),
        },
    ];
    
    // Save features
    storage.save_features(transaction_id, &features).await?;
    
    // Retrieve features
    let retrieved_features = storage.get_features(transaction_id).await?;
    
    // Verify features
    assert_eq!(retrieved_features.len(), 3);
    
    // Verify amounts
    match retrieved_features[0].value.as_ref() {
        FeatureValue::DoubleList(amounts) => assert_eq!(amounts, &vec![100.0, 200.0, 300.0]),
        _ => panic!("Expected DoubleList value"),
    }
    
    // Verify datetime
    match retrieved_features[1].value.as_ref() {
        FeatureValue::DateTime(dt) => assert_eq!(dt.timestamp(), now.timestamp()),
        _ => panic!("Expected DateTime value"),
    }
    
    // Verify categories
    match retrieved_features[2].value.as_ref() {
        FeatureValue::StringList(categories) => assert_eq!(categories, &vec!["electronics", "clothing"]),
        _ => panic!("Expected StringList value"),
    }
    
    Ok(())
}

// Helper function to save raw JSON features for testing
async fn save_raw_features(
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

    sqlx::query!(
        r#"
        INSERT INTO features (
            transaction_id, schema_version_major, schema_version_minor, payload
        ) VALUES ($1, $2, $3, $4)
        "#,
        transaction_id,
        1,
        0,
        features_json
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

#[tokio::test]
async fn test_save_features_with_invalid_array_types() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (_, storage) = get_test_storage().await?;
    let transaction_id = storage.insert_transaction().await?;

    // Test case 1: Array with mixed types (should fail)
    let invalid_mixed_array = json!([{
        "name": "categories",
        "type": "string_array",
        "value": ["electronics", 42, true]  // Mixed types in array
    }]);

    let result = save_raw_features(&storage, transaction_id, invalid_mixed_array).await;
    assert!(result.is_err());
    let err_str = result.unwrap_err().to_string();
    assert!(err_str.contains("validation failed"), "Expected validation error, got: {}", err_str);

    // Test case 2: Array of objects where strings are expected
    let invalid_object_array = json!([{
        "name": "categories",
        "type": "string_array",
        "value": [{"name": "electronics"}, {"name": "gadgets"}]  // Objects instead of strings
    }]);

    let result = save_raw_features(&storage, transaction_id, invalid_object_array).await;
    assert!(result.is_err());
    let err_str = result.unwrap_err().to_string();
    assert!(err_str.contains("validation failed"), "Expected validation error, got: {}", err_str);

    // Test case 3: Valid array (should succeed)
    let valid_array = json!([{
        "name": "categories",
        "type": "string_array",
        "value": ["electronics", "gadgets"]
    }]);

    let result = save_raw_features(&storage, transaction_id, valid_array).await;
    assert!(result.is_ok(), "Valid array was rejected: {:?}", result);

    Ok(())
}