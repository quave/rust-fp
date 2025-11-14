use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItem {
    pub name: String,
    pub category: String,
    pub price: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerData {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingData {
    pub payment_type: String,
    pub payment_details: String,
    pub billing_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcomOrder {
    pub order_number: String,
    pub created_at: DateTime<Utc>,
    pub items: Vec<OrderItem>,
    pub customer: CustomerData,
    pub billing: BillingData,
    pub delivery_type: String,
    pub delivery_details: String,
}