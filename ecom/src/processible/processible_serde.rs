use std::error::Error;

use async_trait::async_trait;
use processing::model::{ProcessibleSerde, processible::ColumnFilter};

use crate::model::{EcomOrder};

#[async_trait]
impl ProcessibleSerde for EcomOrder {
    fn as_json(&self) -> Result<serde_json::Value, Box<dyn Error + Send + Sync>> {
        Ok(serde_json::to_value(self)?)
    }

    fn from_json(json: serde_json::Value) -> Result<Self, Box<dyn Error + Send + Sync>> {
        Ok(serde_json::from_value(json)?)
    }

    fn list_column_fields() -> Vec<ColumnFilter<Self>> {
        crate::processible::columns::COLUMNS.clone()
    }
}
