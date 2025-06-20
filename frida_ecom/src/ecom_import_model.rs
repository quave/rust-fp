use std::error::Error;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use frida_core::model::{Importable, ImportableSerde};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportOrderItem {
    pub name: String,
    pub category: String,
    pub price: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportCustomerData {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportBillingData {
    pub payment_type: String,
    pub payment_details: String,
    pub billing_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportOrder {
    pub order_number: String,
    pub items: Vec<ImportOrderItem>,
    pub customer: ImportCustomerData,
    pub billing: ImportBillingData,
    pub delivery_type: String,
    pub delivery_details: String,
}

#[async_trait]
impl Importable for ImportOrder {
    fn validate(&self) -> Result<(), String> {
        Ok(())
    }
}

#[async_trait]
impl ImportableSerde for ImportOrder {
    fn as_json(&self) -> Result<String, Box<dyn Error + Send + Sync>> {
        Ok(serde_json::to_string(self)?)
    }

    fn from_json(json: &str) -> Result<Self, Box<dyn Error + Send + Sync>> {
        Ok(serde_json::from_str(json)?)
    }
}
