pub mod expression_based;

use std::error::Error;

pub use expression_based::*;

use crate::model::{Feature, ScoringResult, mongo_model::ScoringChannel};
use async_trait::async_trait;

#[async_trait]
pub trait Scorer: Send + Sync + 'static {
    fn channel(&self) -> ScoringChannel;
    async fn score(
        &self,
        simple_features: &[Feature],
        graph_features: &[Feature]
    ) -> Result<Box<dyn ScoringResult>, Box<dyn Error + Send + Sync>>;
}
