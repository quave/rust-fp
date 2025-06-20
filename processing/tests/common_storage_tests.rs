use chrono::Utc;
use processing::{
    model::{Feature, FeatureValue, ModelId, ScorerResult},
    storage::{CommonStorage, ProdCommonStorage},
    test_utils::{setup_test_environment, create_test_pool, create_test_common_storage},
};
use serde_json::{json, Value};
use sqlx::PgPool;
use std::error::Error;
use tracing::debug;

// Ensure DB migrations are applied before tests
#[tokio::test(flavor = "multi_thread")]
#[serial_test::serial]
async fn setup() -> Result<(), Box<dyn Error + Send + Sync>> {
    setup_test_environment().await?;
    Ok(())
}

// Reset tables to ensure clean state
async fn reset_test_tables(pool: &PgPool) -> Result<(), Box<dyn Error + Send + Sync>> {
    sqlx::query("TRUNCATE TABLE match_node_transactions CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("TRUNCATE TABLE match_node CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("TRUNCATE TABLE transactions CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("TRUNCATE TABLE features CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("TRUNCATE TABLE triggered_rules CASCADE")
        .execute(pool)
        .await?;
    
    Ok(())
}

// Helper to create a DB pool and storage, plus reset tables
async fn get_test_storage() -> Result<(PgPool, ProdCommonStorage), Box<dyn Error + Send + Sync>> {
    let pool = create_test_pool().await?;
    let storage = create_test_common_storage().await?;
    
    // Reset tables to ensure clean state
    reset_test_tables(&pool).await?;
    
    Ok((pool, storage))
}

// Helper function to save raw features for testing
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

    // Use dynamic SQL query instead of macro to avoid compile-time checking
    sqlx::query(
        r#"
        INSERT INTO features (
            transaction_id, schema_version_major, schema_version_minor, payload
        ) VALUES ($1, $2, $3, $4)
        "#
    )
    .bind(transaction_id)
    .bind(1i32)
    .bind(0i32)
    .bind(features_json)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

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
async fn test_save_scores() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (pool, storage) = get_test_storage().await?;
    
    // Create a transaction
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
async fn test_save_scores_with_empty_list() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (pool, storage) = get_test_storage().await?;
    let transaction_id = storage.insert_transaction().await?;

    // Save empty scores list
    storage.save_scores(transaction_id, &[]).await?;

    // Verify no scores were saved
    let saved_scores = sqlx::query!(
        r#"
        SELECT COUNT(*) as count
        FROM triggered_rules
        WHERE transaction_id = $1
        "#,
        transaction_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(saved_scores.count.unwrap_or(0), 0);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[serial_test::serial]
async fn test_save_scores_with_duplicate_names() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (pool, storage) = get_test_storage().await?;
    let transaction_id = storage.insert_transaction().await?;

    // Create scores with duplicate names
    let scores = vec![
        ScorerResult {
            name: "duplicate_score".to_string(),
            score: 50,
        },
        ScorerResult {
            name: "duplicate_score".to_string(),
            score: 75,
        },
    ];

    // Save scores
    storage.save_scores(transaction_id, &scores).await?;

    // Verify both scores were saved
    let saved_scores = sqlx::query!(
        r#"
        SELECT rule_name, rule_score
        FROM triggered_rules
        WHERE transaction_id = $1
        ORDER BY rule_score
        "#,
        transaction_id
    )
    .fetch_all(&pool)
    .await?;

    assert_eq!(saved_scores.len(), 2);
    assert_eq!(saved_scores[0].rule_name, "duplicate_score");
    assert_eq!(saved_scores[0].rule_score, 50);
    assert_eq!(saved_scores[1].rule_name, "duplicate_score");
    assert_eq!(saved_scores[1].rule_score, 75);

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

#[tokio::test(flavor = "multi_thread")]
#[serial_test::serial]
async fn test_transaction_matching_sql_function() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (pool, _) = get_test_storage().await?;
    
    // Execute the SQL test function that runs all the matching test cases
    let test_results = sqlx::query!(
        r#"
        SELECT * FROM test_find_connected_transactions()
        "#
    )
    .fetch_all(&pool)
    .await?;
    
    // Verify that we got the expected number of test cases
    assert_eq!(test_results.len(), 9, "Expected 9 test cases to be run");
    
    // Check that all tests passed
    let failed_tests: Vec<_> = test_results
        .iter()
        .filter(|row| !row.pass_fail.unwrap_or(false))
        .collect();
    
    // Log the test results for debugging
    for result in &test_results {
        debug!(
            "Test case {}: {:?} - Expected: {}, Actual: {}, Passed: {}",
            result.case_number.unwrap_or(0),
            result.description.as_deref().unwrap_or(""),
            result.expected.unwrap_or(0),
            result.actual.unwrap_or(0),
            result.pass_fail.unwrap_or(false)
        );
    }
    
    // Assert that all tests passed
    assert!(
        failed_tests.is_empty(),
        "Found {} failed tests: {:?}",
        failed_tests.len(),
        failed_tests
            .iter()
            .map(|row| format!(
                "Test {} ({}): expected={}, actual={}", 
                row.case_number.unwrap_or(0), 
                row.description.as_deref().unwrap_or(""),
                row.expected.unwrap_or(0), 
                row.actual.unwrap_or(0)
            ))
            .collect::<Vec<_>>()
            .join(", ")
    );
    
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[serial_test::serial]
async fn test_find_connected_transactions_api() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (pool, storage) = get_test_storage().await?;
    
    // SECTION 1: Basic connectivity test
    // Set up initial test transactions
    sqlx::query!(
        r#"
        INSERT INTO transactions (id, created_at) VALUES 
        (1, '2024-01-01'),
        (2, '2024-01-02'),
        (3, '2024-01-03'),
        (4, '2024-01-04'),
        (5, '2024-01-05')
        "#
    )
    .execute(&pool)
    .await?;
    
    // Create test match node
    sqlx::query!(
        r#"
        INSERT INTO match_node (id, matcher, value, confidence, importance) VALUES
        (1, 'customer.email', 'test@test.com', 100, 0),
        (2, 'customer.phone', '+1234567890', 80, 0)
        "#
    )
    .execute(&pool)
    .await?;
    
    // Connect all transactions to email match node
    // Connect transactions one by one
    sqlx::query!(
        r#"INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (1, 1)"#
    )
    .execute(&pool)
    .await?;
    
    sqlx::query!(
        r#"INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (1, 2)"#
    )
    .execute(&pool)
    .await?;
    
    sqlx::query!(
        r#"INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (1, 3)"#
    )
    .execute(&pool)
    .await?;
    
    sqlx::query!(
        r#"INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (1, 4)"#
    )
    .execute(&pool)
    .await?;
    
    sqlx::query!(
        r#"INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (1, 5)"#
    )
    .execute(&pool)
    .await?;
    
    // Connect some transactions to phone match node
    sqlx::query!(
        r#"INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (2, 1)"#
    )
    .execute(&pool)
    .await?;
    
    sqlx::query!(
        r#"INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (2, 3)"#
    )
    .execute(&pool)
    .await?;
    
    sqlx::query!(
        r#"INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (2, 5)"#
    )
    .execute(&pool)
    .await?;
    
    // Test finding all connected transactions
    let all_connected = storage.find_connected_transactions(1, None, None, None, None, None).await?;
    
    // Should find all 5 transactions
    assert_eq!(all_connected.len(), 5, "Should find all 5 connected transactions");
    
    // Check that the confidence and path information is included
    let tx1 = all_connected.iter().find(|tx| tx.transaction_id == 1).unwrap();
    assert_eq!(tx1.confidence, 100, "Should have confidence of 100");
    assert_eq!(tx1.path_matchers.len(), 1, "Should have 1 path matcher for root node");
    
    // Clean up before next test
    sqlx::query!("TRUNCATE TABLE match_node_transactions CASCADE").execute(&pool).await?;
    sqlx::query!("TRUNCATE TABLE match_node CASCADE").execute(&pool).await?;
    sqlx::query!("TRUNCATE TABLE transactions CASCADE").execute(&pool).await?;
    
    // SECTION 2: Test max_depth parameter
    // Set up transactions in a chain
    sqlx::query!(
        r#"
        INSERT INTO transactions (id, created_at) VALUES 
        (1, '2024-01-01'),
        (2, '2024-01-02'),
        (3, '2024-01-03'),
        (4, '2024-01-04'),
        (5, '2024-01-05'),
        (6, '2024-01-06'),
        (7, '2024-01-07'),
        (8, '2024-01-08'),
        (9, '2024-01-09'),
        (10, '2024-01-10')
        "#
    )
    .execute(&pool)
    .await?;
    
    // Create match nodes for a chain
    sqlx::query!(
        r#"
        INSERT INTO match_node (id, matcher, value, confidence, importance) VALUES
        (1, 'link.1-2', 'chain1', 100, 0),
        (2, 'link.2-3', 'chain2', 100, 0),
        (3, 'link.3-4', 'chain3', 100, 0),
        (4, 'link.4-5', 'chain4', 100, 0),
        (5, 'link.5-6', 'chain5', 100, 0),
        (6, 'link.6-7', 'chain6', 100, 0),
        (7, 'link.7-8', 'chain7', 100, 0),
        (8, 'link.8-9', 'chain8', 100, 0),
        (9, 'link.9-10', 'chain9', 100, 0)
        "#
    )
    .execute(&pool)
    .await?;
    
    // Connect transactions in a chain (one by one to avoid conflicts)
    sqlx::query!(r#"INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (1, 1)"#).execute(&pool).await?;
    sqlx::query!(r#"INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (1, 2)"#).execute(&pool).await?;
    
    sqlx::query!(r#"INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (2, 2)"#).execute(&pool).await?;
    sqlx::query!(r#"INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (2, 3)"#).execute(&pool).await?;
    
    sqlx::query!(r#"INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (3, 3)"#).execute(&pool).await?;
    sqlx::query!(r#"INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (3, 4)"#).execute(&pool).await?;
    
    sqlx::query!(r#"INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (4, 4)"#).execute(&pool).await?;
    sqlx::query!(r#"INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (4, 5)"#).execute(&pool).await?;
    
    sqlx::query!(r#"INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (5, 5)"#).execute(&pool).await?;
    sqlx::query!(r#"INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (5, 6)"#).execute(&pool).await?;
    
    sqlx::query!(r#"INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (6, 6)"#).execute(&pool).await?;
    sqlx::query!(r#"INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (6, 7)"#).execute(&pool).await?;
    
    sqlx::query!(r#"INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (7, 7)"#).execute(&pool).await?;
    sqlx::query!(r#"INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (7, 8)"#).execute(&pool).await?;
    
    sqlx::query!(r#"INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (8, 8)"#).execute(&pool).await?;
    sqlx::query!(r#"INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (8, 9)"#).execute(&pool).await?;
    
    sqlx::query!(r#"INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (9, 9)"#).execute(&pool).await?;
    sqlx::query!(r#"INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (9, 10)"#).execute(&pool).await?;
    
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[serial_test::serial]
async fn test_save_matching_fields() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (pool, storage) = get_test_storage().await?;
    
    // Create transactions to test with
    let transaction_id1 = storage.insert_transaction().await?;
    let transaction_id2 = storage.insert_transaction().await?;
    
    // Create custom matching config using HashMap - define values explicitly for test
    let mut matcher_configs = std::collections::HashMap::new();
    matcher_configs.insert("customer.email".to_string(), (85, 75));
    matcher_configs.insert("billing.payment_details".to_string(), (90, 80));
    matcher_configs.insert("test.matcher".to_string(), (95, 85));
    matcher_configs.insert("ip.address".to_string(), (70, 60));
    
    // Create test matching fields
    let matching_fields1 = vec![
        processing::model::MatchingField {
            matcher: "customer.email".to_string(),
            value: "test@example.com".to_string(),
        },
        processing::model::MatchingField {
            matcher: "billing.payment_details".to_string(),
            value: "4111-1111-1111-1111".to_string(),
        },
        processing::model::MatchingField {
            matcher: "test.matcher".to_string(),
            value: "test-value".to_string(),
        }
    ];
    
    // Save matching fields for first transaction
    storage.save_matching_fields(transaction_id1, &matching_fields1, &matcher_configs).await?;
    
    // Query database to verify nodes were created
    let saved_nodes = sqlx::query!(
        r#"
        SELECT matcher, value, confidence, importance
        FROM match_node
        ORDER BY matcher
        "#
    )
    .fetch_all(&pool)
    .await?;
    
    // Verify 3 nodes were created with correct values
    assert_eq!(saved_nodes.len(), 3, "Expected 3 match nodes to be created");
    
    // Verify the billing.payment_details node using our explicit test values
    assert_eq!(saved_nodes[0].matcher, "billing.payment_details");
    assert_eq!(saved_nodes[0].value, "4111-1111-1111-1111");
    assert_eq!(saved_nodes[0].confidence, 90); // From our test config
    assert_eq!(saved_nodes[0].importance, 80);  // From our test config
    
    // Verify the customer.email node using our explicit test values
    assert_eq!(saved_nodes[1].matcher, "customer.email");
    assert_eq!(saved_nodes[1].value, "test@example.com");
    assert_eq!(saved_nodes[1].confidence, 85); // From our test config
    assert_eq!(saved_nodes[1].importance, 75);  // From our test config
    
    // Verify the custom matcher node using our explicit test values
    assert_eq!(saved_nodes[2].matcher, "test.matcher");
    assert_eq!(saved_nodes[2].value, "test-value");
    assert_eq!(saved_nodes[2].confidence, 95);  // From our test config
    assert_eq!(saved_nodes[2].importance, 85);  // From our test config
    
    // Verify node-transaction connections
    let node_transactions = sqlx::query!(
        r#"
        SELECT COUNT(*) as count
        FROM match_node_transactions
        WHERE transaction_id = $1
        "#,
        transaction_id1
    )
    .fetch_one(&pool)
    .await?;
    
    assert_eq!(node_transactions.count.unwrap(), 3, "Expected 3 node-transaction connections");
    
    // Test matching fields for second transaction with some overlap
    let matching_fields2 = vec![
        processing::model::MatchingField {
            matcher: "customer.email".to_string(),
            value: "test@example.com".to_string(),  // Same as transaction 1
        },
        processing::model::MatchingField {
            matcher: "ip.address".to_string(),
            value: "192.168.1.1".to_string(),       // New matcher/value
        }
    ];
    
    // Save matching fields for second transaction
    storage.save_matching_fields(transaction_id2, &matching_fields2, &matcher_configs).await?;
    
    // Verify nodes after second save
    let updated_nodes = sqlx::query!(
        r#"
        SELECT matcher, value
        FROM match_node
        ORDER BY matcher
        "#
    )
    .fetch_all(&pool)
    .await?;
    
    // Should now have 4 nodes (the 3 from before plus the new ip.address)
    assert_eq!(updated_nodes.len(), 4, "Expected 4 match nodes after second save");
    
    // Verify connections between transactions
    let common_node_id = sqlx::query!(
        r#"
        SELECT id
        FROM match_node
        WHERE matcher = 'customer.email' AND value = 'test@example.com'
        "#
    )
    .fetch_one(&pool)
    .await?
    .id;
    
    // Check that both transactions are connected to the same node
    let connected_transactions = sqlx::query!(
        r#"
        SELECT transaction_id
        FROM match_node_transactions
        WHERE node_id = $1
        ORDER BY transaction_id
        "#,
        common_node_id
    )
    .fetch_all(&pool)
    .await?;
    
    assert_eq!(connected_transactions.len(), 2, "Expected both transactions to be connected to the common node");
    assert_eq!(connected_transactions[0].transaction_id, transaction_id1);
    assert_eq!(connected_transactions[1].transaction_id, transaction_id2);
    
    // Test idempotency - saving the same fields again should not create duplicates
    storage.save_matching_fields(transaction_id1, &matching_fields1, &matcher_configs).await?;
    
    // Check that node count hasn't changed
    let final_node_count = sqlx::query!(
        r#"
        SELECT COUNT(*) as count
        FROM match_node
        "#
    )
    .fetch_one(&pool)
    .await?;
    
    assert_eq!(final_node_count.count.unwrap(), 4, "Node count should not change after duplicate save");
    
    // Check that node-transaction connections haven't duplicated
    let final_connection_count = sqlx::query!(
        r#"
        SELECT COUNT(*) as count
        FROM match_node_transactions
        "#
    )
    .fetch_one(&pool)
    .await?;
    
    assert_eq!(final_connection_count.count.unwrap(), 5, "Expected 5 total node-transaction connections");
    
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[serial_test::serial]
async fn test_save_matching_fields_empty() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (pool, storage) = get_test_storage().await?;
    
    // Create a transaction
    let transaction_id = storage.insert_transaction().await?;
    
    // Create empty matching fields list
    let empty_fields: Vec<processing::model::MatchingField> = vec![];
    
    // Create empty config HashMap
    let matcher_configs = std::collections::HashMap::new();
    
    // Save empty matching fields
    storage.save_matching_fields(transaction_id, &empty_fields, &matcher_configs).await?;
    
    // Verify no nodes were created
    let node_count = sqlx::query!(
        r#"
        SELECT COUNT(*) as count
        FROM match_node
        "#
    )
    .fetch_one(&pool)
    .await?;
    
    assert_eq!(node_count.count.unwrap(), 0, "No nodes should be created with empty fields");
    
    // Verify no connections were created
    let connection_count = sqlx::query!(
        r#"
        SELECT COUNT(*) as count
        FROM match_node_transactions
        "#
    )
    .fetch_one(&pool)
    .await?;
    
    assert_eq!(connection_count.count.unwrap(), 0, "No connections should be created with empty fields");
    
    Ok(())
}

// Test for get_direct_connections
#[tokio::test(flavor = "multi_thread")]
#[serial_test::serial]
async fn test_get_direct_connections() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (pool, storage) = get_test_storage().await?;

    // Create three transactions
    let tx1 = storage.insert_transaction().await?;
    let tx2 = storage.insert_transaction().await?;
    let tx3 = storage.insert_transaction().await?;

    // Setup matcher nodes
    let email_node_id = sqlx::query!(
        r#"
        INSERT INTO match_node (matcher, value, confidence, importance)
        VALUES ('email', 'test@example.com', 90, 80)
        RETURNING id
        "#
    )
    .fetch_one(&pool)
    .await?
    .id;

    let phone_node_id = sqlx::query!(
        r#"
        INSERT INTO match_node (matcher, value, confidence, importance)
        VALUES ('phone', '1234567890', 85, 75)
        RETURNING id
        "#
    )
    .fetch_one(&pool)
    .await?
    .id;

    // Connect transactions via match nodes using the correct node IDs
    sqlx::query!(
        r#"INSERT INTO match_node_transactions (node_id, transaction_id) VALUES ($1, $2)"#, 
        email_node_id, tx1
    )
    .execute(&pool)
    .await?;
    
    sqlx::query!(
        r#"INSERT INTO match_node_transactions (node_id, transaction_id) VALUES ($1, $2)"#, 
        email_node_id, tx2
    )
    .execute(&pool)
    .await?;
    
    sqlx::query!(
        r#"INSERT INTO match_node_transactions (node_id, transaction_id) VALUES ($1, $2)"#, 
        phone_node_id, tx1
    )
    .execute(&pool)
    .await?;
    
    sqlx::query!(
        r#"INSERT INTO match_node_transactions (node_id, transaction_id) VALUES ($1, $2)"#, 
        phone_node_id, tx3
    )
    .execute(&pool)
    .await?;

    // Get direct connections for tx1
    let connections = storage.get_direct_connections(tx1).await?;
    
    // Should have two connections (to tx2 via email and to tx3 via phone)
    assert_eq!(connections.len(), 2);
    
    // Verify connection details
    let mut contains_tx2_email = false;
    let mut contains_tx3_phone = false;
    
    for conn in connections {
        if conn.transaction_id == tx2 && conn.matcher == "email" {
            contains_tx2_email = true;
            assert_eq!(conn.confidence, 90);
            assert_eq!(conn.importance, 80);
        } else if conn.transaction_id == tx3 && conn.matcher == "phone" {
            contains_tx3_phone = true;
            assert_eq!(conn.confidence, 85);
            assert_eq!(conn.importance, 75);
        }
    }
    
    assert!(contains_tx2_email, "Should contain a connection to tx2 via email");
    assert!(contains_tx3_phone, "Should contain a connection to tx3 via phone");
    
    // Get direct connections for tx2 (only connected to tx1 via email)
    let connections = storage.get_direct_connections(tx2).await?;
    assert_eq!(connections.len(), 1);
    assert_eq!(connections[0].transaction_id, tx1);
    assert_eq!(connections[0].matcher, "email");
    
    // Get direct connections for an unconnected transaction
    let tx4 = storage.insert_transaction().await?;
    let connections = storage.get_direct_connections(tx4).await?;
    assert_eq!(connections.len(), 0, "Unconnected transaction should have no connections");
    
    Ok(())
}