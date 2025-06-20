use std::error::Error;

use crate::model::{Feature, Importable, Processible, ScorerResult};
use async_trait::async_trait;

// Define the Storage trait
#[async_trait]
pub trait Storage<IT: Importable, T: Processible>: Sized + Send + Sync {
    async fn new(database_url: &str) -> Result<Self, Box<dyn Error + Send + Sync>>;
    async fn initialize_schema(&self) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn save_transaction(
        &self,
        transaction: &IT,
    ) -> Result<T::Id, Box<dyn Error + Send + Sync>>;
    async fn get_transaction(
        &self,
        transaction_id: &T::Id,
    ) -> Result<T, Box<dyn Error + Send + Sync>>;
    async fn save_features(
        &self,
        transaction_id: &T::Id,
        features: &[Feature],
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn save_scores(
        &self,
        transaction_id: &T::Id,
        scores: &Vec<ScorerResult>,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
}
