use std::error::Error;

use crate::{    
    model::{ExpressionRule, Feature, ScoringResult, mongo_model::ScoringChannel}, scorers::Scorer
};
use async_trait::async_trait;
use evalexpr::*;

pub struct ExpressionBasedScorer 
{
    activation: ScoringChannel,
}

impl ExpressionBasedScorer {
    pub fn new(activation: ScoringChannel) -> Self { Self { activation } }

    fn setup_context(&self, simple_features: &[Feature], graph_features: &[Feature]) -> HashMapContext {
        let mut context = HashMapContext::new();

        // Add features to context
        for feature in [simple_features, graph_features].concat() {
            // Clone the feature value for the context
            let value_clone = (*feature.value).clone();

            #[cfg(test)]
            println!("Setting feature: {} = {:?}", feature.name, value_clone);

            // Set the value in the context
            if let Err(e) = context.set_value(feature.name.clone(), value_clone.into()) {
                #[cfg(test)]
                println!("Error setting feature {}: {}", feature.name, e);
                #[cfg(not(test))]
                tracing::error!("Error setting feature {}: {}", feature.name, e);
            }
        }

        // Debugging output for test mode
        #[cfg(test)]
        {
            println!("Context variables:");
            for var in context.iter_variables() {
                println!("  {} = {:?}", var.0, var.1);
            }
        }

        context
    }
}

#[async_trait]
impl Scorer for ExpressionBasedScorer {
    fn channel(&self) -> ScoringChannel {
        self.activation.clone()
    }

    async fn score(
        &self,
        simple_features: &[Feature],
        graph_features: &[Feature],
    ) -> Result<Box<dyn ScoringResult>, Box<dyn Error + Send + Sync>> {
        let context = self.setup_context(&simple_features, &graph_features);

        let triggered_rules = self
            .activation
            .model
            .expression_rules
            .iter()
            .filter_map(|expression| {
                // Evaluate the expression
                match eval_with_context(&expression.rule.as_str(), &context) {
                    Ok(value) => match value {
                        Value::Boolean(true) => Some(expression.clone()),
                        _ => None,
                    },
                    Err(e) => {
                        #[cfg(test)]
                        println!(
                            "  Error evaluating expression '{}': {} {}",
                            expression.name, expression.rule, e
                        );
                        #[cfg(not(test))]
                        tracing::error!(
                            "Error evaluating expression '{}': {} {}",
                            expression.name,
                            expression.rule,
                            e
                        );
                        None
                    }
                }
            }).collect::<Vec<ExpressionRule>>();
        
        Ok(Box::new(triggered_rules))
    }
}
