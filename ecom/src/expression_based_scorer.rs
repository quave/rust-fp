use processing::scorers::ExpressionBasedScorer;

pub fn get_expression_based_scorer() -> ExpressionBasedScorer {
    let expressions = vec![
        // High amount rule
        ("High order total".to_string(), "amount > 1000.0".to_string()),
        
        // Multiple item rule
        ("Multiple items".to_string(), "item_count > 3".to_string()),
        
        // New customer risk
        ("New customer".to_string(), "is_new_customer".to_string()),
        
        // High risk country
        ("High risk country".to_string(), "country_code == \"RU\" || country_code == \"BY\"".to_string()),
        
        // Unusual time of day
        ("Late night order".to_string(), "order_hour >= 22 || order_hour <= 4".to_string()),
    ];
    
    ExpressionBasedScorer::new_with_expressions(expressions)
}
