pub struct BaseRuleScorer {
    rules: Vec<Box<dyn Fn(&[Feature]) -> Option<ScorerResult> + Send + Sync>>,
}

impl BaseRuleScorer {
    pub fn add_rule(&mut self, rule: impl Fn(&[Feature]) -> Option<ScorerResult> + 'static) {
        self.rules.push(Box::new(rule));
    }
}
