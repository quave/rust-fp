pub mod expression_based;

pub use expression_based::*;

use crate::model::{ScorerResult, Feature};
use async_trait::async_trait;

#[async_trait]
pub trait Scorer {
    async fn score(&self, features: Vec<Feature>) -> Vec<ScorerResult>;
}
