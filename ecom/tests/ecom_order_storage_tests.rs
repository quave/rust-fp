use std::error::Error;

use processing::{
    storage::{ImportableStorage, ProcessibleStorage, WebStorage},
    test_utils::{setup_test_environment, get_test_database_url},
};

use ecom::{
    ecom_order_storage::EcomOrderStorage,
    ecom_import_model::{ImportOrder, ImportOrderItem, ImportCustomerData, ImportBillingData},
};
use tokio::sync::OnceCell;

static SETUP: OnceCell<()> = OnceCell::const_new();

async fn ensure_setup() {
    SETUP.get_or_init(|| async {
        setup_test_environment().await.expect("Failed to setup test environment");
    }).await;
}

async fn get_test_storage() -> EcomOrderStorage {
    ensure_setup().await;
    EcomOrderStorage::new(&get_test_database_url())
        .await
        .expect("Failed to create storage")
}

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
async fn test_save_transaction() -> Result<(), Box<dyn Error + Send + Sync>> {
    let storage = get_test_storage().await;
    let test_order = create_test_order("TEST-123");
    let transaction_id = storage.save_transaction(&test_order).await?;
    assert!(transaction_id > 0);
    Ok(())
}

#[tokio::test]
async fn test_save_and_retrieve_order() -> Result<(), Box<dyn Error + Send + Sync>> {
    let storage = get_test_storage().await;
    let test_order = create_test_order("TEST-123");

    // Test saving
    let id = storage.save_transaction(&test_order).await?;

    // Test retrieving
    let retrieved_order = storage.get_processible(id).await?;

    assert_eq!(test_order.order_number, retrieved_order.order.order_number);
    assert_eq!(test_order.customer.name, retrieved_order.customer.name);
    assert_eq!(test_order.items.len(), retrieved_order.items.len());
    assert_eq!(test_order.billing.payment_type, retrieved_order.billing.payment_type);

    Ok(())
}

#[tokio::test]
async fn test_get_transactions() -> Result<(), Box<dyn Error + Send + Sync>> {
    let storage = get_test_storage().await;
    
    // Create multiple orders
    let order1 = create_test_order("TEST-1");
    let order2 = create_test_order("TEST-2");
    
    let _id1 = storage.save_transaction(&order1).await?;
    let _id2 = storage.save_transaction(&order2).await?;

    // Get all transactions
    let transactions = storage.get_transactions().await?;
    
    assert!(transactions.len() >= 2);
    assert!(transactions.iter().any(|t| t.order.order_number == "TEST-1"));
    assert!(transactions.iter().any(|t| t.order.order_number == "TEST-2"));

    Ok(())
}

#[tokio::test]
async fn test_get_transaction() -> Result<(), Box<dyn Error + Send + Sync>> {
    let storage = get_test_storage().await;
    let test_order = create_test_order("TEST-123");

    // Save and get transaction
    let id = storage.save_transaction(&test_order).await?;
    let transaction = storage.get_transaction(id).await?;

    assert_eq!(test_order.order_number, transaction.order.order_number);
    assert_eq!(test_order.customer.name, transaction.customer.name);
    assert_eq!(test_order.items.len(), transaction.items.len());
    assert_eq!(test_order.billing.payment_type, transaction.billing.payment_type);

    Ok(())
}

#[tokio::test]
async fn test_filter_orders() -> Result<(), Box<dyn Error + Send + Sync>> {
    let storage = get_test_storage().await;
    
    // Create test orders
    let order1 = create_test_order("TEST-1");
    let order2 = create_test_order("TEST-2");
    
    let _id1 = storage.save_transaction(&order1).await?;
    let _id2 = storage.save_transaction(&order2).await?;

    // Get filtered orders
    let order_ids = storage.filter_orders(None).await?;
    
    assert!(order_ids.len() >= 2);
    assert!(order_ids.contains(&_id1));
    assert!(order_ids.contains(&_id2));

    Ok(())
}

#[tokio::test]
async fn test_get_orders_by_ids() -> Result<(), Box<dyn Error + Send + Sync>> {
    let storage = get_test_storage().await;
    
    // Create test orders
    let order1 = create_test_order("TEST-1");
    let order2 = create_test_order("TEST-2");
    
    let tx_id1 = storage.save_transaction(&order1).await?;
    let tx_id2 = storage.save_transaction(&order2).await?;
    let order1 = storage.get_transaction(tx_id1).await?;
    let order2 = storage.get_transaction(tx_id2).await?;

    // Get orders by IDs
    let orders = storage.get_orders_by_ids(vec![order1.order.id, order2.order.id]).await?;
    
    assert_eq!(orders.len(), 2);
    assert!(orders.iter().any(|o| o.order.order_number == "TEST-1"));
    assert!(orders.iter().any(|o| o.order.order_number == "TEST-2"));

    Ok(())
}

#[tokio::test]
async fn test_order_relationships() -> Result<(), Box<dyn Error + Send + Sync>> {
    let storage = get_test_storage().await;
    let test_order = create_test_order("TEST-123");

    // Save and retrieve order
    let id = storage.save_transaction(&test_order).await?;
    let retrieved_order = storage.get_processible(id).await?;

    // Verify relationships
    assert_eq!(retrieved_order.items.len(), 2);
    assert_eq!(retrieved_order.items[0].order_id, retrieved_order.order.id);
    assert_eq!(retrieved_order.items[1].order_id, retrieved_order.order.id);
    assert_eq!(retrieved_order.customer.order_id, retrieved_order.order.id);
    assert_eq!(retrieved_order.billing.order_id, retrieved_order.order.id);

    Ok(())
}

#[tokio::test]
async fn test_order_data_integrity() -> Result<(), Box<dyn Error + Send + Sync>> {
    let storage = get_test_storage().await;
    let test_order = create_test_order("TEST-123");

    // Save and retrieve order
    let id = storage.save_transaction(&test_order).await?;
    let retrieved_order = storage.get_processible(id).await?;

    // Verify data integrity
    assert_eq!(retrieved_order.items[0].name, "Test Item 1");
    assert_eq!(retrieved_order.items[0].category, "Test Category 1");
    assert_eq!(retrieved_order.items[0].price, 10.0);
    assert_eq!(retrieved_order.items[1].name, "Test Item 2");
    assert_eq!(retrieved_order.items[1].category, "Test Category 2");
    assert_eq!(retrieved_order.items[1].price, 20.0);
    assert_eq!(retrieved_order.customer.name, "Test Customer");
    assert_eq!(retrieved_order.customer.email, "test@example.com");
    assert_eq!(retrieved_order.billing.payment_type, "credit_card");
    assert_eq!(retrieved_order.billing.billing_address, "Test Address");

    Ok(())
}
