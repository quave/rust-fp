use std::error::Error;

use frida_core::{
    storage::{ImportableStorage, ProcessibleStorage}, 
    test_utils::{setup_test_environment, get_test_database_url},
};
use frida_ecom::{ecom_import_model::*, ecom_order_storage::EcomOrderStorage};
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

#[tokio::test]
async fn test_save_and_retrieve_order() -> Result<(), Box<dyn Error + Send + Sync>> {
    let storage = get_test_storage().await;
    let test_order = ImportOrder {
        order_number: "TEST-123".to_string(),
        items: vec![ImportOrderItem {
            name: "Test Item".to_string(),
            category: "Test Category".to_string(),
            price: 10.0,
        }],
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
    };

    // Test saving
    let id: i64 = storage.save_transaction(&test_order).await?;

    // Test retrieving
    let retrieved_order = storage.get_processible(id).await?;

    assert_eq!(test_order.order_number, retrieved_order.order.order_number);
    assert_eq!(test_order.customer.name, retrieved_order.customer.name);
    assert_eq!(test_order.items.len(), retrieved_order.items.len());

    Ok(())
}
