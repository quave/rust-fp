use chrono::Utc;
use processing::{
    model::{FeatureValue, ModelId, Feature},
    storage::{CommonStorage},
};
use common::test_helpers::truncate_processing_tables;
use serde_json::json;
use std::error::Error;

use super::setup::{get_test_storage, save_raw_features};
use common::test_helpers::create_test_transaction;

// Basic feature storage tests
#[tokio::test]
#[serial_test::serial]
async fn test_save_and_get_features() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (pool, storage) = get_test_storage().await?;
    
    // Clean up any existing test data
    truncate_processing_tables(&pool).await?;
    
    // Create a transaction
    let transaction_id = create_test_transaction(&pool).await?;
    
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
    
    // First create the row with simple_features (using Some)
    let initial_simple_features = vec![
        Feature {
            name: "amount".to_string(),
            value: Box::new(FeatureValue::Double(50.0)),
        },
    ];
    
    storage
        .save_features(transaction_id, &Some(&initial_simple_features), &features)
        .await
        .map_err(|e| "save_features initial ".to_string() + &e.to_string())?;
    
    // Now update with None simple_features (this calls UPDATE)
    let updated_features = vec![
        Feature {
            name: "amount".to_string(),
            value: Box::new(FeatureValue::Double(100.0)),
        },
        Feature {
            name: "is_high_value".to_string(),
            value: Box::new(FeatureValue::Bool(true)),
        },
    ];
    
    storage
        .save_features(transaction_id, &None, &updated_features)
        .await
        .map_err(|e| "save_features update ".to_string() + &e.to_string())?;
    
    // Retrieve features
    let (simple_features, graph_features) = 
        storage
            .get_features(transaction_id)
            .await
            .map_err(|e| "get_features ".to_string() + &e.to_string())?;
    
    // The simple_features should be from the initial save
    assert!(simple_features.is_some());
    let simple_features = simple_features.unwrap();
    assert_eq!(simple_features.len(), 1);
    assert_eq!(simple_features[0].name, "amount");
    
    // The graph_features should be from the update
    assert_eq!(graph_features.len(), 2);
    assert_eq!(graph_features[0].name, "amount");
    assert_eq!(graph_features[1].name, "is_high_value");
    
    // Verify updated values
    match *graph_features[0].value {
        FeatureValue::Double(amount) => assert_eq!(amount, 100.0),
        _ => panic!("Expected Double value"),
    }
    
    match *graph_features[1].value {
        FeatureValue::Bool(is_high_value) => assert!(is_high_value),
        _ => panic!("Expected Bool value"),
    }
    
    Ok(())
}

#[tokio::test]
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

#[tokio::test]
#[serial_test::serial]
async fn test_save_features_with_complex_values() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (pool, storage) = get_test_storage().await?;
    
    // Clean up any existing test data
    truncate_processing_tables(&pool).await?;
    
    // Create a transaction
    let transaction_id = create_test_transaction(&pool).await?;
    let now = Utc::now();
    
    // First create the row with simple_features
    let initial_simple_features = vec![
        Feature {
            name: "amount".to_string(),
            value: Box::new(FeatureValue::Double(50.0)),
        },
    ];
    
    let initial_graph_features = vec![
        Feature {
            name: "is_high_value".to_string(),
            value: Box::new(FeatureValue::Bool(false)),
        },
    ];
    
    storage.save_features(transaction_id, &Some(&initial_simple_features), &initial_graph_features).await?;
    
    // Now update with None simple_features and complex graph features
    let complex_features = vec![
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
    
    // Save complex features (this calls UPDATE)
    storage.save_features(transaction_id, &None, &complex_features).await?;
    
    // Retrieve features
    let (simple_features, graph_features) = storage.get_features(transaction_id).await?;
    
    // The simple_features should still be from the initial save
    assert!(simple_features.is_some());
    let simple_features = simple_features.unwrap();
    assert_eq!(simple_features.len(), 1);
    assert_eq!(simple_features[0].name, "amount");
    
    // The graph_features should be the complex ones we updated
    assert_eq!(graph_features.len(), 3);
    
    // Verify amounts
    match graph_features[0].value.as_ref() {
        FeatureValue::DoubleList(amounts) => assert_eq!(*amounts, vec![100.0, 200.0, 300.0]),
        _ => panic!("Expected DoubleList value"),
    }
    
    // Verify datetime
    match graph_features[1].value.as_ref() {
        FeatureValue::DateTime(dt) => assert_eq!(dt.timestamp(), now.timestamp()),
        _ => panic!("Expected DateTime value"),
    }
    
    // Verify categories
    match graph_features[2].value.as_ref() {
        FeatureValue::StringList(categories) => assert_eq!(*categories, vec!["electronics".to_string(), "clothing".to_string()]),
        _ => panic!("Expected StringList value"),
    }
    
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn test_save_features_with_invalid_array_types() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (pool, storage) = get_test_storage().await?;
    
    // Clean up any existing test data
    truncate_processing_tables(&pool).await?;
    
    let transaction_id = create_test_transaction(&pool).await?;

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

#[tokio::test]
#[serial_test::serial]
async fn test_save_features_with_invalid_schema() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (pool, storage) = get_test_storage().await?;
    let transaction_id = create_test_transaction(&pool).await?;

    // Create features with invalid schema
    let invalid_features = vec![
        Feature {
            name: "invalid_feature".to_string(), // Not in schema enum
            value: Box::new(FeatureValue::Int(42)),
        },
    ];

    // Try to save invalid features
    let result = storage.save_features(transaction_id, &None, &invalid_features).await;
    assert!(result.is_err());
    let err_str = result.unwrap_err().to_string();
    assert!(err_str.contains("validation failed"), "Expected validation error, got: {}", err_str);

    Ok(())
} 