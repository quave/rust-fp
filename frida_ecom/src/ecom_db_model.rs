use std::error::Error;

use async_trait::async_trait;
use chrono::{serde::ts_seconds, DateTime, Utc};
use serde::{Deserialize, Serialize};

use frida_core::model::{Feature, FeatureValue, Processible};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbOrderItem {
    pub id: i64,
    pub order_id: i64,
    pub name: String,
    pub category: String,
    pub price: f64,
    #[serde(with = "ts_seconds")]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbCustomerData {
    pub id: i64,
    pub order_id: i64,
    pub name: String,
    pub email: String,
    #[serde(with = "ts_seconds")]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbBillingData {
    pub id: i64,
    pub order_id: i64,
    pub payment_type: String,
    pub payment_details: String,
    pub billing_address: String,
    #[serde(with = "ts_seconds")]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbOrder {
    pub id: i64,
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

#[async_trait]
impl Processible for Order {
    fn extract_features(&self) -> Vec<Feature> {
        let mut features = Vec::new();

        features.push(Feature {
            name: "order_total".to_string(),
            value: Box::new(FeatureValue::Double(
                self.items.iter().map(|i| i.price).sum(),
            )),
        });

        features.push(Feature {
            name: "item_count".to_string(),
            value: Box::new(FeatureValue::Int(self.items.len() as i64)),
        });

        // Add more feature extraction logic...

        features
    }

    fn id(&self) -> i64 {
        self.order.id
    }

    fn as_json(&self) -> Result<String, Box<dyn Error + Send + Sync>> {
        Ok(serde_json::to_string(self)?)
    }
}
