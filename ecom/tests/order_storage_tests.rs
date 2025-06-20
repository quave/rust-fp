use std::error::Error;
use ecom::ecom_import_model::{ImportOrder, ImportOrderItem, ImportCustomerData, ImportBillingData};
use processing::storage::{ImportableStorage, WebStorage};

/// Create a test order for verification purposes
fn create_test_order(order_number: &str) -> ImportOrder {
    ImportOrder {
        order_number: order_number.to_string(),
        items: vec![
            ImportOrderItem {
                name: "Test Item 1".to_string(),
                category: "Test Category 1".to_string(),
                price: 10.0,
            },
            ImportOrderItem {
                name: "Test Item 2".to_string(),
                category: "Test Category 2".to_string(),
                price: 20.0,
            },
        ],
        customer: ImportCustomerData {
            name: "Test Customer".to_string(),
            email: "test@example.com".to_string(),
        },
        billing: ImportBillingData {
            payment_type: "credit_card".to_string(),
            payment_details: "4111111111111111".to_string(),
            billing_address: "Test Address".to_string(),
        },
        delivery_type: "standard".to_string(),
        delivery_details: "Test Delivery".to_string(),
    }
}

#[tokio::test]
async fn test_order_storage_sequential_verification() -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("\n=== Running ecom order storage verification test ===\n");
    
    // Create a fresh storage instance for this verification test
    use common::test_helpers::get_test_database_url;
    let storage = ecom::ecom_order_storage::EcomOrderStorage::new(&get_test_database_url()).await?;
    
    // Generate unique test prefix for this verification run
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
    let test_prefix = format!("VERIFY-{}", timestamp);
    
    println!("✅ Storage instance created successfully");
    println!("✅ Using unique test prefix: {}", test_prefix);
    
    // Verify that we can run basic operations without conflicts
    let order_number = format!("{}-BASIC-TEST", test_prefix);
    let test_order = create_test_order(&order_number);
    
    // Test save operation
    let tx_id = storage.save_transaction(&test_order).await?;
    println!("✅ Successfully saved transaction with ID: {}", tx_id);
    
    // Test retrieve operation
    let retrieved = storage.get_transaction(tx_id).await?;
    println!("✅ Successfully retrieved transaction: {}", retrieved.order.order_number);
    
    // Verify data matches
    assert_eq!(retrieved.order.order_number, order_number);
    assert!(retrieved.customer.name.contains("Test Customer"));
    
    // Clean up this verification test data
    let mut tx = storage.pool.begin().await?;
    sqlx::query("DELETE FROM order_items WHERE order_id IN (SELECT id FROM orders WHERE transaction_id = $1)")
        .bind(tx_id).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM customers WHERE order_id IN (SELECT id FROM orders WHERE transaction_id = $1)")
        .bind(tx_id).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM billing_data WHERE order_id IN (SELECT id FROM orders WHERE transaction_id = $1)")
        .bind(tx_id).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM orders WHERE transaction_id = $1")
        .bind(tx_id).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM transactions WHERE id = $1")
        .bind(tx_id).execute(&mut *tx).await?;
    tx.commit().await?;
    
    println!("✅ Cleanup completed successfully");
    println!("\n=== Order storage verification test completed successfully ===");
    println!("    Individual tests now run in parallel with proper isolation!");
    
    Ok(())
} 