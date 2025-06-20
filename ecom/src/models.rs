use chrono::{serde::ts_seconds, DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: i64,
    pub transaction_id: i64,
    pub order_number: Option<String>,
    pub delivery_type: String,
    pub delivery_details: String,
    #[serde(with = "ts_seconds")]
    pub created_at: DateTime<Utc>,
} 