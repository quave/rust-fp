use processing::{
    model::MatchingField,
    storage::CommonStorage,
};
use common::test_helpers::{truncate_processing_tables, create_test_transaction};
use std::error::Error;
use serial_test::serial;

use super::setup::{get_test_storage};

#[tokio::test]
#[serial]
async fn test_save_matching_fields() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (pool, storage) = get_test_storage().await?;
    
    // Clean up any existing test data
    truncate_processing_tables(&pool).await?;
    
    // Create transactions to test with
    let transaction_id1 = create_test_transaction(&pool).await?;
    let transaction_id2 = create_test_transaction(&pool).await?;
    
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
    let saved_nodes = common::test_helpers::get_all_match_nodes(&pool).await?;
    
    // Verify 3 nodes were created with correct values
    assert_eq!(saved_nodes.len(), 3, "Expected 3 match nodes to be created");
    
    // Verify nodes were created with correct values (order-independent)
    
    // Find the billing.payment_details node
    let billing_node = saved_nodes.iter().find(|node| 
        node.0 == "billing.payment_details" && node.1 == "4111-1111-1111-1111"
    ).expect("billing.payment_details node should exist");
    assert_eq!(billing_node.2, 100); // Default from ProdCommonStorage
    assert_eq!(billing_node.3, 80);  // Default from ProdCommonStorage
    
    // Find the customer.email node
    let email_node = saved_nodes.iter().find(|node| 
        node.0 == "customer.email" && node.1 == "test@example.com"
    ).expect("customer.email node should exist");
    assert_eq!(email_node.2, 100); // Default from ProdCommonStorage
    assert_eq!(email_node.3, 90);  // Default from ProdCommonStorage
    
    // Find the custom matcher node - uses default values since it's not in the config
    let test_node = saved_nodes.iter().find(|node| 
        node.0 == "test.matcher" && node.1 == "test-value"
    ).expect("test.matcher node should exist");
    assert_eq!(test_node.2, 80);  // Default confidence
    assert_eq!(test_node.3, 50);  // Default importance
    
    // Verify node-transaction connections
    let connection_count = common::test_helpers::count_match_node_transactions(&pool, transaction_id1 as i64).await?;
    
    assert_eq!(connection_count, 3, "Expected 3 node-transaction connections");
    
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
    let updated_nodes = common::test_helpers::get_all_match_nodes(&pool).await?;
    
    // Should now have 4 nodes (the 3 from before plus the new ip.address)
    assert_eq!(updated_nodes.len(), 4, "Expected 4 match nodes after second save");
    
    // Verify connections between transactions
    let common_node_id = common::test_helpers::get_match_node_id(&pool, "customer.email", "test@example.com").await?;
    
    // Check that both transactions are connected to the same node
    let connected_transactions = common::test_helpers::get_transactions_for_match_node(&pool, common_node_id).await?;
    
    assert_eq!(connected_transactions.len(), 2, "Expected both transactions to be connected to the common node");
    assert_eq!(connected_transactions[0], transaction_id1 as i64);
    assert_eq!(connected_transactions[1], transaction_id2 as i64);
    
    // Test idempotency - saving the same fields again should not create duplicates
    storage.save_matching_fields(transaction_id1, &matching_fields1).await?;
    
    // Check that node count hasn't changed
    let final_node_count = common::test_helpers::count_match_nodes(&pool).await?;
    
    assert_eq!(final_node_count, 4, "Node count should not change after duplicate save");
    
    // Check that node-transaction connections haven't duplicated
    let final_connection_count = common::test_helpers::count_all_match_node_transactions(&pool).await?;
    
    assert_eq!(final_connection_count, 5, "Expected 5 total node-transaction connections");
    
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_save_matching_fields_empty() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (pool, storage) = get_test_storage().await?;
    
    // Clean up any existing test data
    truncate_processing_tables(&pool).await?;
    
    // Create a transaction
    let transaction_id = create_test_transaction(&pool).await?;
    
    // Create empty matching fields list
    let empty_fields: Vec<MatchingField> = vec![];
    
    // Save empty matching fields
    storage.save_matching_fields(transaction_id, &empty_fields).await?;
    
    // Verify no nodes were created
    let node_count = common::test_helpers::count_match_nodes(&pool).await?;
    
    assert_eq!(node_count, 0, "No nodes should be created with empty fields");
    
    // Verify no connections were created
    let connection_count = common::test_helpers::count_all_match_node_transactions(&pool).await?;
    
    assert_eq!(connection_count, 0, "No connections should be created with empty fields");
    
    Ok(())
} 