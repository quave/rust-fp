use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use chrono::NaiveDateTime;

/// SeaORM Transaction Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "transactions")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub label_id: Option<i64>,
    pub comment: Option<String>,
    pub last_scoring_date: Option<NaiveDateTime>,
    pub processing_complete: bool,
    pub created_at: NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "order::Entity")]
    Orders,
}

impl Related<order::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Orders.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

/// SeaORM Order Entity
pub mod order {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "orders")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub transaction_id: i64,
        pub order_number: String,
        pub delivery_type: String,
        pub delivery_details: String,
        pub created_at: NaiveDateTime,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(belongs_to = "super::Entity", from = "Column::TransactionId", to = "super::Column::Id")]
        Transaction,
        #[sea_orm(has_many = "super::order_item::Entity")]
        OrderItems,
        #[sea_orm(has_many = "super::customer::Entity")]
        Customers,
        #[sea_orm(has_many = "super::billing_data::Entity")]
        BillingData,
    }

    impl Related<super::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Transaction.def()
        }
    }

    impl Related<super::order_item::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::OrderItems.def()
        }
    }

    impl Related<super::customer::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Customers.def()
        }
    }

    impl Related<super::billing_data::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::BillingData.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

/// SeaORM Order Item Entity
pub mod order_item {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "order_items")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub order_id: i64,
        pub name: String,
        pub category: String,
        pub price: f32,
        pub created_at: NaiveDateTime,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(belongs_to = "super::order::Entity", from = "Column::OrderId", to = "super::order::Column::Id")]
        Order,
    }

    impl Related<super::order::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Order.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

/// SeaORM Customer Entity
pub mod customer {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "customers")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub order_id: i64,
        pub name: String,
        pub email: String,
        pub created_at: NaiveDateTime,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(belongs_to = "super::order::Entity", from = "Column::OrderId", to = "super::order::Column::Id")]
        Order,
    }

    impl Related<super::order::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Order.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

/// SeaORM Billing Data Entity
pub mod billing_data {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "billing_data")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub order_id: i64,
        pub payment_type: String,
        pub payment_details: String,
        pub billing_address: String,
        pub created_at: NaiveDateTime,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(belongs_to = "super::order::Entity", from = "Column::OrderId", to = "super::order::Column::Id")]
        Order,
    }

    impl Related<super::order::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Order.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_creation() {
        // Test that we can create entity instances
        let transaction = Model {
            id: 1,
            label_id: None,
            comment: None,
            last_scoring_date: None,
            processing_complete: false,
            created_at: chrono::DateTime::from_timestamp(0, 0).unwrap().naive_utc(),
        };
        
        assert_eq!(transaction.id, 1);
        assert_eq!(transaction.processing_complete, false);
    }

    #[test]
    fn test_order_entity_creation() {
        // Test that we can create order entity instances
        let order = order::Model {
            id: 1,
            transaction_id: 1,
            order_number: "ORD-001".to_string(),
            delivery_type: "standard".to_string(),
            delivery_details: "Home delivery".to_string(),
            created_at: chrono::DateTime::from_timestamp(0, 0).unwrap().naive_utc(),
        };
        
        assert_eq!(order.id, 1);
        assert_eq!(order.order_number, "ORD-001");
    }
} 