use crate::model::{ModelId, Processible};
use async_trait::async_trait;
use std::error::Error;

#[async_trait]
pub trait ProcessibleStorage<P: Processible>: Send + Sync {
    async fn get_processible(
        &self,
        transaction_id: ModelId,
    ) -> Result<P, Box<dyn Error + Send + Sync>>;

    async fn set_transaction_id(
        &self,
        processible_id: ModelId,
        transaction_id: ModelId,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
}