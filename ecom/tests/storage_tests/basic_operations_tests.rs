use ecom::ecom_import_model::{ImportOrder, ImportOrderItem, ImportCustomerData, ImportBillingData};
use ecom::models::Order;
use processing::storage::{ImportableStorage, WebStorage};
use std::error::Error;
use common::test_helpers::{generate_unique_id, get_test_database_url};

/// Helper function to create test import order data
fn create_test_import_order(unique_prefix: &str) -> ImportOrder {
    ImportOrder {
        order_number: format!("{}-ORDER", unique_prefix),
        items: vec![
            ImportOrderItem {
                name: "Test Product 1".to_string(),
                category: "Electronics".to_string(),
                price: 99.99,
            },
            ImportOrderItem {
                name: "Test Product 2".to_string(),
                category: "Books".to_string(),
                price: 19.99,
            },
        ],
        customer: ImportCustomerData {
            name: "John Doe".to_string(),
            email: format!("{}@test.com", unique_prefix.to_lowercase()),
        },
        billing: ImportBillingData {
            payment_type: "credit_card".to_string(),
            payment_details: "4111111111111111".to_string(),
            billing_address: "123 Test Street".to_string(),
        },
        delivery_type: "standard".to_string(),
        delivery_details: "Standard delivery".to_string(),
    }
}

#[tokio::test]
async fn test_save_transaction() -> Result<(), Box<dyn Error + Send + Sync>> {
    // Create storage instance
    let storage = ecom::ecom_order_storage::EcomOrderStorage::new(&get_test_database_url()).await?;
    
    // Generate unique test identifier
    let test_id = generate_unique_id("SAVE-TX");
    let test_order = create_test_import_order(&test_id);
    
    // Save transaction and verify
    let transaction_id = storage.save_transaction(&test_order).await?;
    assert!(transaction_id > 0, "Transaction ID should be positive");
    
    Ok(())
}

#[tokio::test]
async fn test_get_transaction() -> Result<(), Box<dyn Error + Send + Sync>> {
    // Create storage instance
    let storage = ecom::ecom_order_storage::EcomOrderStorage::new(&get_test_database_url()).await?;
    
    // Generate unique test identifier and create order
    let test_id = generate_unique_id("GET-TX");
    let test_order = create_test_import_order(&test_id);
    
    // Save and retrieve transaction
    let transaction_id = storage.save_transaction(&test_order).await?;
    let retrieved: Order = storage.get_transaction(transaction_id).await?;
    
    // Verify retrieved data matches original
    assert_eq!(retrieved.order.order_number, test_order.order_number);
    assert_eq!(retrieved.customer.email, test_order.customer.email);
    assert_eq!(retrieved.items.len(), test_order.items.len());
    
    Ok(())
}

#[tokio::test]
async fn test_save_and_retrieve_order() -> Result<(), Box<dyn Error + Send + Sync>> {
    // Create storage instance
    let storage = ecom::ecom_order_storage::EcomOrderStorage::new(&get_test_database_url()).await?;
    
    // Generate unique test identifier
    let test_id = generate_unique_id("SAVE-RETRIEVE");
    let test_order = create_test_import_order(&test_id);
    
    // Save transaction
    let transaction_id = storage.save_transaction(&test_order).await?;
    
    // Retrieve and verify all components
    let retrieved: Order = storage.get_transaction(transaction_id).await?;
    
    // Verify order details
    assert_eq!(retrieved.order.order_number, test_order.order_number);
    assert_eq!(retrieved.order.delivery_type, test_order.delivery_type);
    assert_eq!(retrieved.order.delivery_details, test_order.delivery_details);
    
    // Verify customer details
    assert_eq!(retrieved.customer.name, test_order.customer.name);
    assert_eq!(retrieved.customer.email, test_order.customer.email);
    
    // Verify billing details
    assert_eq!(retrieved.billing.payment_type, test_order.billing.payment_type);
    assert_eq!(retrieved.billing.payment_details, test_order.billing.payment_details);
    assert_eq!(retrieved.billing.billing_address, test_order.billing.billing_address);
    
    // Verify order items
    assert_eq!(retrieved.items.len(), 2);
    assert_eq!(retrieved.items[0].name, "Test Product 1");
    assert_eq!(retrieved.items[0].category, "Electronics");
    assert_eq!(retrieved.items[0].price, 99.99);
    assert_eq!(retrieved.items[1].name, "Test Product 2");
    assert_eq!(retrieved.items[1].category, "Books");
    assert_eq!(retrieved.items[1].price, 19.99);
    
    Ok(())
} 