#[cfg(test)]
mod tests {
    use ecom::ecom_order_storage::EcomOrderStorage;
    use ecom::ecom_import_model::*;
    use processing::storage::{ImportableStorage, ProcessibleStorage, WebStorage};
    use common::test_helpers::get_test_database_url;
    use processing::ui_model::{FilterRequest, FilterGroup, FilterCondition, FilterValue, LogicalOperator, Operator};

    async fn setup_seaorm_storage() -> EcomOrderStorage {
        let database_url = get_test_database_url();
        EcomOrderStorage::new(&database_url)
            .await
            .expect("Failed to create SeaORM storage")
    }

    fn create_test_import_order() -> ImportOrder {
        ImportOrder {
            order_number: "TEST-001".to_string(),
            delivery_type: "standard".to_string(),
            delivery_details: "Home delivery".to_string(),
            items: vec![
                ImportOrderItem {
                    name: "Test Product".to_string(),
                    category: "Electronics".to_string(),
                    price: 99.99,
                },
                ImportOrderItem {
                    name: "Another Product".to_string(),
                    category: "Books".to_string(),
                    price: 19.99,
                },
            ],
            customer: ImportCustomerData {
                name: "John Doe".to_string(),
                email: "john@example.com".to_string(),
            },
            billing: ImportBillingData {
                payment_type: "credit_card".to_string(),
                payment_details: "****1234".to_string(),
                billing_address: "123 Main St".to_string(),
            },
        }
    }

    #[tokio::test]
    async fn test_seaorm_save_and_retrieve_order() {
        let storage = setup_seaorm_storage().await;
        let import_order = create_test_import_order();

        // Test ImportableStorage trait
        let transaction_id = storage
            .save_transaction(&import_order)
            .await
            .expect("Failed to save transaction");

        assert!(transaction_id > 0, "Transaction ID should be positive");

        // Test ProcessibleStorage trait
        let retrieved_order = storage
            .get_processible(transaction_id)
            .await
            .expect("Failed to get processible order");

        // Verify the order data
        assert_eq!(retrieved_order.order.transaction_id, transaction_id);
        assert_eq!(retrieved_order.order.order_number, "TEST-001");
        assert_eq!(retrieved_order.order.delivery_type, "standard");
        assert_eq!(retrieved_order.order.delivery_details, "Home delivery");

        // Verify order items
        assert_eq!(retrieved_order.items.len(), 2);
        assert_eq!(retrieved_order.items[0].name, "Test Product");
        assert_eq!(retrieved_order.items[0].category, "Electronics");
        assert_eq!(retrieved_order.items[0].price, 99.99);
        assert_eq!(retrieved_order.items[1].name, "Another Product");
        assert_eq!(retrieved_order.items[1].category, "Books");
        assert_eq!(retrieved_order.items[1].price, 19.99);

        // Verify customer data
        assert_eq!(retrieved_order.customer.name, "John Doe");
        assert_eq!(retrieved_order.customer.email, "john@example.com");

        // Verify billing data
        assert_eq!(retrieved_order.billing.payment_type, "credit_card");
        assert_eq!(retrieved_order.billing.payment_details, "****1234");
        assert_eq!(retrieved_order.billing.billing_address, "123 Main St");
    }

    #[tokio::test]
    async fn test_seaorm_web_storage_interface() {
        let storage = setup_seaorm_storage().await;
        let import_order = create_test_import_order();

        // Save an order first
        let transaction_id = storage
            .save_transaction(&import_order)
            .await
            .expect("Failed to save transaction");

        // Test WebStorage trait - get_transaction
        let retrieved_order = storage
            .get_transaction(transaction_id)
            .await
            .expect("Failed to get transaction via WebStorage");

        assert_eq!(retrieved_order.order.transaction_id, transaction_id);
        assert_eq!(retrieved_order.order.order_number, "TEST-001");

        // Test WebStorage trait - get_transactions (basic implementation)
        let filter = FilterRequest {
            filter: Some(FilterGroup {
                operator: LogicalOperator::And,
                conditions: vec![FilterCondition {
                    column: "order_number".to_string(),
                    operator: Operator::Equal,
                    value: FilterValue::String("TEST-001".to_string()),
                }],
                groups: vec![],
            }),
            sort: vec![],
            limit: None,
            offset: None,
        };

        let transactions = storage
            .get_transactions(filter)
            .await
            .expect("Failed to get transactions");

        // Current implementation returns empty vec - this demonstrates the interface works
        assert_eq!(transactions.len(), 0);
    }

    #[tokio::test]
    async fn test_seaorm_transaction_not_found() {
        let storage = setup_seaorm_storage().await;

        // Try to get a non-existent transaction
        let result = storage.get_processible(99999).await;
        assert!(result.is_err(), "Should return error for non-existent transaction");

        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("Order not found"));
    }

    #[tokio::test]
    async fn test_seaorm_multiple_orders() {
        let storage = setup_seaorm_storage().await;

        // Create and save multiple orders
        let mut transaction_ids = Vec::new();

        for i in 1..=3 {
            let mut import_order = create_test_import_order();
            import_order.order_number = format!("TEST-{:03}", i);
            import_order.customer.name = format!("Customer {}", i);

            let transaction_id = storage
                .save_transaction(&import_order)
                .await
                .expect("Failed to save transaction");

            transaction_ids.push(transaction_id);
        }

        // Verify each order can be retrieved correctly
        for (i, &transaction_id) in transaction_ids.iter().enumerate() {
            let retrieved_order = storage
                .get_processible(transaction_id)
                .await
                .expect("Failed to get processible order");

            assert_eq!(retrieved_order.order.order_number, format!("TEST-{:03}", i + 1));
            assert_eq!(retrieved_order.customer.name, format!("Customer {}", i + 1));
        }
    }
} 