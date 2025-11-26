use crate::mocks::MockCommonStorage;
use chrono::{DateTime, Utc};
use processing::{
    model::{
        Channel, Feature, FeatureValue,
        sea_orm_storage_model::expression_rule::Model as ExpressionRule,
    },
    scorers::{ExpressionBasedScorer, Scorer},
};
use std::sync::Arc;

#[tokio::test]
async fn test_int_feature() {
    // Create test features
    let features = vec![Feature {
        name: "transaction_count".to_string(),
        value: Box::new(FeatureValue::Int(10)),
    }];

    let expressions = vec![ExpressionRule {
        id: 1,
        model_id: 0,
        name: "Transaction Count Score".to_string(),
        description: None,
        rule: "transaction_count > 5".to_string(),
        score: 100,
        created_at: chrono::Utc::now().naive_utc(),
    }];

    let mut storage = MockCommonStorage::new();
    storage.expect_get_channel_by_name().returning(|name| {
        Ok(Some(Channel {
            id: 1,
            name: name.to_string(),
            model_id: 0,
            created_at: chrono::Utc::now().naive_utc(),
        }))
    });
    storage.expect_get_expression_rules().returning({
        let expressions = expressions.clone();
        move |_| Ok(expressions.clone())
    });
    storage
        .expect_save_scores()
        .returning(|_, _, total, triggered| {
            assert_eq!(total, 100);
            assert_eq!(triggered.len(), 1);
            Ok(())
        });

    let scorer = ExpressionBasedScorer::new_init("Basic".to_string(), Arc::new(storage))
        .await
        .unwrap();
    scorer
        .score_and_save_result(42, 77, features)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_double_feature() {
    // Create test features
    let features = vec![Feature {
        name: "amount".to_string(),
        value: Box::new(FeatureValue::Double(120.75)),
    }];

    let expressions = vec![ExpressionRule {
        id: 2,
        model_id: 0,
        name: "High Amount Score".to_string(),
        description: None,
        rule: "amount > 100.0".to_string(),
        score: 100,
        created_at: chrono::Utc::now().naive_utc(),
    }];

    let mut storage = MockCommonStorage::new();
    storage.expect_get_channel_by_name().returning(|name| {
        Ok(Some(Channel {
            id: 1,
            name: name.to_string(),
            model_id: 0,
            created_at: chrono::Utc::now().naive_utc(),
        }))
    });
    storage.expect_get_channel_by_name().returning(|name| {
        Ok(Some(Channel {
            id: 1,
            name: name.to_string(),
            model_id: 0,
            created_at: chrono::Utc::now().naive_utc(),
        }))
    });
    storage.expect_get_expression_rules().returning({
        let expressions = expressions.clone();
        move |_| Ok(expressions.clone())
    });
    storage
        .expect_save_scores()
        .returning(|_, _, total, triggered| {
            assert_eq!(total, 100);
            assert_eq!(triggered.len(), 1);
            Ok(())
        });

    let scorer = ExpressionBasedScorer::new_init("Basic".to_string(), Arc::new(storage))
        .await
        .unwrap();
    scorer
        .score_and_save_result(42, 77, features)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_bool_feature() {
    // Create test features
    let features = vec![Feature {
        name: "is_high_risk".to_string(),
        value: Box::new(FeatureValue::Bool(true)),
    }];

    println!("Created feature: is_high_risk = true");

    let expressions = vec![ExpressionRule {
        id: 3,
        model_id: 0,
        name: "Risk Score".to_string(),
        description: None,
        rule: "is_high_risk".to_string(),
        score: 100,
        created_at: chrono::Utc::now().naive_utc(),
    }];

    let mut storage = MockCommonStorage::new();
    storage.expect_get_channel_by_name().returning(|name| {
        Ok(Some(Channel {
            id: 1,
            name: name.to_string(),
            model_id: 0,
            created_at: chrono::Utc::now().naive_utc(),
        }))
    });
    storage.expect_get_expression_rules().returning({
        let expressions = expressions.clone();
        move |_| Ok(expressions.clone())
    });
    storage
        .expect_save_scores()
        .returning(|_, _, total, triggered| {
            assert_eq!(total, 100);
            assert_eq!(triggered.len(), 1);
            Ok(())
        });

    let scorer = ExpressionBasedScorer::new_init("Basic".to_string(), Arc::new(storage))
        .await
        .unwrap();
    scorer
        .score_and_save_result(42, 77, features)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_string_feature() {
    // Create test features
    let features = vec![Feature {
        name: "country".to_string(),
        value: Box::new(FeatureValue::String("US".to_string())),
    }];

    let expressions = vec![ExpressionRule {
        id: 4,
        model_id: 0,
        name: "Country Risk Score".to_string(),
        description: None,
        rule: "country == \"US\"".to_string(),
        score: 100,
        created_at: chrono::Utc::now().naive_utc(),
    }];

    let mut storage = MockCommonStorage::new();
    storage.expect_get_channel_by_name().returning(|name| {
        Ok(Some(Channel {
            id: 1,
            name: name.to_string(),
            model_id: 0,
            created_at: chrono::Utc::now().naive_utc(),
        }))
    });
    storage.expect_get_expression_rules().returning({
        let expressions = expressions.clone();
        move |_| Ok(expressions.clone())
    });
    storage
        .expect_save_scores()
        .returning(|_, _, total, triggered| {
            assert_eq!(total, 100);
            assert_eq!(triggered.len(), 1);
            Ok(())
        });

    let scorer = ExpressionBasedScorer::new_init("Basic".to_string(), Arc::new(storage))
        .await
        .unwrap();
    scorer
        .score_and_save_result(42, 77, features)
        .await
        .unwrap();
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

    let expressions = vec![
        ExpressionRule {
            id: 5,
            model_id: 0,
            name: "Date Age Score".to_string(),
            description: None,
            rule: "recent_date > created_at".to_string(),
            score: 100,
            created_at: chrono::Utc::now().naive_utc(),
        },
        ExpressionRule {
            id: 6,
            model_id: 0,
            name: "Recent Enough".to_string(),
            description: None,
            rule: "recent_date > 1600000000000".to_string(),
            score: 100,
            created_at: chrono::Utc::now().naive_utc(),
        },
    ];

    let mut storage = MockCommonStorage::new();
    storage.expect_get_channel_by_name().returning(|name| {
        Ok(Some(Channel {
            id: 1,
            name: name.to_string(),
            model_id: 0,
            created_at: chrono::Utc::now().naive_utc(),
        }))
    });
    storage.expect_get_expression_rules().returning({
        let expressions = expressions.clone();
        move |_| Ok(expressions.clone())
    });
    storage
        .expect_save_scores()
        .returning(|_, _, total, triggered| {
            assert_eq!(total, 200);
            assert_eq!(triggered.len(), 2);
            Ok(())
        });

    let scorer = ExpressionBasedScorer::new_init("Basic".to_string(), Arc::new(storage))
        .await
        .unwrap();
    scorer
        .score_and_save_result(42, 77, features)
        .await
        .unwrap();
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

    let expressions = vec![
        ExpressionRule {
            id: 7,
            model_id: 0,
            name: "Many Purchases".to_string(),
            description: None,
            rule: "len(purchase_amounts) > 3".to_string(),
            score: 100,
            created_at: chrono::Utc::now().naive_utc(),
        },
        ExpressionRule {
            id: 8,
            model_id: 0,
            name: "Few Items".to_string(),
            description: None,
            rule: "len(item_counts) < 5".to_string(),
            score: 100,
            created_at: chrono::Utc::now().naive_utc(),
        },
        ExpressionRule {
            id: 9,
            model_id: 0,
            name: "Multiple Categories".to_string(),
            description: None,
            rule: "len(categories) >= 3".to_string(),
            score: 100,
            created_at: chrono::Utc::now().naive_utc(),
        },
        ExpressionRule {
            id: 10,
            model_id: 0,
            name: "Has Flags".to_string(),
            description: None,
            rule: "len(high_value_flags) > 0".to_string(),
            score: 100,
            created_at: chrono::Utc::now().naive_utc(),
        },
    ];

    let mut storage = MockCommonStorage::new();
    storage.expect_get_channel_by_name().returning(|name| {
        Ok(Some(Channel {
            id: 1,
            name: name.to_string(),
            model_id: 0,
            created_at: chrono::Utc::now().naive_utc(),
        }))
    });
    storage.expect_get_expression_rules().returning({
        let expressions = expressions.clone();
        move |_| Ok(expressions.clone())
    });
    storage
        .expect_save_scores()
        .returning(|_, _, total, triggered| {
            assert_eq!(total, 400);
            assert_eq!(triggered.len(), 4);
            Ok(())
        });

    let scorer = ExpressionBasedScorer::new_init("Basic".to_string(), Arc::new(storage))
        .await
        .unwrap();
    scorer
        .score_and_save_result(42, 77, features)
        .await
        .unwrap();
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

    let expressions = vec![
        ExpressionRule {
            id: 11,
            model_id: 0,
            name: "High Amount".to_string(),
            description: None,
            rule: "total_amount > 500.0".to_string(),
            score: 100,
            created_at: chrono::Utc::now().naive_utc(),
        },
        ExpressionRule {
            id: 12,
            model_id: 0,
            name: "Multiple Transactions".to_string(),
            description: None,
            rule: "transaction_count >= 5".to_string(),
            score: 100,
            created_at: chrono::Utc::now().naive_utc(),
        },
        ExpressionRule {
            id: 13,
            model_id: 0,
            name: "New Customer".to_string(),
            description: None,
            rule: "is_new_customer".to_string(),
            score: 100,
            created_at: chrono::Utc::now().naive_utc(),
        },
        ExpressionRule {
            id: 14,
            model_id: 0,
            name: "Credit Card Used".to_string(),
            description: None,
            rule: "payment_method == \"credit_card\"".to_string(),
            score: 100,
            created_at: chrono::Utc::now().naive_utc(),
        },
    ];

    let mut storage = MockCommonStorage::new();
    storage.expect_get_channel_by_name().returning(|name| {
        Ok(Some(Channel {
            id: 1,
            name: name.to_string(),
            model_id: 0,
            created_at: chrono::Utc::now().naive_utc(),
        }))
    });
    storage.expect_get_expression_rules().returning({
        let expressions = expressions.clone();
        move |_| Ok(expressions.clone())
    });
    storage
        .expect_save_scores()
        .returning(|_, _, total, triggered| {
            assert_eq!(total, 400);
            assert_eq!(triggered.len(), 4);
            Ok(())
        });

    let scorer = ExpressionBasedScorer::new_init("Basic".to_string(), Arc::new(storage))
        .await
        .unwrap();
    scorer
        .score_and_save_result(42, 77, features)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_invalid_expressions() {
    // Create a simple feature
    let features = vec![Feature {
        name: "amount".to_string(),
        value: Box::new(FeatureValue::Double(100.0)),
    }];

    let expressions = vec![
        ExpressionRule {
            id: 15,
            model_id: 0,
            name: "Valid Score".to_string(),
            description: None,
            rule: "amount > 50.0".to_string(),
            score: 100,
            created_at: chrono::Utc::now().naive_utc(),
        },
        ExpressionRule {
            id: 16,
            model_id: 0,
            name: "Syntax Error".to_string(),
            description: None,
            rule: "amount * )".to_string(),
            score: 100,
            created_at: chrono::Utc::now().naive_utc(),
        },
        ExpressionRule {
            id: 17,
            model_id: 0,
            name: "Unknown Variable".to_string(),
            description: None,
            rule: "unknown_var > 10".to_string(),
            score: 100,
            created_at: chrono::Utc::now().naive_utc(),
        },
        ExpressionRule {
            id: 18,
            model_id: 0,
            name: "Type Error".to_string(),
            description: None,
            rule: "amount + \"string\" == 0".to_string(),
            score: 100,
            created_at: chrono::Utc::now().naive_utc(),
        },
    ];

    let mut storage = MockCommonStorage::new();
    storage.expect_get_channel_by_name().returning(|name| {
        Ok(Some(Channel {
            id: 1,
            name: name.to_string(),
            model_id: 0,
            created_at: chrono::Utc::now().naive_utc(),
        }))
    });
    storage.expect_get_expression_rules().returning({
        let expressions = expressions.clone();
        move |_| Ok(expressions.clone())
    });
    storage
        .expect_save_scores()
        .returning(|_, _, total, triggered| {
            assert_eq!(total, 100);
            assert_eq!(triggered.len(), 1);
            Ok(())
        });

    let scorer = ExpressionBasedScorer::new_init("Basic".to_string(), Arc::new(storage))
        .await
        .unwrap();
    scorer
        .score_and_save_result(42, 77, features)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_empty_features() {
    // No features
    let empty_features: Vec<Feature> = vec![];

    let expressions = vec![
        ExpressionRule {
            id: 19,
            model_id: 0,
            name: "Always True".to_string(),
            description: None,
            rule: "true".to_string(),
            score: 100,
            created_at: chrono::Utc::now().naive_utc(),
        },
        ExpressionRule {
            id: 20,
            model_id: 0,
            name: "Missing Feature Check".to_string(),
            description: None,
            rule: "missing_feature == 10".to_string(),
            score: 100,
            created_at: chrono::Utc::now().naive_utc(),
        },
    ];

    let mut storage = MockCommonStorage::new();
    storage.expect_get_channel_by_name().returning(|name| {
        Ok(Some(Channel {
            id: 1,
            name: name.to_string(),
            model_id: 0,
            created_at: chrono::Utc::now().naive_utc(),
        }))
    });
    storage.expect_get_expression_rules().returning({
        let expressions = expressions.clone();
        move |_| Ok(expressions.clone())
    });
    storage
        .expect_save_scores()
        .returning(|_, _, total, triggered| {
            assert_eq!(total, 100);
            assert_eq!(triggered.len(), 1);
            Ok(())
        });

    let scorer = ExpressionBasedScorer::new_init("Basic".to_string(), Arc::new(storage))
        .await
        .unwrap();
    scorer
        .score_and_save_result(42, 77, empty_features)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_empty_expressions() {
    // Create some features
    let features = vec![Feature {
        name: "amount".to_string(),
        value: Box::new(FeatureValue::Double(100.0)),
    }];

    let expressions: Vec<ExpressionRule> = vec![];

    let mut storage = MockCommonStorage::new();
    storage.expect_get_channel_by_name().returning(|name| {
        Ok(Some(Channel {
            id: 1,
            name: name.to_string(),
            model_id: 0,
            created_at: chrono::Utc::now().naive_utc(),
        }))
    });
    storage.expect_get_expression_rules().returning({
        let expressions = expressions.clone();
        move |_| Ok(expressions.clone())
    });
    storage
        .expect_save_scores()
        .returning(|_, _, total, triggered| {
            assert_eq!(total, 0);
            assert_eq!(triggered.len(), 0);
            Ok(())
        });

    let scorer = ExpressionBasedScorer::new_init("Basic".to_string(), Arc::new(storage))
        .await
        .unwrap();
    scorer
        .score_and_save_result(42, 77, features)
        .await
        .unwrap();
}
