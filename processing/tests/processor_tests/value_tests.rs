use processing::model::Processible;
use super::super::mocks::TestTransaction;
use common::{test_assert_eq, test_feature_value, test_assert};
use common::test_helpers::{TestResult, TestError};

#[tokio::test]
async fn test_feature_value_types() -> TestResult {
    // Create a high-value transaction
    let transaction = TestTransaction::high_value();
    
    // Test simple feature extraction
    let simple_features = transaction.extract_simple_features();
    test_assert_eq!(simple_features.len(), 1);
    
    // Verify the feature value type
    let is_high_value_feature = &simple_features[0];
    test_assert_eq!(is_high_value_feature.name, "is_high_value");
    
    // Check that it's a boolean value using the new macro
    test_feature_value!(&*is_high_value_feature.value, Bool(true));
    
    Ok(())
}

#[tokio::test]
async fn test_graph_feature_value_types() -> TestResult {
    // Create a high-value transaction for testing
    let transaction = TestTransaction::high_value();
    
    // Extract graph features
    let empty_connected = Vec::new();
    let empty_direct = Vec::new();
    let graph_features = transaction.extract_graph_features(&empty_connected, &empty_direct);
    
    // Should have multiple features
    test_assert!(graph_features.len() >= 5, "Expected at least 5 graph features, got {}", graph_features.len());
    
    // Test specific feature types
    for feature in &graph_features {
        match feature.name.as_str() {
            "is_high_value" => {
                test_feature_value!(&*feature.value, Bool(true));
            },
            "connected_transaction_count" => {
                test_feature_value!(&*feature.value, Int);
            },
            "direct_connection_count" => {
                test_feature_value!(&*feature.value, Int);
            },
            "amount" => {
                test_feature_value!(&*feature.value, Double);
            },
            "amounts" => {
                test_feature_value!(&*feature.value, DoubleList);
            },
            "categories" => {
                test_feature_value!(&*feature.value, StringList);
            },
            "created_at" => {
                test_feature_value!(&*feature.value, DateTime);
            },
            _ => {}, // Ignore other features
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_low_value_transaction_features() -> TestResult {
    // Create a low-value transaction
    let transaction = TestTransaction::low_value();
    
    // Test simple feature extraction
    let simple_features = transaction.extract_simple_features();
    test_assert_eq!(simple_features.len(), 1);
    
    // Find the is_high_value feature
    let is_high_value_feature = simple_features
        .iter()
        .find(|f| f.name == "is_high_value")
        .ok_or_else(|| TestError::assertion_failure("Should have is_high_value feature"))?;
    
    // Should be false for low-value transaction
    test_feature_value!(&*is_high_value_feature.value, Bool(false));
    
    Ok(())
} 