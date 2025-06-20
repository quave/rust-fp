use processing::{
    model::{Feature, FeatureValue, ScorerResult},
    scorers::RuleBasedScorer,
};

pub fn get_rule_based_scorer() -> RuleBasedScorer {
    let mut scorer = RuleBasedScorer::new();

    // Example rule: High total amount
    scorer.add_rule(
        |features| match features.iter().find(|f| f.name == "amount") {
            Some(Feature { value, .. }) => {
                if let FeatureValue::Double(total) = **value {
                    if total > 1000.0 {
                        return Some(ScorerResult {
                            score: 70,
                            name: "High order total".to_string(),
                        });
                    }
                }
                None
            }
            None => None,
        },
    );

    // Add more rules...

    scorer
}
