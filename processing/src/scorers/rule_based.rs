use async_trait::async_trait;

use crate::{
    model::{Feature, ScorerResult},
    scorers::Scorer,
};

pub struct RuleBasedScorer {
    rules: Vec<Box<dyn Fn(&[Feature]) -> Option<ScorerResult> + Send + Sync + 'static>>,
}

impl RuleBasedScorer {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn add_rule(
        &mut self,
        rule: impl Fn(&[Feature]) -> Option<ScorerResult> + Send + Sync + 'static,
    ) {
        self.rules.push(Box::new(rule));
    }
}

#[async_trait]
impl Scorer for RuleBasedScorer {
    async fn score(&self, features: Vec<Feature>) -> Vec<ScorerResult> {
        self.rules
            .iter()
            .filter_map(|rule| rule(&features))
            .collect()
    }
}
