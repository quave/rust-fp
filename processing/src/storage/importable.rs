use crate::model::{Importable, ModelId};
use async_trait::async_trait;
use std::error::Error;

#[async_trait]
pub trait ImportableStorage<I: Importable>: Send + Sync {
    async fn save_transaction(
        &self,
        tx_data: &I,
    ) -> Result<ModelId, Box<dyn Error + Send + Sync>>;
} 