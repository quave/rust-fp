use chrono::Utc;
use frida_core::{
    model::{Feature, FeatureValue, ModelId, ScorerResult},
    storage::{CommonStorage, ProdCommonStorage},
    test_utils::initialize_test_schema,
};
use sqlx::PgPool;
use std::error::Error;

async fn setup_test_db() -> Result<(PgPool, ProdCommonStorage), Box<dyn Error + Send + Sync>> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://frida:frida@localhost:5432/frida_test".to_string());
    
    let pool = PgPool::connect(&database_url).await?;
    let storage = ProdCommonStorage::new(&database_url).await?;
    
    // Initialize schema
    initialize_test_schema(&pool).await?;
    
    Ok((pool, storage))
}

#[tokio::test]
async fn test_save_and_get_features() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (_pool, storage) = setup_test_db().await?;
    
    // Create test features
    let transaction_id: ModelId = 1;
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
    storage.save_features(transaction_id, &features).await.map_err(|e| "save_features ".to_string() + &e.to_string())?;
    
    // Retrieve features
    let retrieved_features = storage.get_features(transaction_id).await.map_err(|e| "get_features ".to_string() + &e.to_string())?;
    
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
    let (_pool, storage) = setup_test_db().await?;
    
    // Create test scores
    let transaction_id: ModelId = 1;
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
    .fetch_all(&_pool)
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
    let (_pool, storage) = setup_test_db().await?;
    
    // Try to get features for a nonexistent transaction
    let nonexistent_id: ModelId = 999;
    let features = storage.get_features(nonexistent_id).await?;
    
    // Should return an empty vector
    assert!(features.is_empty());
    
    Ok(())
}

#[tokio::test]
async fn test_save_features_with_complex_values() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (_pool, storage) = setup_test_db().await?;
    
    let transaction_id: ModelId = 1;
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