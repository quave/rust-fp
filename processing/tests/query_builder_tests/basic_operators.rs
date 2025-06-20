use processing::ui_model::*;
use super::{get_query_string};

#[tokio::test]
async fn test_simple_select_without_filters() {
    let request = FilterRequest {
        filter: None,
        sort: vec![],
        limit: None,
        offset: None,
    };
    
    let query = get_query_string(request);
    assert_eq!(query, "SELECT t.id FROM orders t");
}

#[tokio::test]
async fn test_equal_operator_with_string() {
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
            groups: vec![],
        }),
        sort: vec![],
        limit: None,
        offset: None,
    };
    
    let query = get_query_string(request);
    assert_eq!(query, "SELECT t.id FROM orders t WHERE t.status = $1");
}

#[tokio::test]
async fn test_not_equal_operator_with_number() {
    let request = FilterRequest {
        filter: Some(FilterGroup {
            operator: LogicalOperator::And,
            conditions: vec![
                FilterCondition {
                    column: "amount".to_string(),
                    operator: Operator::NotEqual,
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
    assert_eq!(query, "SELECT t.id FROM orders t WHERE t.amount != $1");
}

#[tokio::test]
async fn test_greater_than_operator() {
    let request = FilterRequest {
        filter: Some(FilterGroup {
            operator: LogicalOperator::And,
            conditions: vec![
                FilterCondition {
                    column: "amount".to_string(),
                    operator: Operator::GreaterThan,
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
    assert_eq!(query, "SELECT t.id FROM orders t WHERE t.amount > $1");
}

#[tokio::test]
async fn test_greater_than_or_equal_operator() {
    let request = FilterRequest {
        filter: Some(FilterGroup {
            operator: LogicalOperator::And,
            conditions: vec![
                FilterCondition {
                    column: "amount".to_string(),
                    operator: Operator::GreaterThanOrEqual,
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
    assert_eq!(query, "SELECT t.id FROM orders t WHERE t.amount >= $1");
}

#[tokio::test]
async fn test_less_than_operator() {
    let request = FilterRequest {
        filter: Some(FilterGroup {
            operator: LogicalOperator::And,
            conditions: vec![
                FilterCondition {
                    column: "amount".to_string(),
                    operator: Operator::LessThan,
                    value: FilterValue::Number(200.0),
                },
            ],
            groups: vec![],
        }),
        sort: vec![],
        limit: None,
        offset: None,
    };
    
    let query = get_query_string(request);
    assert_eq!(query, "SELECT t.id FROM orders t WHERE t.amount < $1");
}

#[tokio::test]
async fn test_less_than_or_equal_operator() {
    let request = FilterRequest {
        filter: Some(FilterGroup {
            operator: LogicalOperator::And,
            conditions: vec![
                FilterCondition {
                    column: "amount".to_string(),
                    operator: Operator::LessThanOrEqual,
                    value: FilterValue::Number(150.0),
                },
            ],
            groups: vec![],
        }),
        sort: vec![],
        limit: None,
        offset: None,
    };
    
    let query = get_query_string(request);
    assert_eq!(query, "SELECT t.id FROM orders t WHERE t.amount <= $1");
}

#[tokio::test]
async fn test_like_operator() {
    let request = FilterRequest {
        filter: Some(FilterGroup {
            operator: LogicalOperator::And,
            conditions: vec![
                FilterCondition {
                    column: "status".to_string(),
                    operator: Operator::Like,
                    value: FilterValue::String("pending%".to_string()),
                },
            ],
            groups: vec![],
        }),
        sort: vec![],
        limit: None,
        offset: None,
    };
    
    let query = get_query_string(request);
    assert_eq!(query, "SELECT t.id FROM orders t WHERE t.status LIKE $1");
}

#[tokio::test]
async fn test_is_null_operator() {
    let request = FilterRequest {
        filter: Some(FilterGroup {
            operator: LogicalOperator::And,
            conditions: vec![
                FilterCondition {
                    column: "status".to_string(),
                    operator: Operator::IsNull,
                    value: FilterValue::Null,
                },
            ],
            groups: vec![],
        }),
        sort: vec![],
        limit: None,
        offset: None,
    };
    
    let query = get_query_string(request);
    assert_eq!(query, "SELECT t.id FROM orders t WHERE t.status IS NULL");
}

#[tokio::test]
async fn test_is_not_null_operator() {
    let request = FilterRequest {
        filter: Some(FilterGroup {
            operator: LogicalOperator::And,
            conditions: vec![
                FilterCondition {
                    column: "status".to_string(),
                    operator: Operator::IsNotNull,
                    value: FilterValue::Null,
                },
            ],
            groups: vec![],
        }),
        sort: vec![],
        limit: None,
        offset: None,
    };
    
    let query = get_query_string(request);
    assert_eq!(query, "SELECT t.id FROM orders t WHERE t.status IS NOT NULL");
}

#[tokio::test]
async fn test_boolean_filter() {
    let request = FilterRequest {
        filter: Some(FilterGroup {
            operator: LogicalOperator::And,
            conditions: vec![
                FilterCondition {
                    column: "customer.is_active".to_string(),
                    operator: Operator::Equal,
                    value: FilterValue::Boolean(true),
                },
            ],
            groups: vec![],
        }),
        sort: vec![],
        limit: None,
        offset: None,
    };
    
    let query = get_query_string(request);
    assert_eq!(query, "SELECT t.id FROM orders t LEFT JOIN customers customers_1 ON t.customer_id = customers_1.id WHERE customers_1.is_active = $1");
} 