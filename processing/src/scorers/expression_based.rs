use async_trait::async_trait;
use evalexpr::*;
use crate::{
    model::{ScorerResult, Feature},
    scorers::Scorer,
};

pub struct ExpressionBasedScorer {
    expressions: Vec<(String, String)>,
}

impl ExpressionBasedScorer {
    pub fn new() -> Self {
        Self { expressions: Vec::new() }
    }

    pub fn new_with_expressions(expressions: Vec<(String, String)>) -> Self {
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
    
    // Convert an evalexpr Value to a score (100 for true, 0 for false)
    fn value_to_score(&self, value: Value) -> i32 {
        match value {
            Value::Boolean(true) => 100,
            Value::Boolean(false) => 0,
            _ => {
                // Non-boolean results are unexpected
                #[cfg(test)]
                println!("  Warning: Expression produced non-boolean result: {:?}", value);
                #[cfg(not(test))]
                tracing::warn!("Expression produced non-boolean result: {:?}", value);
                
                // Default to 0 for non-boolean results
                0
            }
        }
    }
}

#[async_trait]
impl Scorer for ExpressionBasedScorer {
    async fn score(&self, features: Vec<Feature>) -> Vec<ScorerResult> {
        let context = self.setup_context(&features);
        let mut results = Vec::new();
        
        for (name, expr) in &self.expressions {
            #[cfg(test)]
            println!("Evaluating expression: {} = {}", name, expr);
            
            // Evaluate the expression
            match eval_with_context(expr, &context) {
                Ok(value) => {
                    let score = self.value_to_score(value.clone());
                    
                    #[cfg(test)]
                    println!("  Result: {:?} -> score: {}", value, score);
                    
                    results.push(ScorerResult {
                        name: name.to_string(),
                        score,
                    });
                },
                Err(e) => {
                    #[cfg(test)]
                    println!("  Error evaluating expression '{}': {}", expr, e);
                    #[cfg(not(test))]
                    tracing::warn!("Error evaluating expression '{}': {}", expr, e);
                }
            }
        }
        
        results
    }
}
