use crate::model::{ModelId, Processible};
use async_trait::async_trait;
use std::error::Error;

#[async_trait]
pub trait ProcessibleStorage<P: Processible>: Send + Sync {
    async fn get_processible(
        &self,
        transaction_id: ModelId,
    ) -> Result<P, Box<dyn Error + Send + Sync>>;
}