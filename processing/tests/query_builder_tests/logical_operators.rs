use processing::ui_model::*;
use super::{get_query_string};

#[tokio::test]
async fn test_in_operator_with_strings() {
    let request = FilterRequest {
        filter: Some(FilterGroup {
            operator: LogicalOperator::And,
            conditions: vec![
                FilterCondition {
                    column: "status".to_string(),
                    operator: Operator::In,
                    value: FilterValue::StringList(vec![
                        "pending".to_string(),
                        "processing".to_string(),
                        "completed".to_string(),
                    ]),
                },
            ],
            groups: vec![],
        }),
        sort: vec![],
        limit: None,
        offset: None,
    };
    
    let query = get_query_string(request);
    assert_eq!(query, "SELECT t.id FROM orders t WHERE t.status IN ($1, $2, $3)");
}

#[tokio::test]
async fn test_in_operator_with_numbers() {
    let request = FilterRequest {
        filter: Some(FilterGroup {
            operator: LogicalOperator::And,
            conditions: vec![
                FilterCondition {
                    column: "amount".to_string(),
                    operator: Operator::In,
                    value: FilterValue::NumberList(vec![10.0, 20.0, 30.0]),
                },
            ],
            groups: vec![],
        }),
        sort: vec![],
        limit: None,
        offset: None,
    };
    
    let query = get_query_string(request);
    assert_eq!(query, "SELECT t.id FROM orders t WHERE t.amount IN ($1, $2, $3)");
}

#[tokio::test]
async fn test_not_in_operator() {
    let request = FilterRequest {
        filter: Some(FilterGroup {
            operator: LogicalOperator::And,
            conditions: vec![
                FilterCondition {
                    column: "status".to_string(),
                    operator: Operator::NotIn,
                    value: FilterValue::StringList(vec![
                        "cancelled".to_string(),
                        "refunded".to_string(),
                    ]),
                },
            ],
            groups: vec![],
        }),
        sort: vec![],
        limit: None,
        offset: None,
    };
    
    let query = get_query_string(request);
    assert_eq!(query, "SELECT t.id FROM orders t WHERE t.status NOT IN ($1, $2)");
}

#[tokio::test]
async fn test_between_operator() {
    let request = FilterRequest {
        filter: Some(FilterGroup {
            operator: LogicalOperator::And,
            conditions: vec![
                FilterCondition {
                    column: "amount".to_string(),
                    operator: Operator::Between,
                    value: FilterValue::Range(50.0, 200.0),
                },
            ],
            groups: vec![],
        }),
        sort: vec![],
        limit: None,
        offset: None,
    };
    
    let query = get_query_string(request);
    assert_eq!(query, "SELECT t.id FROM orders t WHERE t.amount BETWEEN $1 AND $2");
}

#[tokio::test]
async fn test_and_operator_with_multiple_conditions() {
    let request = FilterRequest {
        filter: Some(FilterGroup {
            operator: LogicalOperator::And,
            conditions: vec![
                FilterCondition {
                    column: "status".to_string(),
                    operator: Operator::Equal,
                    value: FilterValue::String("completed".to_string()),
                },
                FilterCondition {
                    column: "amount".to_string(),
                    operator: Operator::GreaterThan,
                    value: FilterValue::Number(100.0),
                },
            ],
            groups: vec![],
        }),
        sort: vec![],
        limit: None,
        offset: None,
    };
    
    let query = get_query_string(request);
    assert_eq!(query, "SELECT t.id FROM orders t WHERE t.status = $1 AND t.amount > $2");
}

#[tokio::test]
async fn test_or_operator_with_multiple_conditions() {
    let request = FilterRequest {
        filter: Some(FilterGroup {
            operator: LogicalOperator::Or,
            conditions: vec![
                FilterCondition {
                    column: "status".to_string(),
                    operator: Operator::Equal,
                    value: FilterValue::String("pending".to_string()),
                },
                FilterCondition {
                    column: "amount".to_string(),
                    operator: Operator::LessThan,
                    value: FilterValue::Number(50.0),
                },
            ],
            groups: vec![],
        }),
        sort: vec![],
        limit: None,
        offset: None,
    };
    
    let query = get_query_string(request);
    assert_eq!(query, "SELECT t.id FROM orders t WHERE t.status = $1 OR t.amount < $2");
}

#[tokio::test]
async fn test_nested_groups() {
    let request = FilterRequest {
        filter: Some(FilterGroup {
            operator: LogicalOperator::And,
            conditions: vec![
                FilterCondition {
                    column: "status".to_string(),
                    operator: Operator::Equal,
                    value: FilterValue::String("completed".to_string()),
                },
            ],
            groups: vec![
                FilterGroup {
                    operator: LogicalOperator::Or,
                    conditions: vec![
                        FilterCondition {
                            column: "amount".to_string(),
                            operator: Operator::GreaterThan,
                            value: FilterValue::Number(100.0),
                        },
                        FilterCondition {
                            column: "customer_id".to_string(),
                            operator: Operator::Equal,
                            value: FilterValue::Number(1.0),
                        },
                    ],
                    groups: vec![],
                },
            ],
        }),
        sort: vec![],
        limit: None,
        offset: None,
    };
    
    let query = get_query_string(request);
    assert_eq!(query, "SELECT t.id FROM orders t WHERE t.status = $1 AND (t.amount > $2 OR t.customer_id = $3)");
}

#[tokio::test]
async fn test_filter_with_array_and_range_values() {
    let request = FilterRequest {
        filter: Some(FilterGroup {
            operator: LogicalOperator::And,
            conditions: vec![
                FilterCondition {
                    column: "status".to_string(),
                    operator: Operator::In,
                    value: FilterValue::StringList(vec![
                        "pending".to_string(),
                        "processing".to_string(),
                    ]),
                },
                FilterCondition {
                    column: "amount".to_string(),
                    operator: Operator::Between,
                    value: FilterValue::Range(50.0, 500.0),
                },
                FilterCondition {
                    column: "customer_id".to_string(),
                    operator: Operator::In,
                    value: FilterValue::NumberList(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
                },
            ],
            groups: vec![
                FilterGroup {
                    operator: LogicalOperator::Or,
                    conditions: vec![
                        FilterCondition {
                            column: "created_at".to_string(),
                            operator: Operator::GreaterThan,
                            value: FilterValue::String("2023-01-01".to_string()),
                        },
                        FilterCondition {
                            column: "status".to_string(),
                            operator: Operator::Equal,
                            value: FilterValue::String("urgent".to_string()),
                        },
                    ],
                    groups: vec![],
                },
            ],
        }),
        sort: vec![],
        limit: None,
        offset: None,
    };
    
    let query = get_query_string(request);
    assert_eq!(query, "SELECT t.id FROM orders t WHERE t.status IN ($1, $2) AND t.amount BETWEEN $3 AND $4 AND t.customer_id IN ($5, $6, $7, $8, $9) AND (t.created_at > $10 OR t.status = $11)");
} 