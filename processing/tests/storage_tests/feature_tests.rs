use chrono::Utc;
use processing::{
    model::{Feature, FeatureValue, ModelId},
    storage::CommonStorage,
};
use serde_json::json;
use std::error::Error;

use super::setup::{get_test_storage, save_raw_features};

// Basic feature storage tests
#[tokio::test(flavor = "multi_thread")]
#[serial_test::serial]
async fn test_save_and_get_features() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (_, storage) = get_test_storage().await?;
    
    // Create a transaction
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

#[tokio::test(flavor = "multi_thread")]
#[serial_test::serial]
async fn test_get_features_nonexistent() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (_, storage) = get_test_storage().await?;
    
    // Try to get features for a nonexistent transaction
    let nonexistent_id: ModelId = 999;
    let result = storage.get_features(nonexistent_id).await;
    
    // Should return a RowNotFound error
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("RowNotFound") || err.to_string().contains("no rows returned"));
    
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[serial_test::serial]
async fn test_save_features_with_complex_values() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (_, storage) = get_test_storage().await?;
    
    // Create a transaction
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

#[tokio::test(flavor = "multi_thread")]
#[serial_test::serial]
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

#[tokio::test(flavor = "multi_thread")]
#[serial_test::serial]
async fn test_save_features_with_invalid_schema() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (_, storage) = get_test_storage().await?;
    let transaction_id = storage.insert_transaction().await?;

    // Create features with invalid schema
    let invalid_features = vec![
        Feature {
            name: "invalid_feature".to_string(), // Not in schema enum
            value: Box::new(FeatureValue::Int(42)),
        },
    ];

    // Try to save invalid features
    let result = storage.save_features(transaction_id, &invalid_features).await;
    assert!(result.is_err());
    let err_str = result.unwrap_err().to_string();
    assert!(err_str.contains("validation failed"), "Expected validation error, got: {}", err_str);

    Ok(())
} 