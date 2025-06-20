use async_trait::async_trait;
use chrono::{serde::ts_seconds, DateTime, Utc};
use serde::{Deserialize, Serialize};

use processing::model::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbOrderItem {
    pub id: ModelId,
    pub order_id: ModelId,
    pub name: String,
    pub category: String,
    pub price: f32,
    #[serde(with = "ts_seconds")]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbCustomerData {
    pub id: ModelId,
    pub order_id: ModelId,
    pub name: String,
    pub email: String,
    #[serde(with = "ts_seconds")]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbBillingData {
    pub id: ModelId,
    pub order_id: ModelId,
    pub payment_type: String,
    pub payment_details: String,
    pub billing_address: String,
    #[serde(with = "ts_seconds")]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbOrder {
    pub id: ModelId,
    pub transaction_id: ModelId,
    pub order_number: String,
    pub delivery_type: String,
    pub delivery_details: String,
    #[serde(with = "ts_seconds")]
    pub created_at: DateTime<Utc>,
}

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

#[async_trait]
impl Processible for Order {
    fn extract_features(&self) -> Vec<Feature> {
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
