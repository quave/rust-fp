#[cfg(test)]
mod tests {
    use ecom::storage_model::*;

    #[test]
    fn test_order_entity_creation() {
        // Test creating an order entity
        let order = order::Model {
            id: 1,
            transaction_id: Some(1),
            order_number: "ORD-12345".to_string(),
            delivery_type: "express".to_string(),
            delivery_details: "Next day delivery".to_string(),
            created_at: chrono::DateTime::from_timestamp(1640995200, 0).unwrap().naive_utc(),
        };
        
        assert_eq!(order.id, 1);
        assert_eq!(order.order_number, "ORD-12345");
        assert_eq!(order.delivery_type, "express");
    }

    #[test]
    fn test_order_item_entity_creation() {
        // Test creating an order item entity
        let item = order_item::Model {
            id: 1,
            order_id: 1,
            name: "Widget".to_string(),
            category: "Electronics".to_string(),
            price: 29.99,
            created_at: chrono::DateTime::from_timestamp(1640995200, 0).unwrap().naive_utc(),
        };
        
        assert_eq!(item.id, 1);
        assert_eq!(item.order_id, 1);
        assert_eq!(item.name, "Widget");
        assert_eq!(item.category, "Electronics");
        assert_eq!(item.price, 29.99);
    }

    #[test]
    fn test_customer_entity_creation() {
        // Test creating a customer entity
        let customer = customer::Model {
            id: 1,
            order_id: 1,
            name: "John Doe".to_string(),
            email: "john.doe@example.com".to_string(),
            created_at: chrono::DateTime::from_timestamp(1640995200, 0).unwrap().naive_utc(),
        };
        
        assert_eq!(customer.id, 1);
        assert_eq!(customer.order_id, 1);
        assert_eq!(customer.name, "John Doe");
        assert_eq!(customer.email, "john.doe@example.com");
    }

    #[test]
    fn test_billing_data_entity_creation() {
        // Test creating a billing data entity
        let billing = billing_data::Model {
            id: 1,
            order_id: 1,
            payment_type: "Credit Card".to_string(),
            payment_details: "**** **** **** 1234".to_string(),
            billing_address: "123 Main St, City, State 12345".to_string(),
            created_at: chrono::DateTime::from_timestamp(1640995200, 0).unwrap().naive_utc(),
        };
        
        assert_eq!(billing.id, 1);
        assert_eq!(billing.order_id, 1);
        assert_eq!(billing.payment_type, "Credit Card");
        assert_eq!(billing.payment_details, "**** **** **** 1234");
        assert_eq!(billing.billing_address, "123 Main St, City, State 12345");
    }

    #[test]
    fn test_entity_serialization() {
        // Test that entities can be serialized/deserialized
        let order = order::Model {
            id: 1,
            transaction_id: Some(1),
            order_number: "ORD-12345".to_string(),
            delivery_type: "express".to_string(),
            delivery_details: "Next day delivery".to_string(),
            created_at: chrono::DateTime::from_timestamp(1640995200, 0).unwrap().naive_utc(),
        };
        
        // Test serialization
        let json_result = serde_json::to_string(&order);
        assert!(json_result.is_ok());
        
        let json = json_result.unwrap();
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"delivery_type\":\"express\""));
        
        // Test deserialization
        let deserialized: Result<order::Model, _> = serde_json::from_str(&json);
        assert!(deserialized.is_ok());
        
        let deserialized_order = deserialized.unwrap();
        assert_eq!(deserialized_order.id, order.id);
        assert_eq!(deserialized_order.delivery_type, order.delivery_type);
    }
} 