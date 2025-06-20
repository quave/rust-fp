use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use frida_core::model::Importable;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportOrderItem {
    pub name: String,
    pub category: String,
    pub price: f64,
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
impl Importable for ImportOrder {}
