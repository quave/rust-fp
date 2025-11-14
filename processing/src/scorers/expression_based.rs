use async_trait::async_trait;
use evalexpr::*;
use crate::{
    model::{ScorerResult, Feature},
    scorers::Scorer,
};

#[derive(Debug, Clone)]
pub struct ExpressionRule {
    pub name: String,
    pub expression: String,
    pub score: i32
}

pub struct ExpressionBasedScorer {
    expressions: Vec<ExpressionRule>,
}

impl ExpressionBasedScorer {
    pub fn new() -> Self {
        Self { expressions: Vec::new() }
    }

    pub fn new_with_expressions(expressions: Vec<ExpressionRule>) -> Self {
        Self { expressions }
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
impl Scorer for ExpressionBasedScorer {
    async fn score(&self, features: Vec<Feature>) -> Vec<ScorerResult> {
        let context = self.setup_context(&features);

        self.expressions.iter().filter_map(|expression| {
            // Evaluate the expression
            match eval_with_context(expression.expression.as_str(), &context) {
                Ok(value) => {
                    match value {
                        Value::Boolean(true) => Some(expression.score),
                        _ => None,
                    }.map(|score| ScorerResult {
                        name: expression.name.clone(),
                        score,
                    })
                },
                Err(e) => {
                    #[cfg(test)]
                    println!("  Error evaluating expression '{}': {} {}", expression.name, expression.expression, e);
                    #[cfg(not(test))]
                    tracing::warn!("Error evaluating expression '{}': {} {}", expression.name, expression.expression, e);
                    None
                }
            }
        }).collect()
    }
}
