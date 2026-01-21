use chrono::Utc;
use common::test_helpers::{create_test_transaction, truncate_processing_tables};
use processing::{
    model::{MatchingField, sea_orm_storage_model as entities},
    storage::CommonStorage,
};
use sea_orm::ActiveModelTrait;
use sea_orm::ActiveValue::{NotSet, Set};
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
};
use serial_test::serial;
use std::error::Error;

use super::setup::get_test_storage;

async fn get_match_node_id(
    db: &DatabaseConnection,
    matcher: &str,
    value: &str,
) -> Result<i64, Box<dyn Error + Send + Sync>> {
    let row = entities::match_node::Entity::find()
        .filter(entities::match_node::Column::Matcher.eq(matcher.to_string()))
        .filter(entities::match_node::Column::Value.eq(value.to_string()))
        .one(db)
        .await?
        .ok_or("match node not found")?;
    Ok(row.id)
}

async fn get_transactions_for_match_node(
    db: &DatabaseConnection,
    node_id: i64,
) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
    let rows = entities::match_node_transactions::Entity::find()
        .filter(entities::match_node_transactions::Column::NodeId.eq(node_id))
        .order_by_asc(entities::match_node_transactions::Column::PayloadNumber)
        .all(db)
        .await?;
    Ok(rows.into_iter().map(|row| row.payload_number).collect())
}

async fn count_all_match_node_transactions(
    db: &DatabaseConnection,
) -> Result<i64, Box<dyn Error + Send + Sync>> {
    let count = entities::match_node_transactions::Entity::find()
        .count(db)
        .await? as i64;
    Ok(count)
}

async fn get_all_match_nodes(
    db: &DatabaseConnection,
) -> Result<Vec<(String, String, i32, i32)>, Box<dyn Error + Send + Sync>> {
    let rows = entities::match_node::Entity::find()
        .order_by_asc(entities::match_node::Column::Id)
        .all(db)
        .await?;
    Ok(rows
        .into_iter()
        .map(|row| (row.matcher, row.value, row.confidence, row.importance))
        .collect())
}

async fn count_match_nodes(db: &DatabaseConnection) -> Result<i64, Box<dyn Error + Send + Sync>> {
    let count = entities::match_node::Entity::find().count(db).await? as i64;
    Ok(count)
}

async fn count_match_node_transactions(
    db: &DatabaseConnection,
    payload_number: &str,
) -> Result<i64, Box<dyn Error + Send + Sync>> {
    let count = entities::match_node_transactions::Entity::find()
        .filter(
            entities::match_node_transactions::Column::PayloadNumber
                .eq(payload_number.to_string()),
        )
        .count(db)
        .await? as i64;
    Ok(count)
}

#[tokio::test]
#[serial]
async fn test_save_matching_fields() -> Result<(), Box<dyn Error + Send + Sync>> {
    let storage = get_test_storage().await?;
    let db = &storage.db;

    // Clean up any existing test data
    truncate_processing_tables(db).await?;

    // Create transactions to test with
    let transaction_id1 = create_test_transaction(&storage.db).await?;
    let transaction_id2 = create_test_transaction(&storage.db).await?;
    let transaction1 = storage.get_transaction(transaction_id1).await?;
    let transaction2 = storage.get_transaction(transaction_id2).await?;

    // Create custom matching config using HashMap - define values explicitly for test
    let mut matcher_configs = std::collections::HashMap::new();
    matcher_configs.insert("customer.email".to_string(), (85, 75));
    matcher_configs.insert("billing.payment_details".to_string(), (90, 80));
    matcher_configs.insert("test.matcher".to_string(), (95, 85));
    matcher_configs.insert("ip.address".to_string(), (70, 60));

    // Create test matching fields
    let matching_fields1 = vec![
        MatchingField::new_simple("customer.email".to_string(), "test@example.com".to_string()),
        MatchingField::new_simple(
            "billing.payment_details".to_string(),
            "4111-1111-1111-1111".to_string(),
        ),
        MatchingField::new_simple("test.matcher".to_string(), "test-value".to_string()),
    ];

    // Save matching fields for first transaction
    storage
        .save_matching_fields(transaction_id1, &matching_fields1)
        .await?;

    // Query database to verify nodes were created
    let saved_nodes = get_all_match_nodes(&storage.db).await?;

    // Verify 3 nodes were created with correct values
    assert_eq!(saved_nodes.len(), 3, "Expected 3 match nodes to be created");

    // Verify nodes were created with correct values (order-independent)

    // Find the billing.payment_details node
    let billing_node = saved_nodes
        .iter()
        .find(|node| node.0 == "billing.payment_details" && node.1 == "4111-1111-1111-1111")
        .expect("billing.payment_details node should exist");
    assert_eq!(billing_node.2, 100); // Default from ProdCommonStorage
    assert_eq!(billing_node.3, 80); // Default from ProdCommonStorage

    // Find the customer.email node
    let email_node = saved_nodes
        .iter()
        .find(|node| node.0 == "customer.email" && node.1 == "test@example.com")
        .expect("customer.email node should exist");
    assert_eq!(email_node.2, 100); // Default from ProdCommonStorage
    assert_eq!(email_node.3, 90); // Default from ProdCommonStorage

    // Find the custom matcher node - uses default values since it's not in the config
    let test_node = saved_nodes
        .iter()
        .find(|node| node.0 == "test.matcher" && node.1 == "test-value")
        .expect("test.matcher node should exist");
    assert_eq!(test_node.2, 80); // Default confidence
    assert_eq!(test_node.3, 50); // Default importance

    // Verify node-transaction connections
    let connection_count =
        count_match_node_transactions(&storage.db, &transaction1.payload_number).await?;

    assert_eq!(
        connection_count, 3,
        "Expected 3 node-transaction connections"
    );

    // Test matching fields for second transaction with some overlap
    let matching_fields2 = vec![
        MatchingField::new_simple("customer.email".to_string(), "test@example.com".to_string()),
        MatchingField::new_simple("ip.address".to_string(), "192.168.1.1".to_string()),
    ];

    // Save matching fields for second transaction
    storage
        .save_matching_fields(transaction_id2, &matching_fields2)
        .await?;

    // Verify nodes after second save
    let updated_nodes = get_all_match_nodes(&storage.db).await?;

    // Should now have 4 nodes (the 3 from before plus the new ip.address)
    assert_eq!(
        updated_nodes.len(),
        4,
        "Expected 4 match nodes after second save"
    );

    // Verify connections between transactions
    let common_node_id =
        get_match_node_id(&storage.db, "customer.email", "test@example.com").await?;

    // Check that both transactions are connected to the same node
    let connected_transactions =
        get_transactions_for_match_node(&storage.db, common_node_id).await?;

    assert_eq!(
        connected_transactions.len(),
        2,
        "Expected both transactions to be connected to the common node"
    );
    assert_eq!(connected_transactions[0], transaction1.payload_number);
    assert_eq!(connected_transactions[1], transaction2.payload_number);

    // Test idempotency - saving the same fields again should not create duplicates
    storage
        .save_matching_fields(transaction_id1, &matching_fields1)
        .await?;

    // Check that node count hasn't changed
    let final_node_count = count_match_nodes(&storage.db).await?;

    assert_eq!(
        final_node_count, 4,
        "Node count should not change after duplicate save"
    );

    // Check that node-transaction connections haven't duplicated
    let final_connection_count = count_all_match_node_transactions(&storage.db).await?;

    assert_eq!(
        final_connection_count, 5,
        "Expected 5 total node-transaction connections"
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_save_matching_fields_empty() -> Result<(), Box<dyn Error + Send + Sync>> {
    let storage = get_test_storage().await?;
    let db = &storage.db;

    // Clean up any existing test data
    truncate_processing_tables(db).await?;

    // Create a transaction
    let transaction_id = create_test_transaction(&storage.db).await?;

    // Create empty matching fields list
    let empty_fields: Vec<MatchingField> = vec![];

    // Save empty matching fields
    storage
        .save_matching_fields(transaction_id, &empty_fields)
        .await?;

    // Verify no nodes were created
    let node_count = count_match_nodes(&storage.db).await?;

    assert_eq!(
        node_count, 0,
        "No nodes should be created with empty fields"
    );

    // Verify no connections were created
    let connection_count = count_all_match_node_transactions(&storage.db).await?;

    assert_eq!(
        connection_count, 0,
        "No connections should be created with empty fields"
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_matching_excludes_same_payload_transactions()
-> Result<(), Box<dyn Error + Send + Sync>> {
    let storage = get_test_storage().await?;
    let db = &storage.db;
    truncate_processing_tables(db).await?;

    let root_transaction_id = create_test_transaction(&storage.db).await?;
    let root_transaction = storage.get_transaction(root_transaction_id).await?;

    let duplicate_model = entities::transaction::ActiveModel {
        id: NotSet,
        payload_number: Set(root_transaction.payload_number.clone()),
        transaction_version: Set(root_transaction.transaction_version + 1),
        is_latest: Set(true),
        payload: Set(root_transaction.payload.clone()),
        schema_version_major: Set(root_transaction.schema_version_major),
        schema_version_minor: Set(root_transaction.schema_version_minor),
        label_id: Set(root_transaction.label_id),
        comment: Set(root_transaction.comment.clone()),
        last_scoring_date: Set(None),
        processing_complete: Set(false),
        created_at: Set(Utc::now().naive_utc()),
        updated_at: Set(Utc::now().naive_utc()),
    };

    let _duplicate_id = duplicate_model.insert(&storage.db).await?.id;

    let match_node = entities::match_node::ActiveModel {
        id: NotSet,
        matcher: Set("customer.email".to_string()),
        value: Set("dup@test.com".to_string()),
        confidence: Set(100),
        importance: Set(0),
    }
    .insert(&storage.db)
    .await?;

    entities::match_node_transactions::ActiveModel {
        node_id: Set(match_node.id),
        payload_number: Set(root_transaction.payload_number.clone()),
        datetime_alpha: Set(None),
        datetime_beta: Set(None),
        long_alpha: Set(None),
        lat_alpha: Set(None),
        long_beta: Set(None),
        lat_beta: Set(None),
        long_gamma: Set(None),
        lat_gamma: Set(None),
        long_delta: Set(None),
        lat_delta: Set(None),
        created_at: Set(Utc::now().naive_utc()),
    }
    .insert(&storage.db)
    .await?;

    let graph_results = storage
        .find_connected_transactions(&root_transaction.payload_number, None, None, None, None)
        .await?;
    assert_eq!(
        graph_results.len(),
        0,
        "Root payload should be excluded from graph results"
    );

    let direct_results = storage.get_direct_connections(&root_transaction.payload_number).await?;
    assert!(
        direct_results.is_empty(),
        "Direct connections must exclude duplicate payload revisions"
    );

    Ok(())
}
