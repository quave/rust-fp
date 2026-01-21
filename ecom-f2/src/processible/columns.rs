use std::sync::Arc;

use async_graphql::Value;
use chrono::{DateTime, NaiveDate, Utc};
use processing::{
    graphql::types::scalar_base_type_name,
    model::processible::{ColumnFilter, ColumnScalar, ColumnValueTrait, Filter},
};
use serde::Serialize;

use crate::model::{Address, EcomF2Order, enum_name};

lazy_static::lazy_static! {
    pub(crate) static ref COLUMNS: Vec<ColumnFilter<EcomF2Order>> = {
        let mut columns = Vec::new();
        columns.extend(base_columns());
        columns.extend(billing_columns());
        columns.extend(customer_account_columns());
        columns.extend(payment_columns());
        columns.extend(address_columns(
            "billing_address",
            "Billing address",
            select_billing_address,
            "payload->'billingIdentity'->'address'",
        ));
        columns.extend(address_columns(
            "shipment_address",
            "Shipment address",
            select_shipment_address,
            "(payload->'deviatingShipmentIdentity'->'addresses'->0)",
        ));
        columns
    };
}

fn base_columns() -> Vec<ColumnFilter<EcomF2Order>> {
    vec![
        make_column(
            "created",
            "Order creation timestamp",
            ColumnScalar::String,
            |order| Value::from(order.created().to_rfc3339()),
            Some("(payload->'order'->>'date')::timestamptz".to_string()),
        ),
        make_column(
            "order_number",
            "Order number identifier",
            ColumnScalar::String,
            |order| Value::from(order.order_number()),
            Some("payload->'order'->>'sourceId'".to_string()),
        ),
        make_column(
            "state",
            "Order lifecycle state",
            ColumnScalar::String,
            |order| {
                let state = order.state();
                opt_enum_value(state.as_ref())
            },
            Some("payload->'shipment'->>'state'".to_string()),
        ),
        make_column(
            "shipment_type",
            "Shipment type",
            ColumnScalar::String,
            |order| {
                let shipment_type = order.shipment_type();
                opt_enum_value(shipment_type.as_ref())
            },
            Some("payload->'shipment'->>'type'".to_string()),
        ),
        make_column(
            "origin",
            "Order origin channel",
            ColumnScalar::String,
            |order| {
                let origin = order.origin();
                opt_enum_value(origin.as_ref())
            },
            Some("payload->'order'->>'channel'".to_string()),
        ),
        make_column(
            "checkout_time",
            "Checkout duration in seconds",
            ColumnScalar::Int,
            |order| {
                let checkout_time = order.checkout_time();
                opt_i64_value(checkout_time)
            },
            Some(
                "EXTRACT(EPOCH FROM ((payload->>'eventDate')::timestamptz - \
                 (payload->'order'->>'date')::timestamptz))"
                    .to_string(),
            ),
        ),
        make_column(
            "referrer",
            "Marketing referrer",
            ColumnScalar::String,
            |order| {
                let referrer = order.referrer();
                opt_string_value(referrer.as_ref())
            },
            Some("payload->'order'->>'channelDetail'".to_string()),
        ),
        make_column(
            "report_date",
            "Reporting timestamp",
            ColumnScalar::String,
            |order| {
                let report_date = order.report_date();
                opt_datetime_value(report_date.as_ref())
            },
            Some("(payload->>'eventDate')::timestamptz".to_string()),
        ),
        make_column(
            "device_ident_site",
            "Device identification site",
            ColumnScalar::String,
            |order| {
                let site = order.device_ident_site();
                opt_string_value(site.as_ref())
            },
            Some("payload->'deviceData'->>'smartId'".to_string()),
        ),
        make_column(
            "device_ident_token",
            "Device identification token",
            ColumnScalar::String,
            |order| {
                let token = order.device_ident_token();
                opt_string_value(token.as_ref())
            },
            Some("payload->'deviceData'->>'exactId'".to_string()),
        ),
    ]
}

fn billing_columns() -> Vec<ColumnFilter<EcomF2Order>> {
    vec![
        make_column(
            "billing_first_name",
            "Billing first name",
            ColumnScalar::String,
            |order| opt_string_value(order.billing_identity.first_name.as_ref()),
            Some("payload->'billingIdentity'->>'firstName'".to_string()),
        ),
        make_column(
            "billing_last_name",
            "Billing last name",
            ColumnScalar::String,
            |order| opt_string_value(order.billing_identity.last_name.as_ref()),
            Some("payload->'billingIdentity'->>'lastName'".to_string()),
        ),
        make_column(
            "billing_date_of_birth",
            "Billing date of birth",
            ColumnScalar::String,
            |order| opt_date_value(order.billing_identity.date_of_birth.as_ref()),
            Some("payload->'billingIdentity'->>'dateOfBirth'".to_string()),
        ),
        make_column(
            "billing_email",
            "Billing email",
            ColumnScalar::String,
            |order| {
                opt_string_value(
                    order
                        .billing_identity
                        .email_address
                        .as_ref()
                        .and_then(|addr| addr.email.as_ref()),
                )
            },
            Some("payload->'billingIdentity'->'emailAddress'->>'email'".to_string()),
        ),
        make_column(
            "billing_phone_numbers",
            "Billing phone numbers",
            ColumnScalar::List(Box::new(ColumnScalar::String)),
            |order| list_string_value(&order.customer_phone_numbers()),
            None,
        ),
    ]
}

fn customer_account_columns() -> Vec<ColumnFilter<EcomF2Order>> {
    vec![
        make_column(
            "customer_number",
            "Customer number",
            ColumnScalar::String,
            |order| {
                let value = order
                    .customer_account
                    .as_ref()
                    .and_then(|account| account.source_id.as_ref())
                    .cloned()
                    .or_else(|| order.billing_identity.source_id.clone())
                    .unwrap_or_default();
                Value::from(value)
            },
            Some("payload->'customerAccount'->>'sourceId'".to_string()),
        ),
        make_column(
            "customer_account_created",
            "Customer account creation timestamp",
            ColumnScalar::String,
            |order| {
                let datetime = order
                    .customer_account
                    .as_ref()
                    .and_then(|account| account.created_date.as_ref());
                opt_datetime_value(datetime)
            },
            Some("(payload->'customerAccount'->>'createdDate')::timestamptz".to_string()),
        ),
        make_column(
            "customer_type",
            "Customer type",
            ColumnScalar::String,
            |order| {
                order
                    .customer_account
                    .as_ref()
                    .and_then(|account| account.account_type.as_ref())
                    .cloned()
                    .map(Value::from)
                    .unwrap_or(Value::Null)
            },
            Some("payload->'customerAccount'->>'type'".to_string()),
        ),
        make_column(
            "customer_number_of_orders",
            "Number of past orders",
            ColumnScalar::Int,
            |order| {
                opt_i64_value(
                    order
                        .customer_account
                        .as_ref()
                        .and_then(|account| account.number_of_past_orders),
                )
            },
            Some("(payload->'customerAccount'->>'numberOfPastOrders')::bigint".to_string()),
        ),
        make_column(
            "customer_open_balance",
            "Open balance",
            ColumnScalar::String,
            |order| {
                order
                    .customer_account
                    .as_ref()
                    .and_then(|account| account.open_balance.as_ref())
                    .cloned()
                    .map(Value::from)
                    .unwrap_or(Value::Null)
            },
            Some("payload->'customerAccount'->>'openBalance'".to_string()),
        ),
    ]
}

fn payment_columns() -> Vec<ColumnFilter<EcomF2Order>> {
    vec![
        make_column(
            "payment_method",
            "Payment method",
            ColumnScalar::String,
            |order| {
                order
                    .billing_identity
                    .payment_details
                    .as_ref()
                    .and_then(|details| details.method.as_ref())
                    .and_then(enum_name)
                    .map(Value::from)
                    .unwrap_or(Value::Null)
            },
            Some("payload->'billingIdentity'->'paymentDetails'->>'method'".to_string()),
        ),
        make_column(
            "payment_detail_type",
            "Payment detail type",
            ColumnScalar::String,
            |order| {
                order
                    .billing_identity
                    .payment_details
                    .as_ref()
                    .and_then(|details| details.kind.as_ref())
                    .and_then(enum_name)
                    .map(Value::from)
                    .unwrap_or(Value::Null)
            },
            Some("payload->'billingIdentity'->'paymentDetails'->>'type'".to_string()),
        ),
    ]
}

fn address_columns(
    prefix: &str,
    help_prefix: &str,
    selector: fn(&EcomF2Order) -> Option<&Address>,
    json_root: &str,
) -> Vec<ColumnFilter<EcomF2Order>> {
    let root = format!("({json_root})");
    vec![
        address_column(
            &format!("{}_street", prefix),
            &format!("{} street", help_prefix),
            ColumnScalar::String,
            selector,
            |address| opt_string_value(address.street.as_ref()),
            Some(format!("{root}->>'street'")),
        ),
        address_column(
            &format!("{}_house_number", prefix),
            &format!("{} house number", help_prefix),
            ColumnScalar::String,
            selector,
            |address| opt_string_value(address.house_number.as_ref()),
            Some(format!("{root}->>'houseNumber'")),
        ),
        address_column(
            &format!("{}_city", prefix),
            &format!("{} city", help_prefix),
            ColumnScalar::String,
            selector,
            |address| opt_string_value(address.city.as_ref()),
            Some(format!("{root}->>'city'")),
        ),
        address_column(
            &format!("{}_postal_code", prefix),
            &format!("{} postal code", help_prefix),
            ColumnScalar::String,
            selector,
            |address| opt_string_value(address.postal_code.as_ref()),
            Some(format!("{root}->>'postalCode'")),
        ),
        address_column(
            &format!("{}_country", prefix),
            &format!("{} country", help_prefix),
            ColumnScalar::String,
            selector,
            |address| opt_enum_value(address.country.as_ref()),
            Some(format!("{root}->>'country'")),
        ),
    ]
}

fn make_column<F>(
    column: &str,
    help: &str,
    scalar: ColumnScalar,
    resolver: F,
    sql_expr: Option<String>,
) -> ColumnFilter<EcomF2Order>
where
    F: Fn(&EcomF2Order) -> Value + Send + Sync + 'static,
{
    ColumnFilter {
        column: column.to_string(),
        help_text: help.to_string(),
        scalar: scalar.clone(),
        resolver: Arc::new(resolver),
        filter_statement: sql_expr.map(
            |expr| -> Arc<dyn Fn(&Filter<Box<dyn ColumnValueTrait>>) -> String + Send + Sync> {
                let scalar = scalar.clone();
                Arc::new(move |filter: &Filter<Box<dyn ColumnValueTrait>>| {
                    format!(
                        "{} {}",
                        expr,
                        filter
                            .operator_value
                            .to_plain_statement(scalar_base_type_name(&scalar)),
                    )
                })
            },
        ),
    }
}

fn address_column<F>(
    column: &str,
    help: &str,
    scalar: ColumnScalar,
    selector: fn(&EcomF2Order) -> Option<&Address>,
    value_fn: F,
    sql_expr: Option<String>,
) -> ColumnFilter<EcomF2Order>
where
    F: Fn(&Address) -> Value + Send + Sync + 'static,
{
    make_column(
        column,
        help,
        scalar,
        move |order| {
            selector(order)
                .map(|address| value_fn(address))
                .unwrap_or(Value::Null)
        },
        sql_expr,
    )
}

fn select_billing_address(order: &EcomF2Order) -> Option<&Address> {
    order.billing_identity.address.as_ref()
}

fn select_shipment_address(order: &EcomF2Order) -> Option<&Address> {
    order
        .deviating_shipment_identity
        .iter()
        .flat_map(|shipment| shipment.address.iter())
        .next()
}

fn opt_string_value(value: Option<&String>) -> Value {
    value.cloned().map(Value::from).unwrap_or(Value::Null)
}

fn opt_i64_value(value: Option<i64>) -> Value {
    value.map(Value::from).unwrap_or(Value::Null)
}

fn opt_datetime_value(value: Option<&DateTime<Utc>>) -> Value {
    value
        .map(|dt| Value::from(dt.to_rfc3339()))
        .unwrap_or(Value::Null)
}

fn opt_date_value(value: Option<&NaiveDate>) -> Value {
    value
        .map(|d| Value::from(d.to_string()))
        .unwrap_or(Value::Null)
}

fn opt_enum_value<T>(value: Option<&T>) -> Value
where
    T: Serialize,
{
    value
        .and_then(enum_name)
        .map(Value::from)
        .unwrap_or(Value::Null)
}

fn list_string_value(values: &[String]) -> Value {
    if values.is_empty() {
        Value::Null
    } else {
        Value::List(values.iter().cloned().map(Value::from).collect())
    }
}
