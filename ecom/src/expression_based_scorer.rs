use processing::scorers::{ExpressionBasedScorer, ExpressionRule};

pub fn get_expression_based_scorer() -> ExpressionBasedScorer {
    let expressions = vec![
        // High amount rule
        ExpressionRule {
            name: "High order total".to_string(),
            expression: "amount > 1000.0".to_string(),
            score: 10,
        },
        
        // Multiple item rule
        ExpressionRule {
            name: "Multiple items".to_string(),
            expression: "item_count > 3".to_string(),
            score: 11,
        },
        
        // New customer risk
        ExpressionRule {
            name: "New customer".to_string(),
            expression: "is_new_customer".to_string(),
            score: 12,
        },
        
        // High risk country
        ExpressionRule {
            name: "High risk country".to_string(),
            expression: "country_code == \"RU\" || country_code == \"BY\"".to_string(),
            score: 13,
        },
        
        // Unusual time of day
        ExpressionRule {
            name: "Late night order".to_string(),
            expression: "order_hour >= 22 || order_hour <= 4".to_string(),
            score: 14,
        },
    ];
    
    ExpressionBasedScorer::new_with_expressions(expressions)
}
