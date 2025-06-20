use processing::ui_model::*;
use super::{get_query_string};

#[tokio::test]
async fn test_sorting() {
    let request = FilterRequest {
        filter: None,
        sort: vec![
            SortField {
                field: "created_at".to_string(),
                direction: SortDirection::Desc,
            },
            SortField {
                field: "amount".to_string(),
                direction: SortDirection::Asc,
            },
        ],
        limit: None,
        offset: None,
    };
    
    let query = get_query_string(request);
    assert_eq!(query, "SELECT t.id FROM orders t ORDER BY t.created_at DESC, t.amount ASC");
}

#[tokio::test]
async fn test_limit_and_offset() {
    let request = FilterRequest {
        filter: None,
        sort: vec![],
        limit: Some(10),
        offset: Some(20),
    };
    
    let query = get_query_string(request);
    assert_eq!(query, "SELECT t.id FROM orders t LIMIT 10 OFFSET 20");
}

#[tokio::test]
async fn test_complex_query() {
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
                    value: FilterValue::Range(100.0, 1000.0),
                },
            ],
            groups: vec![
                FilterGroup {
                    operator: LogicalOperator::Or,
                    conditions: vec![
                        FilterCondition {
                            column: "customer.name".to_string(),
                            operator: Operator::Like,
                            value: FilterValue::String("Premium%".to_string()),
                        },
                        FilterCondition {
                            column: "customer.email".to_string(),
                            operator: Operator::Like,
                            value: FilterValue::String("%@vip.com".to_string()),
                        },
                    ],
                    groups: vec![],
                },
            ],
        }),
        sort: vec![
            SortField {
                field: "created_at".to_string(),
                direction: SortDirection::Desc,
            },
            SortField {
                field: "amount".to_string(),
                direction: SortDirection::Asc,
            },
        ],
        limit: Some(25),
        offset: Some(50),
    };
    
    let query = get_query_string(request);
    assert_eq!(
        query,
        "SELECT t.id FROM orders t LEFT JOIN customers customers_1 ON t.customer_id = customers_1.id WHERE t.status IN ($1, $2) AND t.amount BETWEEN $3 AND $4 AND (customers_1.name LIKE $5 OR customers_1.email LIKE $6) ORDER BY t.created_at DESC, t.amount ASC LIMIT 25 OFFSET 50"
    );
}

#[tokio::test]
async fn test_sorting_with_relationships() {
    let request = FilterRequest {
        filter: None,
        sort: vec![
            SortField {
                field: "customer.name".to_string(),
                direction: SortDirection::Asc,
            },
            SortField {
                field: "items.price".to_string(),
                direction: SortDirection::Desc,
            },
        ],
        limit: None,
        offset: None,
    };
    
    let query = get_query_string(request);
    assert_eq!(query, "SELECT t.id FROM orders t LEFT JOIN customers customers_1 ON t.customer_id = customers_1.id LEFT JOIN order_items order_items_1 ON t.id = order_items_1.order_id ORDER BY customers_1.name ASC, order_items_1.price DESC");
}

#[tokio::test]
async fn test_complex_filter_with_sorting_and_pagination() {
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
                    column: "amount".to_string(),
                    operator: Operator::GreaterThan,
                    value: FilterValue::Number(500.0),
                },
            ],
            groups: vec![
                FilterGroup {
                    operator: LogicalOperator::Or,
                    conditions: vec![
                        FilterCondition {
                            column: "items.product_name".to_string(),
                            operator: Operator::Like,
                            value: FilterValue::String("Premium%".to_string()),
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
        sort: vec![
            SortField {
                field: "created_at".to_string(),
                direction: SortDirection::Desc,
            },
            SortField {
                field: "customer.name".to_string(),
                direction: SortDirection::Asc,
            },
        ],
        limit: Some(20),
        offset: Some(100),
    };
    
    let query = get_query_string(request);
    assert_eq!(
        query,
        "SELECT t.id FROM orders t LEFT JOIN customers customers_1 ON t.customer_id = customers_1.id LEFT JOIN order_items order_items_1 ON t.id = order_items_1.order_id WHERE customers_1.is_active = $1 AND t.amount > $2 AND (order_items_1.product_name LIKE $3 OR order_items_1.quantity >= $4) ORDER BY t.created_at DESC, customers_1.name ASC LIMIT 20 OFFSET 100"
    );
}

#[tokio::test]
async fn test_empty_filter_with_complex_sorting() {
    let request = FilterRequest {
        filter: None,
        sort: vec![
            SortField {
                field: "amount".to_string(),
                direction: SortDirection::Desc,
            },
            SortField {
                field: "customer.email".to_string(),
                direction: SortDirection::Asc,
            },
            SortField {
                field: "created_at".to_string(),
                direction: SortDirection::Desc,
            },
        ],
        limit: Some(15),
        offset: None,
    };
    
    let query = get_query_string(request);
    assert_eq!(query, "SELECT t.id FROM orders t LEFT JOIN customers customers_1 ON t.customer_id = customers_1.id ORDER BY t.amount DESC, customers_1.email ASC, t.created_at DESC LIMIT 15");
}

#[tokio::test]
async fn test_maximum_complexity_query() {
    let request = FilterRequest {
        filter: Some(FilterGroup {
            operator: LogicalOperator::And,
            conditions: vec![
                FilterCondition {
                    column: "status".to_string(),
                    operator: Operator::NotIn,
                    value: FilterValue::StringList(vec!["cancelled".to_string(), "refunded".to_string()]),
                },
                FilterCondition {
                    column: "customer.is_active".to_string(),
                    operator: Operator::Equal,
                    value: FilterValue::Boolean(true),
                },
            ],
            groups: vec![
                FilterGroup {
                    operator: LogicalOperator::Or,
                    conditions: vec![
                        FilterCondition {
                            column: "amount".to_string(),
                            operator: Operator::GreaterThan,
                            value: FilterValue::Number(1000.0),
                        },
                        FilterCondition {
                            column: "customer.email".to_string(),
                            operator: Operator::Like,
                            value: FilterValue::String("%@premium.com".to_string()),
                        },
                    ],
                    groups: vec![
                        FilterGroup {
                            operator: LogicalOperator::And,
                            conditions: vec![
                                FilterCondition {
                                    column: "items.product_name".to_string(),
                                    operator: Operator::In,
                                    value: FilterValue::StringList(vec![
                                        "iPhone Pro".to_string(),
                                        "MacBook Pro".to_string(),
                                        "iPad Pro".to_string(),
                                    ]),
                                },
                                FilterCondition {
                                    column: "items.quantity".to_string(),
                                    operator: Operator::Between,
                                    value: FilterValue::Range(2.0, 10.0),
                                },
                                FilterCondition {
                                    column: "items.price".to_string(),
                                    operator: Operator::GreaterThanOrEqual,
                                    value: FilterValue::Number(500.0),
                                },
                            ],
                            groups: vec![],
                        },
                    ],
                },
                FilterGroup {
                    operator: LogicalOperator::And,
                    conditions: vec![
                        FilterCondition {
                            column: "created_at".to_string(),
                            operator: Operator::GreaterThan,
                            value: FilterValue::String("2023-01-01".to_string()),
                        },
                        FilterCondition {
                            column: "customer.name".to_string(),
                            operator: Operator::IsNotNull,
                            value: FilterValue::Null,
                        },
                    ],
                    groups: vec![],
                },
            ],
        }),
        sort: vec![
            SortField {
                field: "amount".to_string(),
                direction: SortDirection::Desc,
            },
            SortField {
                field: "created_at".to_string(),
                direction: SortDirection::Desc,
            },
            SortField {
                field: "customer.name".to_string(),
                direction: SortDirection::Asc,
            },
        ],
        limit: Some(50),
        offset: Some(200),
    };
    
    let query = get_query_string(request);
    assert_eq!(
        query,
        "SELECT t.id FROM orders t LEFT JOIN customers customers_1 ON t.customer_id = customers_1.id LEFT JOIN order_items order_items_1 ON t.id = order_items_1.order_id WHERE t.status NOT IN ($1, $2) AND customers_1.is_active = $3 AND (t.amount > $4 OR customers_1.email LIKE $5 OR (order_items_1.product_name IN ($6, $7, $8) AND order_items_1.quantity BETWEEN $9 AND $10 AND order_items_1.price >= $11)) AND (t.created_at > $12 AND customers_1.name IS NOT NULL) ORDER BY t.amount DESC, t.created_at DESC, customers_1.name ASC LIMIT 50 OFFSET 200"
    );
} 