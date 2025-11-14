use std::sync::Arc;

use async_graphql::Value;
use processing::{model::processible::{ColumnFilter, ColumnScalar, FilterOperator}, graphql::types::scalar_base_type_name};

use crate::model::{EcomOrder};

lazy_static::lazy_static!{

    pub(crate) static ref COLUMNS: Vec<ColumnFilter<EcomOrder>> = vec![
        ColumnFilter {
            column: "order_number".to_string(),
            help_text: "The order number".to_string(),
            scalar: ColumnScalar::String,
            resolver: Arc::new(|order: &EcomOrder| Value::from(order.order_number.clone())),
            filter_statement: Some(Arc::new(|filter| format!(
                "payload->>'order_number' {}", 
                filter.operator_value.to_plain_statement(scalar_base_type_name(&ColumnScalar::String)),
            ))),
        },
        ColumnFilter {
            column: "created_at".to_string(),
            help_text: "The created at date".to_string(),
            scalar: ColumnScalar::String,
            resolver: Arc::new(|order: &EcomOrder| Value::from(order.created_at.to_string())),
            filter_statement: Some(Arc::new(|filter| format!(
                "payload->'created_at'::datetime {}", 
                filter.operator_value.to_plain_statement(scalar_base_type_name(&ColumnScalar::String)),
            ))),
        },
        ColumnFilter {
            column: "payment_type".to_string(),
            help_text: "The payment type".to_string(),
            scalar: ColumnScalar::String,
            resolver: Arc::new(|order: &EcomOrder| Value::from(order.billing.payment_type.clone())),
            filter_statement: Some(Arc::new(|filter| format!(
                "payload->'billing'->>'payment_type' {}", 
                filter.operator_value.to_plain_statement(scalar_base_type_name(&ColumnScalar::String)),
            ))),
        },
        ColumnFilter {
            column: "payment_details".to_string(),
            help_text: "The payment details".to_string(),
            scalar: ColumnScalar::String,
            resolver: Arc::new(|order: &EcomOrder| Value::from(order.billing.payment_details.clone())),
            filter_statement: Some(Arc::new(|filter| format!(
                "payload->'billing'->>'payment_details' {}", 
                filter.operator_value.to_plain_statement(scalar_base_type_name(&ColumnScalar::String)),
            ))),
        },
        ColumnFilter {
            column: "billing_address".to_string(),
            help_text: "The billing address".to_string(),
            scalar: ColumnScalar::String,
            resolver: Arc::new(|order: &EcomOrder| Value::from(order.billing.billing_address.clone())),
            filter_statement: Some(Arc::new(|filter| format!(
                "payload->'billing'->>'billing_address' {}", 
                filter.operator_value.to_plain_statement(scalar_base_type_name(&ColumnScalar::String)),
            ))),
        },
        ColumnFilter {
            column: "total_amount".to_string(),
            help_text: "The total amount".to_string(),
            scalar: ColumnScalar::Float,
            resolver: Arc::new(|order: &EcomOrder| Value::from(order.items.iter().map(|i| i.price).sum::<f32>())),
            filter_statement: None,
        },
        ColumnFilter {
            column: "item_prices".to_string(),
            help_text: "The list of item prices".to_string(),
            scalar: ColumnScalar::List(Box::new(ColumnScalar::Float)),
            resolver: Arc::new(|order: &EcomOrder| Value::List(order.items.iter().map(|i| Value::from(i.price)).collect())),
            filter_statement: Some(Arc::new(|filter| {
                let op = filter.operator_value.to_plain_statement(scalar_base_type_name(&ColumnScalar::Float));
                format!(
                    "EXISTS (SELECT 1 FROM jsonb_array_elements(payload->'items') elem WHERE (elem->>'price')::double precision {})",
                    op
                )
            })),
        },
        ColumnFilter {
            column: "item_names".to_string(),
            help_text: "The list of item names".to_string(),
            scalar: ColumnScalar::List(Box::new(ColumnScalar::String)),
            resolver: Arc::new(|order: &EcomOrder| Value::List(order.items.iter().map(|i| Value::from(i.name.clone())).collect())),
            filter_statement: Some(Arc::new(|filter| {
                let value_fragment = filter.operator_value.to_plain_statement(scalar_base_type_name(&ColumnScalar::String));
                match &filter.operator_value {
                    FilterOperator::Contains(_) => {
                        format!(
                            "EXISTS (SELECT 1 FROM jsonb_array_elements(payload->'items') elem WHERE (elem->>'name') LIKE {})",
                            value_fragment
                        )
                    }
                    _ => {
                        format!(
                            "EXISTS (SELECT 1 FROM jsonb_array_elements(payload->'items') elem WHERE (elem->>'name') {})",
                            value_fragment
                        )
                    }
                }
            })),
        },
        ColumnFilter {
            column: "customer_name".to_string(),
            help_text: "The customer name".to_string(),
            scalar: ColumnScalar::String,
            resolver: Arc::new(|order: &EcomOrder| Value::from(order.customer.name.clone())),
            filter_statement: Some(Arc::new(|filter| format!(
                "payload->'customer'->>'name' {}", 
                filter.operator_value.to_plain_statement(scalar_base_type_name(&ColumnScalar::String)),
            ))),
        },
        ColumnFilter {
            column: "customer_email".to_string(),
            help_text: "The customer email".to_string(),
            scalar: ColumnScalar::String,
            resolver: Arc::new(|order: &EcomOrder| Value::from(order.customer.email.clone())),
            filter_statement: Some(Arc::new(|filter| format!(
                "payload->'customer'->>'email' {}", 
                filter.operator_value.to_plain_statement(scalar_base_type_name(&ColumnScalar::String)),
            ))),
        },
    ];
}
