pub mod expression_based;

use std::error::Error;

pub use expression_based::*;

use crate::model::{Feature, ModelId, ScoringModelType};
use async_trait::async_trait;

#[async_trait]
pub trait Scorer {
    async fn score_and_save_result(
        &self,
        transaction_id: ModelId,
        activation_id: ModelId,
        features: Vec<Feature>,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
    fn scorer_type(&self) -> ScoringModelType;
    fn channel_id(&self) -> ModelId;
}
