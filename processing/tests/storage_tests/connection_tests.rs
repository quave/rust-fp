use processing::storage::CommonStorage;
use std::error::Error;
use tracing::debug;

use super::setup::*;
use common::test_helpers::truncate_processing_tables;
use serial_test::serial;

#[tokio::test]
#[serial]
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

#[tokio::test]
#[serial]
async fn test_find_connected_transactions_api() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (pool, storage) = get_test_storage().await?;
    
    // Clean up any existing test data
    truncate_processing_tables(&pool).await?;
    
    // Create transactions to test with
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
    common::test_helpers::truncate_connection_test_tables(&pool).await?;
    
    // SECTION 2: Test max_depth parameter
    // Set up transactions in a chain
    let transaction_data = vec![
        (1, "2024-01-01"),
        (2, "2024-01-02"),
        (3, "2024-01-03"),
        (4, "2024-01-04"),
        (5, "2024-01-05"),
        (6, "2024-01-06"),
        (7, "2024-01-07"),
        (8, "2024-01-08"),
        (9, "2024-01-09"),
        (10, "2024-01-10"),
    ];
    common::test_helpers::create_test_transactions_batch(&pool, &transaction_data).await?;
    
    // Create match nodes for a chain
    let node_data = vec![
        (1, "link.1-2", "chain1", 100, 0),
        (2, "link.2-3", "chain2", 100, 0),
        (3, "link.3-4", "chain3", 100, 0),
        (4, "link.4-5", "chain4", 100, 0),
        (5, "link.5-6", "chain5", 100, 0),
        (6, "link.6-7", "chain6", 100, 0),
        (7, "link.7-8", "chain7", 100, 0),
        (8, "link.8-9", "chain8", 100, 0),
        (9, "link.9-10", "chain9", 100, 0),
    ];
    common::test_helpers::create_match_nodes_batch(&pool, &node_data).await?;
    
    // Connect transactions in a chain (one by one to avoid conflicts)
    let connections = vec![
        (1, 1), (1, 2), (2, 2), (2, 3), (3, 3), (3, 4), (4, 4), (4, 5),
        (5, 5), (5, 6), (6, 6), (6, 7), (7, 7), (7, 8), (8, 8), (8, 9), (9, 9), (9, 10)
    ];
    for (node_id, transaction_id) in connections {
        common::test_helpers::link_transaction_to_match_node(&pool, node_id, transaction_id).await?;
    }
    
    // Test depth-limited query (max_depth=2)
    let depth_limited = storage.find_connected_transactions(1, Some(2), None, None, None, None).await?;
    
    // Should find exactly 2 transactions due to depth limit
    assert_eq!(depth_limited.len(), 2, "Should find 2 transactions with depth limit of 2");
    
    // Verify all returned transactions have depth 2 or less
    for tx in &depth_limited {
        assert!(tx.depth <= 2, "Transaction should have depth 2 or less");
        assert_ne!(tx.transaction_id, 4, "Transaction with id=4 should not be included (it's at depth 3)");
    }
    
    // Test with higher depth limit (max_depth=3) to make sure we can get deeper connections
    
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_get_direct_connections() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (pool, storage) = get_test_storage().await?;
    
    // Clean up any existing test data
    truncate_processing_tables(&pool).await?;
    
    // Create transactions
    let tx1 = create_test_transaction(&storage).await?;
    let tx2 = create_test_transaction(&storage).await?;
    let tx3 = create_test_transaction(&storage).await?;

    // Setup matcher nodes
    let email_node_id = common::test_helpers::create_test_match_node(
        &pool, "email", "test@example.com", 90, 80
    ).await?;

    let phone_node_id = common::test_helpers::create_test_match_node(
        &pool, "phone", "1234567890", 85, 75
    ).await?;

    // Connect transactions via match nodes using the correct node IDs
    common::test_helpers::link_transaction_to_match_node(&pool, email_node_id, tx1).await?;
    common::test_helpers::link_transaction_to_match_node(&pool, email_node_id, tx2).await?;
    common::test_helpers::link_transaction_to_match_node(&pool, phone_node_id, tx1).await?;
    common::test_helpers::link_transaction_to_match_node(&pool, phone_node_id, tx3).await?;

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
    let tx4 = create_test_transaction(&storage).await?;
    let connections = storage.get_direct_connections(tx4).await?;
    assert_eq!(connections.len(), 0, "Unconnected transaction should have no connections");
    
    Ok(())
} 