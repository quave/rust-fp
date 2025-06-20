use async_trait::async_trait;
use chrono::{serde::ts_seconds, DateTime, Utc};
use processing::model::*;
use processing::ui_model::ModelRegistry;
use serde::{Serialize, Deserialize};
use proc_macros::Relatable;
use std::sync::OnceLock;

// Define a static model registry
static MODEL_REGISTRY: OnceLock<ModelRegistry> = OnceLock::new();


/// Order item model
/// 
/// Table: order_items
/// Relations:
///   - belongs_to: DbOrder (via order_id)
#[derive(Debug, Clone, Serialize, Deserialize, Default, Relatable)]
#[table_name("order_items")]
#[import_path("processing::ui_model")]
#[primary_key("id")]
#[relation(r#""order" => (RelationKind::BelongsTo, "orders", "order_id")"#)]
pub struct DbOrderItem {
    pub id: ModelId,
    pub order_id: ModelId,
    pub name: String,
    pub category: String,
    pub price: f32,
    #[serde(with = "ts_seconds")]
    pub created_at: DateTime<Utc>,
}

/// Customer data model
/// 
/// Table: customers
/// Relations:
///   - belongs_to: DbOrder (via order_id)
#[derive(Debug, Clone, Serialize, Deserialize, Default, Relatable)]
#[table_name("customers")]
#[import_path("processing::ui_model")]
#[primary_key("id")]
#[relation(r#""order" => (RelationKind::BelongsTo, "orders", "order_id")"#)]
pub struct DbCustomerData {
    pub id: ModelId,
    pub order_id: ModelId,
    pub name: String,
    pub email: String,
    #[serde(with = "ts_seconds")]
    pub created_at: DateTime<Utc>,
}

/// Billing data model
/// 
/// Table: billing_data
/// Relations:
///   - belongs_to: DbOrder (via order_id)
#[derive(Debug, Clone, Serialize, Deserialize, Default, Relatable)]
#[table_name("billing_data")]
#[import_path("processing::ui_model")]
#[primary_key("id")]
#[relation(r#""order" => (RelationKind::BelongsTo, "orders", "order_id")"#)]
pub struct DbBillingData {
    pub id: ModelId,
    pub order_id: ModelId,
    pub payment_type: String,
    pub payment_details: String,
    pub billing_address: String,
    #[serde(with = "ts_seconds")]
    pub created_at: DateTime<Utc>,
}

/// Order model
/// 
/// Table: orders
/// Relations:
///   - belongs_to: Transaction (via transaction_id)
///   - has_many: DbOrderItem (via order_id)
///   - has_one: DbCustomerData (via order_id)
///   - has_one: DbBillingData (via order_id)
#[derive(Debug, Clone, Serialize, Deserialize, Default, Relatable)]
#[table_name("orders")]
#[import_path("processing::ui_model")]
#[primary_key("id")]
#[relation(r#""transaction" => (RelationKind::BelongsTo, "transactions", "transaction_id")"#)]
#[relation(r#""items" => (RelationKind::HasMany, "order_items", "order_id")"#)]
#[relation(r#""customer" => (RelationKind::HasOne, "customers", "order_id")"#)]
#[relation(r#""billing" => (RelationKind::HasOne, "billing_data", "order_id")"#)]
pub struct DbOrder {
    pub id: ModelId,
    pub transaction_id: ModelId,
    pub order_number: String,
    pub delivery_type: String,
    pub delivery_details: String,
    #[serde(with = "ts_seconds")]
    pub created_at: DateTime<Utc>,
}

/// Combined Order model with related data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub order: DbOrder,
    pub items: Vec<DbOrderItem>,
    pub customer: DbCustomerData,
    pub billing: DbBillingData,
}

impl WebTransaction for Order {
    fn id(&self) -> ModelId {
        self.order.id
    }
}

impl ModelRegistryProvider for Order {
    fn get_registry() -> &'static ModelRegistry {
        use processing::ui_model::Relatable;

        MODEL_REGISTRY.get_or_init(|| {
            let mut registry = ModelRegistry::new(DbOrder::into_table());
            
            registry.add_table(DbOrderItem::into_table());
            registry.add_table(DbCustomerData::into_table());
            registry.add_table(DbBillingData::into_table());
            
            registry
        })
    }
}

#[async_trait]
impl Processible for Order {
    fn extract_simple_features(
        &self,
    ) -> Vec<Feature> {
        let mut features = Vec::new();

        features.push(Feature {
            name: "amount".to_string(),
            value: Box::new(FeatureValue::Double(
                self.items.iter().map(|i| i.price as f64).sum(),
            )),
        });

        features.push(Feature {
            name: "amounts".to_string(),
            value: Box::new(FeatureValue::DoubleList(
                self.items.iter().map(|i| i.price as f64).collect(),
            )),
        });

        features.push(Feature {
            name: "categories".to_string(),
            value: Box::new(FeatureValue::StringList(
                self.items.iter().map(|i| i.category.clone()).collect(),
            )),
        });

        features.push(Feature {
            name: "created_at".to_string(),
            value: Box::new(FeatureValue::DateTime(self.order.created_at)),
        });

        features.push(Feature {
            name: "is_high_value".to_string(),
            value: Box::new(FeatureValue::Bool(
                self.items.iter().map(|i| i.price as f64).sum::<f64>() > 1000.0,
            )),
        });

        features
    }

    fn extract_graph_features(
        &self,
        connected_transactions: &[ConnectedTransaction],
        direct_connections: &[DirectConnection]
    ) -> Vec<Feature> {
        let mut features = Vec::new();
        // Add connection-related features
        features.push(Feature {
            name: "connected_transaction_count".to_string(),
            value: Box::new(FeatureValue::Int(connected_transactions.len() as i64)),
        });
        
        features.push(Feature {
            name: "direct_connection_count".to_string(),
            value: Box::new(FeatureValue::Int(direct_connections.len() as i64)),
        });

        features
    }

    fn tx_id(&self) -> ModelId {
        self.order.transaction_id
    }

    fn id(&self) -> ModelId {
        self.order.id
    }

    fn extract_matching_fields(&self) -> Vec<MatchingField> {
        vec![
            MatchingField {
                matcher: "customer.email".to_string(),
                value: self.customer.email.clone(),
            },
            MatchingField {
                matcher: "billing.payment_details".to_string(),
                value: self.billing.payment_details.clone(),
            }
        ]
    }
}
