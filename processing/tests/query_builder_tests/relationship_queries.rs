use processing::ui_model::*;
use super::{get_query_string};

#[tokio::test]
async fn test_related_table_filter() {
    let request = FilterRequest {
        filter: Some(FilterGroup {
            operator: LogicalOperator::And,
            conditions: vec![
                FilterCondition {
                    column: "customer.name".to_string(),
                    operator: Operator::Equal,
                    value: FilterValue::String("John Doe".to_string()),
                },
            ],
            groups: vec![],
        }),
        sort: vec![],
        limit: None,
        offset: None,
    };
    
    let query = get_query_string(request);
    assert_eq!(query, "SELECT t.id FROM orders t LEFT JOIN customers customers_1 ON t.customer_id = customers_1.id WHERE customers_1.name = $1");
}

#[tokio::test]
async fn test_two_level_relation_filter() {
    let request = FilterRequest {
        filter: Some(FilterGroup {
            operator: LogicalOperator::And,
            conditions: vec![
                FilterCondition {
                    column: "items.product_name".to_string(),
                    operator: Operator::Like,
                    value: FilterValue::String("iPhone%".to_string()),
                },
            ],
            groups: vec![],
        }),
        sort: vec![],
        limit: None,
        offset: None,
    };
    
    let query = get_query_string(request);
    assert_eq!(query, "SELECT t.id FROM orders t LEFT JOIN order_items order_items_1 ON t.id = order_items_1.order_id WHERE order_items_1.product_name LIKE $1");
}

#[tokio::test]
async fn test_complex_multi_level_relation_filter() {
    let request = FilterRequest {
        filter: Some(FilterGroup {
            operator: LogicalOperator::And,
            conditions: vec![
                FilterCondition {
                    column: "customer.email".to_string(),
                    operator: Operator::Like,
                    value: FilterValue::String("%@company.com".to_string()),
                },
                FilterCondition {
                    column: "items.price".to_string(),
                    operator: Operator::GreaterThan,
                    value: FilterValue::Number(100.0),
                },
                FilterCondition {
                    column: "amount".to_string(),
                    operator: Operator::Between,
                    value: FilterValue::Range(500.0, 2000.0),
                },
            ],
            groups: vec![
                FilterGroup {
                    operator: LogicalOperator::Or,
                    conditions: vec![
                        FilterCondition {
                            column: "customer.name".to_string(),
                            operator: Operator::Equal,
                            value: FilterValue::String("VIP Customer".to_string()),
                        },
                        FilterCondition {
                            column: "items.quantity".to_string(),
                            operator: Operator::GreaterThanOrEqual,
                            value: FilterValue::Number(5.0),
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
    assert_eq!(
        query,
        "SELECT t.id FROM orders t LEFT JOIN customers customers_1 ON t.customer_id = customers_1.id LEFT JOIN order_items order_items_1 ON t.id = order_items_1.order_id WHERE customers_1.email LIKE $1 AND order_items_1.price > $2 AND t.amount BETWEEN $3 AND $4 AND (customers_1.name = $5 OR order_items_1.quantity >= $6)"
    );
}

#[tokio::test]
async fn test_nested_multi_model_filter() {
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
                            column: "customer.email".to_string(),
                            operator: Operator::Like,
                            value: FilterValue::String("%@premium.com".to_string()),
                        },
                        FilterCondition {
                            column: "amount".to_string(),
                            operator: Operator::GreaterThan,
                            value: FilterValue::Number(1000.0),
                        },
                    ],
                    groups: vec![
                        FilterGroup {
                            operator: LogicalOperator::And,
                            conditions: vec![
                                FilterCondition {
                                    column: "items.product_name".to_string(),
                                    operator: Operator::Like,
                                    value: FilterValue::String("Premium%".to_string()),
                                },
                                FilterCondition {
                                    column: "items.quantity".to_string(),
                                    operator: Operator::GreaterThanOrEqual,
                                    value: FilterValue::Number(2.0),
                                },
                            ],
                            groups: vec![],
                        },
                    ],
                },
            ],
        }),
        sort: vec![],
        limit: None,
        offset: None,
    };
    
    let query = get_query_string(request);
    assert_eq!(
        query,
        "SELECT t.id FROM orders t LEFT JOIN customers customers_1 ON t.customer_id = customers_1.id LEFT JOIN order_items order_items_1 ON t.id = order_items_1.order_id WHERE t.status = $1 AND (customers_1.email LIKE $2 OR t.amount > $3 OR (order_items_1.product_name LIKE $4 AND order_items_1.quantity >= $5))"
    );
}

#[tokio::test]
async fn test_consistent_alias_numbering() {
    let request = FilterRequest {
        filter: Some(FilterGroup {
            operator: LogicalOperator::And,
            conditions: vec![
                FilterCondition {
                    column: "customer.name".to_string(),
                    operator: Operator::Equal,
                    value: FilterValue::String("Test Customer".to_string()),
                },
                FilterCondition {
                    column: "items.product_name".to_string(),
                    operator: Operator::Like,
                    value: FilterValue::String("Product%".to_string()),
                },
                FilterCondition {
                    column: "customer.email".to_string(),
                    operator: Operator::Like,
                    value: FilterValue::String("%@test.com".to_string()),
                },
                FilterCondition {
                    column: "items.price".to_string(),
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
    assert_eq!(
        query,
        "SELECT t.id FROM orders t LEFT JOIN customers customers_1 ON t.customer_id = customers_1.id LEFT JOIN order_items order_items_1 ON t.id = order_items_1.order_id WHERE customers_1.name = $1 AND order_items_1.product_name LIKE $2 AND customers_1.email LIKE $3 AND order_items_1.price > $4"
    );
}

#[tokio::test]
async fn test_multi_model_filter_with_various_types() {
    let request = FilterRequest {
        filter: Some(FilterGroup {
            operator: LogicalOperator::And,
            conditions: vec![
                FilterCondition {
                    column: "customer.is_active".to_string(),
                    operator: Operator::Equal,
                    value: FilterValue::Boolean(true),
                },
                FilterCondition {
                    column: "items.quantity".to_string(),
                    operator: Operator::In,
                    value: FilterValue::NumberList(vec![1.0, 2.0, 3.0, 5.0, 10.0]),
                },
                FilterCondition {
                    column: "customer.name".to_string(),
                    operator: Operator::NotIn,
                    value: FilterValue::StringList(vec![
                        "Blacklisted User".to_string(),
                        "Suspended Account".to_string(),
                    ]),
                },
                FilterCondition {
                    column: "items.price".to_string(),
                    operator: Operator::Between,
                    value: FilterValue::Range(10.0, 1000.0),
                },
            ],
            groups: vec![
                FilterGroup {
                    operator: LogicalOperator::Or,
                    conditions: vec![
                        FilterCondition {
                            column: "customer.email".to_string(),
                            operator: Operator::Like,
                            value: FilterValue::String("%@vip.com".to_string()),
                        },
                        FilterCondition {
                            column: "amount".to_string(),
                            operator: Operator::GreaterThanOrEqual,
                            value: FilterValue::Number(500.0),
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
    assert_eq!(
        query,
        "SELECT t.id FROM orders t LEFT JOIN customers customers_1 ON t.customer_id = customers_1.id LEFT JOIN order_items order_items_1 ON t.id = order_items_1.order_id WHERE customers_1.is_active = $1 AND order_items_1.quantity IN ($2, $3, $4, $5, $6) AND customers_1.name NOT IN ($7, $8) AND order_items_1.price BETWEEN $9 AND $10 AND (customers_1.email LIKE $11 OR t.amount >= $12)"
    );
}

#[tokio::test]
async fn test_comparison_with_inner_join() {
    let request = FilterRequest {
        filter: Some(FilterGroup {
            operator: LogicalOperator::And,
            conditions: vec![
                FilterCondition {
                    column: "customer.name".to_string(),
                    operator: Operator::Equal,
                    value: FilterValue::String("John Smith".to_string()),
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
    assert_eq!(query, "SELECT t.id FROM orders t LEFT JOIN customers customers_1 ON t.customer_id = customers_1.id WHERE customers_1.name = $1 AND t.amount > $2");
} 