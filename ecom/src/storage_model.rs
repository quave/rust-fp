use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use chrono::NaiveDateTime;

/// SeaORM Order Entity
pub mod order {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "orders")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub transaction_id: Option<i64>,
        pub order_number: String,
        pub delivery_type: String,
        pub delivery_details: String,
        pub created_at: NaiveDateTime,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        // #[sea_orm(belongs_to = "super::Entity", from = "Column::TransactionId", to = "super::Column::Id")]
        // Transaction,
        #[sea_orm(has_many = "super::order_item::Entity")]
        OrderItems,
        #[sea_orm(has_many = "super::customer::Entity")]
        Customers,
        #[sea_orm(has_many = "super::billing_data::Entity")]
        BillingData,
    }

    // impl Related<super::Entity> for Entity {
    //     fn to() -> RelationDef {
    //         Relation::Transaction.def()
    //     }
    // }

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


    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelatedEntity)]
    pub enum RelatedEntity {
        #[sea_orm(entity = "super::order_item::Entity")]
        OrderItem,
        #[sea_orm(entity = "super::customer::Entity")]
        Customer,
        #[sea_orm(entity = "super::billing_data::Entity")]
        BillingData
    }
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

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelatedEntity)]
    pub enum RelatedEntity {
        #[sea_orm(entity = "super::order::Entity")]
        Order,
    }
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

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelatedEntity)]
    pub enum RelatedEntity {
        #[sea_orm(entity = "super::order::Entity")]
        Order,
    }

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

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelatedEntity)]
    pub enum RelatedEntity {
        #[sea_orm(entity = "super::order::Entity")]
        Order,
    }

}
