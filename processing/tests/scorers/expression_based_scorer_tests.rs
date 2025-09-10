use chrono::{DateTime, Utc};
use processing::{
    model::{FeatureValue, Feature},
    scorers::{ExpressionBasedScorer, Scorer},
};

#[tokio::test]
async fn test_int_feature() {
    // Create test features
    let features = vec![
        Feature {
            name: "transaction_count".to_string(),
            value: Box::new(FeatureValue::Int(10)),
        },
    ];

    // Create scorer with expressions that produce boolean results
    let expressions = vec![
        ("Transaction Count Score".to_string(), "transaction_count > 5".to_string()),
    ];
    let scorer = ExpressionBasedScorer::new_with_expressions(expressions);

    // Score features
    let results = scorer.score(features).await;

    // Verify results
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Transaction Count Score");
    assert_eq!(results[0].score, 100); // transaction_count > 5 is true, so score is 100
}

#[tokio::test]
async fn test_double_feature() {
    // Create test features
    let features = vec![
        Feature {
            name: "amount".to_string(),
            value: Box::new(FeatureValue::Double(120.75)),
        },
    ];

    // Create scorer with expressions that produce boolean results
    let expressions = vec![
        ("High Amount Score".to_string(), "amount > 100.0".to_string()),
    ];
    let scorer = ExpressionBasedScorer::new_with_expressions(expressions);

    // Score features
    let results = scorer.score(features).await;

    // Verify results
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "High Amount Score");
    assert_eq!(results[0].score, 100); // amount > 100.0 is true, so score is 100
}

#[tokio::test]
async fn test_bool_feature() {
    // Create test features
    let features = vec![
        Feature {
            name: "is_high_risk".to_string(),
            value: Box::new(FeatureValue::Bool(true)),
        },
    ];

    println!("Created feature: is_high_risk = true");

    // Create scorer with expressions - just use the boolean directly
    let expressions = vec![
        ("Risk Score".to_string(), "is_high_risk".to_string()),
    ];
    println!("Using expression: is_high_risk");
    
    let scorer = ExpressionBasedScorer::new_with_expressions(expressions);

    // Score features
    let results = scorer.score(features).await;
    println!("Got results: {:?}", results);

    // Verify results
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Risk Score");
    assert_eq!(results[0].score, 100); // is_high_risk is true, so score is 100
}

#[tokio::test]
async fn test_string_feature() {
    // Create test features
    let features = vec![
        Feature {
            name: "country".to_string(),
            value: Box::new(FeatureValue::String("US".to_string())),
        },
    ];

    // Create scorer with expressions
    let expressions = vec![
        // Simple string equality
        ("Country Risk Score".to_string(), "country == \"US\"".to_string()),
    ];
    let scorer = ExpressionBasedScorer::new_with_expressions(expressions);

    // Score features
    let results = scorer.score(features).await;

    // Verify results
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Country Risk Score");
    assert_eq!(results[0].score, 100); // country == "US" is true, so score is 100
}

#[tokio::test]
async fn test_datetime_feature() {
    // Create two fixed dates for testing (2023-01-01 and 2023-02-01)
    let older_date = DateTime::parse_from_rfc3339("2023-01-01T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    
    let newer_date = DateTime::parse_from_rfc3339("2023-02-01T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    
    // Create test features
    let features = vec![
        Feature {
            name: "created_at".to_string(),
            value: Box::new(FeatureValue::DateTime(older_date)),
        },
        Feature {
            name: "recent_date".to_string(),
            value: Box::new(FeatureValue::DateTime(newer_date)),
        },
    ];

    // Calculate the timestamp difference in milliseconds (for reference)
    let time_diff = newer_date.timestamp_millis() - older_date.timestamp_millis();
    assert!(time_diff > 0, "Newer date should have larger timestamp");
    
    // Create scorer with expressions - all boolean results
    let expressions = vec![
        // Simple boolean comparison
        ("Date Age Score".to_string(), "recent_date > created_at".to_string()),
        
        // Boolean expression using a fixed comparison
        ("Recent Enough".to_string(), "recent_date > 1600000000000".to_string()), // Timestamp comparison
    ];
    let scorer = ExpressionBasedScorer::new_with_expressions(expressions);

    // Score features
    let results = scorer.score(features).await;

    // Verify results
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].name, "Date Age Score");
    assert_eq!(results[0].score, 100); // recent_date > created_at is true, so score is 100
    assert_eq!(results[1].name, "Recent Enough");
    assert_eq!(results[1].score, 100); // recent_date > 1600000000000 should be true
}

#[tokio::test]
async fn test_array_features() {
    // Create test features
    let features = vec![
        Feature {
            name: "purchase_amounts".to_string(),
            value: Box::new(FeatureValue::DoubleList(vec![100.0, 200.0, 300.0, 400.0])),
        },
        Feature {
            name: "item_counts".to_string(),
            value: Box::new(FeatureValue::IntList(vec![1, 2, 3, 4])),
        },
        Feature {
            name: "categories".to_string(),
            value: Box::new(FeatureValue::StringList(vec![
                "electronics".to_string(),
                "clothing".to_string(),
                "jewelry".to_string(),
            ])),
        },
        Feature {
            name: "high_value_flags".to_string(),
            value: Box::new(FeatureValue::BoolList(vec![true, false, true])),
        },
    ];

    // Create scorer with expressions - all boolean results
    let expressions = vec![
        // Test array lengths with boolean comparisons
        ("Many Purchases".to_string(), "len(purchase_amounts) > 3".to_string()),
        ("Few Items".to_string(), "len(item_counts) < 5".to_string()),
        ("Multiple Categories".to_string(), "len(categories) >= 3".to_string()),
        ("Has Flags".to_string(), "len(high_value_flags) > 0".to_string()),
    ];
    
    let scorer = ExpressionBasedScorer::new_with_expressions(expressions);

    // Score features
    let results = scorer.score(features).await;

    // Verify results - boolean expressions
    assert_eq!(results.len(), 4);
    
    // Check boolean results
    let many_purchases = results.iter().find(|r| r.name == "Many Purchases").unwrap();
    assert_eq!(many_purchases.score, 100); // len(purchase_amounts) > 3 is true, len is 4
    
    let few_items = results.iter().find(|r| r.name == "Few Items").unwrap();
    assert_eq!(few_items.score, 100); // len(item_counts) < 5 is true, len is 4
    
    let multiple_categories = results.iter().find(|r| r.name == "Multiple Categories").unwrap();
    assert_eq!(multiple_categories.score, 100); // len(categories) >= 3 is true, len is 3
    
    let has_flags = results.iter().find(|r| r.name == "Has Flags").unwrap();
    assert_eq!(has_flags.score, 100); // len(high_value_flags) > 0 is true, len is 3
}

#[tokio::test]
async fn test_combined_features() {
    // Create test features with multiple types
    let features = vec![
        Feature {
            name: "total_amount".to_string(),
            value: Box::new(FeatureValue::Double(850.0)),
        },
        Feature {
            name: "transaction_count".to_string(),
            value: Box::new(FeatureValue::Int(5)),
        },
        Feature {
            name: "is_new_customer".to_string(),
            value: Box::new(FeatureValue::Bool(true)),
        },
        Feature {
            name: "payment_method".to_string(),
            value: Box::new(FeatureValue::String("credit_card".to_string())),
        },
    ];

    // Create scorer with expressions that all output boolean values
    let expressions = vec![
        // All expressions produce boolean results
        ("High Amount".to_string(), "total_amount > 500.0".to_string()),
        ("Multiple Transactions".to_string(), "transaction_count >= 5".to_string()),
        ("New Customer".to_string(), "is_new_customer".to_string()),
        ("Credit Card Used".to_string(), "payment_method == \"credit_card\"".to_string()),
    ];
    
    let scorer = ExpressionBasedScorer::new_with_expressions(expressions);

    // Score features
    let results = scorer.score(features).await;

    // Verify results
    assert_eq!(results.len(), 4);
    
    // Check boolean results
    let high_amount = results.iter().find(|r| r.name == "High Amount").unwrap();
    assert_eq!(high_amount.score, 100); // total_amount > 500.0 is true
    
    let multiple_transactions = results.iter().find(|r| r.name == "Multiple Transactions").unwrap();
    assert_eq!(multiple_transactions.score, 100); // transaction_count >= 5 is true
    
    let new_customer = results.iter().find(|r| r.name == "New Customer").unwrap();
    assert_eq!(new_customer.score, 100); // is_new_customer is true
    
    let credit_card_used = results.iter().find(|r| r.name == "Credit Card Used").unwrap();
    assert_eq!(credit_card_used.score, 100); // payment_method == "credit_card" is true
}

#[tokio::test]
async fn test_invalid_expressions() {
    // Create a simple feature
    let features = vec![
        Feature {
            name: "amount".to_string(),
            value: Box::new(FeatureValue::Double(100.0)),
        },
    ];

    // Create scorer with expressions - all boolean or invalid
    let expressions = vec![
        // Valid boolean expression
        ("Valid Score".to_string(), "amount > 50.0".to_string()),
        
        // Invalid expressions that won't work
        ("Syntax Error".to_string(), "amount * )".to_string()),
        ("Unknown Variable".to_string(), "unknown_var > 10".to_string()),
        ("Type Error".to_string(), "amount + \"string\" == 0".to_string()),
    ];
    
    let scorer = ExpressionBasedScorer::new_with_expressions(expressions);

    // Score features
    let results = scorer.score(features).await;

    // Only the valid boolean expression should succeed
    assert_eq!(results.len(), 1);
    
    // Check the valid score
    let valid_score = results.iter().find(|r| r.name == "Valid Score").unwrap();
    assert_eq!(valid_score.score, 100); // amount > 50.0 is true
}

#[tokio::test]
async fn test_empty_features() {
    // No features
    let empty_features: Vec<Feature> = vec![];

    // Create scorer with expressions
    let expressions = vec![
        // Static boolean expression
        ("Always True".to_string(), "true".to_string()),
        ("Missing Feature Check".to_string(), "missing_feature == 10".to_string()), // This should fail
    ];
    
    let scorer = ExpressionBasedScorer::new_with_expressions(expressions);

    // Score features
    let results = scorer.score(empty_features).await;

    // Verify results - only the static boolean expression should succeed
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Always True");
    assert_eq!(results[0].score, 100);
}

#[tokio::test]
async fn test_empty_expressions() {
    // Create some features
    let features = vec![
        Feature {
            name: "amount".to_string(),
            value: Box::new(FeatureValue::Double(100.0)),
        },
    ];

    // Create scorer with no expressions
    let empty_expressions: Vec<(String, String)> = vec![];
    let scorer = ExpressionBasedScorer::new_with_expressions(empty_expressions);

    // Score features
    let results = scorer.score(features).await;

    // Verify results - should be empty
    assert_eq!(results.len(), 0);
} 