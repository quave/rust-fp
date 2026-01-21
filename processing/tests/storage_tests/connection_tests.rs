use chrono::{NaiveDate, Utc};
use processing::{model::sea_orm_storage_model as entities, storage::CommonStorage};
use std::error::Error;
use tracing::debug;

use sea_orm::ActiveValue::{NotSet, Set};
use sea_orm::{
    ActiveModelTrait, ConnectionTrait, DatabaseBackend, DatabaseConnection, EntityTrait, Statement,
};
use serde_json::json;

use super::setup::*;
use common::test_helpers::{create_test_transaction, truncate_processing_tables, truncate_tables};
use serial_test::serial;

/// Generic batch insert for transactions
async fn create_test_transactions_batch(
    db: &DatabaseConnection,
    transaction_data: &[(i64, &str)],
) -> Result<(), Box<dyn Error + Send + Sync>> {
    for (id, created_at) in transaction_data {
        let created_at_dt = NaiveDate::parse_from_str(created_at, "%Y-%m-%d")?
            .and_hms_opt(0, 0, 0)
            .expect("Invalid time");
        let model = entities::transaction::ActiveModel {
            id: Set(*id),
            payload_number: Set(format!("test_payload_{}", id)),
            transaction_version: Set(1),
            is_latest: Set(true),
            payload: Set(json!({})),
            schema_version_major: Set(1),
            schema_version_minor: Set(0),
            label_id: Set(None),
            comment: Set(None),
            last_scoring_date: Set(None),
            processing_complete: Set(false),
            created_at: Set(created_at_dt),
            updated_at: Set(created_at_dt),
        };
        model.insert(db).await?;
    }
    Ok(())
}

/// Truncate connection test tables
async fn truncate_connection_test_tables(
    db: &DatabaseConnection,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let tables = &["match_node_transactions", "match_node", "transactions"];
    truncate_tables(db, tables).await
}

/// Create a test match node and return its ID
async fn create_test_match_node(
    db: &DatabaseConnection,
    matcher: &str,
    value: &str,
    confidence: i32,
    importance: i32,
) -> Result<i64, Box<dyn Error + Send + Sync>> {
    let row = entities::match_node::ActiveModel {
        id: NotSet,
        matcher: Set(matcher.to_string()),
        value: Set(value.to_string()),
        confidence: Set(confidence),
        importance: Set(importance),
    }
    .insert(db)
    .await?;
    Ok(row.id)
}

async fn create_match_nodes_batch(
    db: &DatabaseConnection,
    nodes: &[(i64, &str, &str, i32, i32)],
) -> Result<(), Box<dyn Error + Send + Sync>> {
    for (id, matcher, value, confidence, importance) in nodes {
        entities::match_node::ActiveModel {
            id: Set(*id),
            matcher: Set((*matcher).to_string()),
            value: Set((*value).to_string()),
            confidence: Set(*confidence),
            importance: Set(*importance),
        }
        .insert(db)
        .await?;
    }
    Ok(())
}

async fn link_transaction_to_match_node(
    db: &DatabaseConnection,
    node_id: i64,
    transaction_id: i64,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let txn = entities::transaction::Entity::find_by_id(transaction_id)
        .one(db)
        .await?
        .ok_or("transaction not found")?;
    entities::match_node_transactions::ActiveModel {
        node_id: Set(node_id),
        payload_number: Set(txn.payload_number),
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
    .insert(db)
    .await?;
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_transaction_matching_sql_function() -> Result<(), Box<dyn Error + Send + Sync>> {
    let storage = get_test_storage().await?;

    // Execute the SQL test function that runs all the matching test cases
    let rows = storage
        .db
        .query_all(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT * FROM test_find_connected_transactions()".to_string(),
        ))
        .await?;

    #[derive(Clone)]
    struct TestRow {
        case_number: Option<i32>,
        description: Option<String>,
        expected: Option<i32>,
        actual: Option<i32>,
        pass_fail: Option<bool>,
    }

    let mut test_results = Vec::with_capacity(rows.len());
    for row in rows {
        test_results.push(TestRow {
            case_number: row.try_get("", "case_number")?,
            description: row.try_get("", "description")?,
            expected: row.try_get("", "expected")?,
            actual: row.try_get("", "actual")?,
            pass_fail: row.try_get("", "pass_fail")?,
        });
    }

    // Verify that we got the expected number of test cases
    assert_eq!(test_results.len(), 12, "Expected 12 test cases to be run");

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
    let storage = get_test_storage().await?;
    let db = &storage.db;

    // Clean up any existing test data
    truncate_processing_tables(&db).await?;

    // Create transactions to test with
    let initial_transactions = vec![
        (1, "2024-01-01"),
        (2, "2024-01-02"),
        (3, "2024-01-03"),
        (4, "2024-01-04"),
        (5, "2024-01-05"),
    ];
    create_test_transactions_batch(&storage.db, &initial_transactions).await?;

    // Create test match nodes
    create_match_nodes_batch(
        &storage.db,
        &[
            (1, "customer.email", "test@test.com", 100, 0),
            (2, "customer.phone", "+1234567890", 80, 0),
        ],
    )
    .await?;

    // Connect all transactions to email match node
    for tx_id in 1..=5 {
        link_transaction_to_match_node(&storage.db, 1, tx_id).await?;
    }

    // Connect some transactions to phone match node
    for tx_id in [1, 3, 5] {
        link_transaction_to_match_node(&storage.db, 2, tx_id).await?;
    }

    // Test finding all connected transactions
    let all_connected = storage
        .find_connected_transactions("1", None, None, None, None)
        .await?;

    // Root is excluded; should find the 4 other transactions
    assert_eq!(
        all_connected.len(),
        4,
        "Should find all 4 connected transactions (excluding root)"
    );

    // Clean up before next test
    truncate_connection_test_tables(&db).await?;

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
    create_test_transactions_batch(&storage.db, &transaction_data).await?;

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
    create_match_nodes_batch(&storage.db, &node_data).await?;

    // Connect transactions in a chain (one by one to avoid conflicts)
    let connections = vec![
        (1, 1),
        (1, 2),
        (2, 2),
        (2, 3),
        (3, 3),
        (3, 4),
        (4, 4),
        (4, 5),
        (5, 5),
        (5, 6),
        (6, 6),
        (6, 7),
        (7, 7),
        (7, 8),
        (8, 8),
        (8, 9),
        (9, 9),
        (9, 10),
    ];
    for (node_id, transaction_id) in connections {
        link_transaction_to_match_node(&storage.db, node_id, transaction_id).await?;
    }

    // Test depth-limited query (max_depth=2)
    let depth_limited = storage
        .find_connected_transactions("1", Some(2), None, None, None)
        .await?;

    // Should find exactly 2 transactions due to depth limit
    assert_eq!(
        depth_limited.len(),
        2,
        "Should find 2 transactions with depth limit of 2"
    );

    // Verify all returned transactions have depth 2 or less
    for tx in &depth_limited {
        assert!(tx.parent_transaction_id == 1, "Transaction should have parent transaction id 1");
        assert_ne!(
            tx.transaction_id, 4,
            "Transaction with id=4 should not be included (it's at depth 3)"
        );
    }

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_get_direct_connections() -> Result<(), Box<dyn Error + Send + Sync>> {
    let storage = get_test_storage().await?;
    // Setup matcher nodes
    let email_node_id =
        create_test_match_node(&storage.db, "email", "test@example.com", 90, 80).await?;

    let phone_node_id = create_test_match_node(&storage.db, "phone", "1234567890", 85, 75).await?;

    // Connect transactions via match nodes using the correct node IDs
    link_transaction_to_match_node(&storage.db, email_node_id, 1).await?;
    link_transaction_to_match_node(&storage.db, email_node_id, 2).await?;
    link_transaction_to_match_node(&storage.db, phone_node_id, 1).await?;
    link_transaction_to_match_node(&storage.db, phone_node_id, 3).await?;

    // Get direct connections for tx1
    let connections = storage.get_direct_connections("test_payload_1").await?;

    // Should have two connections (to tx2 via email and to tx3 via phone)
    assert_eq!(connections.len(), 2);

    // Verify connection details
    let mut contains_tx2_email = false;
    let mut contains_tx3_phone = false;

    for conn in connections {
        if conn.transaction_id == 2 && conn.matcher == "email" {
            contains_tx2_email = true;
            assert_eq!(conn.confidence, 90);
            assert_eq!(conn.importance, 80);
        } else if conn.transaction_id == 3 && conn.matcher == "phone" {
            contains_tx3_phone = true;
            assert_eq!(conn.confidence, 85);
            assert_eq!(conn.importance, 75);
        }
    }

    assert!(
        contains_tx2_email,
        "Should contain a connection to tx2 via email"
    );
    assert!(
        contains_tx3_phone,
        "Should contain a connection to tx3 via phone"
    );

    // Get direct connections for tx2 (only connected to tx1 via email)
    let connections = storage.get_direct_connections("test_payload_2").await?;
    assert_eq!(connections.len(), 1);
    assert_eq!(connections[0].transaction_id, 1);
    assert_eq!(connections[0].matcher, "email");

    // Get direct connections for an unconnected transaction
    let _ = create_test_transaction(&storage.db).await?;
    let connections = storage.get_direct_connections("test_payload_4").await?;
    assert_eq!(
        connections.len(),
        0,
        "Unconnected transaction should have no connections"
    );

    Ok(())
}
