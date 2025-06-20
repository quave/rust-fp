use async_trait::async_trait;
use frida_core::{
    model::{Feature, FeatureValue, ScorerResult},
    scorer::Scorer,
};

pub struct RuleBasedScorer {
    rules: Vec<Box<dyn Fn(&[Feature]) -> Option<ScorerResult> + Send + Sync>>,
}

impl RuleBasedScorer {
    pub fn new() -> Self {
        let mut scorer = RuleBasedScorer { rules: Vec::new() };
        scorer.add_default_rules();
        scorer
    }

    fn add_default_rules(&mut self) {
        // Example rule: High total amount
        self.rules.push(Box::new(|features| {
            match features.iter().find(|f| f.name == "order_total") {
                Some(Feature { value, .. }) => {
                    if let FeatureValue::Double(total) = **value {
                        if total > 1000.0 {
                            return Some(ScorerResult {
                                score: 70.0,
                                name: "High order total".to_string(),
                            });
                        }
                    }
                    None
                }
                None => None,
            }
        }));

        // Add more rules...
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
