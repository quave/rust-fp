use std::{error::Error, sync::Arc};

use crate::{
    model::{
        Feature, ModelId, ScoringModelType,
        sea_orm_storage_model::expression_rule::Model as ExpressionRule,
    },
    scorers::Scorer,
    storage::CommonStorage,
};
use async_trait::async_trait;
use evalexpr::*;

pub struct ExpressionBasedScorer<S: CommonStorage> {
    expression_rules: Vec<ExpressionRule>,
    channel_id: ModelId,
    #[allow(dead_code)]
    channel_name: String,
    storage: Arc<S>,
}

impl<S: CommonStorage> ExpressionBasedScorer<S> {
    pub async fn new_init(
        channel_name: String,
        storage: Arc<S>,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let channel = storage
            .get_channel_by_name(&channel_name)
            .await?
            .ok_or_else(|| format!("Channel not found by name: {}", channel_name))?;
        let expression_rules = storage.get_expression_rules(channel.model_id).await?;
        Ok(Self {
            expression_rules,
            channel_id: channel.id,
            channel_name,
            storage,
        })
    }

    fn setup_context(&self, features: &[Feature]) -> HashMapContext {
        let mut context = HashMapContext::new();

        // Add features to context
        for feature in features {
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
impl<S: CommonStorage> Scorer for ExpressionBasedScorer<S> {
    fn scorer_type(&self) -> ScoringModelType {
        ScoringModelType::RuleBased
    }
    fn channel_id(&self) -> ModelId {
        self.channel_id
    }
    // we could also expose channel_name if needed later

    async fn score_and_save_result(
        &self,
        transaction_id: ModelId,
        activation_id: ModelId,
        features: Vec<Feature>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let context = self.setup_context(&features);

        let triggered_rules = self.expression_rules.iter().filter_map(|expression| {
            // Evaluate the expression
            match eval_with_context(&expression.rule.as_str(), &context) {
                Ok(value) => match value {
                    Value::Boolean(true) => Some(expression),
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
        });

        let total_score: i32 = triggered_rules.clone().fold(0, |a, b| a + b.score);
        let triggered_rule_ids: Vec<ModelId> = triggered_rules.map(|rule| rule.id).collect();

        self.storage
            .save_scores(
                transaction_id,
                activation_id,
                total_score,
                &triggered_rule_ids,
            )
            .await?;

        Ok(())
    }
}
