use processing::{
    model::MatchingField,
    storage::CommonStorage,
};
use std::error::Error;

use super::setup::get_test_storage;

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
        MatchingField {
            matcher: "customer.email".to_string(),
            value: "test@example.com".to_string(),
        },
        MatchingField {
            matcher: "billing.payment_details".to_string(),
            value: "4111-1111-1111-1111".to_string(),
        },
        MatchingField {
            matcher: "test.matcher".to_string(),
            value: "test-value".to_string(),
        }
    ];
    
    // Save matching fields for first transaction
    storage.save_matching_fields(transaction_id1, &matching_fields1).await?;
    
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
    
    // Verify the billing.payment_details node
    assert_eq!(saved_nodes[0].matcher, "billing.payment_details");
    assert_eq!(saved_nodes[0].value, "4111-1111-1111-1111");
    assert_eq!(saved_nodes[0].confidence, 100); // Default from ProdCommonStorage
    assert_eq!(saved_nodes[0].importance, 80);  // Default from ProdCommonStorage
    
    // Verify the customer.email node
    assert_eq!(saved_nodes[1].matcher, "customer.email");
    assert_eq!(saved_nodes[1].value, "test@example.com");
    assert_eq!(saved_nodes[1].confidence, 100); // Default from ProdCommonStorage
    assert_eq!(saved_nodes[1].importance, 90);  // Default from ProdCommonStorage
    
    // Verify the custom matcher node - uses default values since it's not in the config
    assert_eq!(saved_nodes[2].matcher, "test.matcher");
    assert_eq!(saved_nodes[2].value, "test-value");
    assert_eq!(saved_nodes[2].confidence, 80);  // Default confidence
    assert_eq!(saved_nodes[2].importance, 50);  // Default importance
    
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
        MatchingField {
            matcher: "customer.email".to_string(),
            value: "test@example.com".to_string(),  // Same as transaction 1
        },
        MatchingField {
            matcher: "ip.address".to_string(),
            value: "192.168.1.1".to_string(),       // New matcher/value
        }
    ];
    
    // Save matching fields for second transaction
    storage.save_matching_fields(transaction_id2, &matching_fields2).await?;
    
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
    storage.save_matching_fields(transaction_id1, &matching_fields1).await?;
    
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
    let empty_fields: Vec<MatchingField> = vec![];
    
    // Save empty matching fields
    storage.save_matching_fields(transaction_id, &empty_fields).await?;
    
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