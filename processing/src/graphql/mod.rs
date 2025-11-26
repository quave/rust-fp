use std::sync::Arc;

use async_graphql::{
    Value,
    dynamic::{
        Field, FieldFuture, FieldValue, InputObject, InputValue, Object, Schema, SchemaError,
        TypeRef,
    },
};
use metrics::histogram;
use std::time::Instant;

use crate::{
    model::{Processible, ProcessibleSerde, Transaction},
    storage::CommonStorage,
};

mod accessors;
mod filters;
mod inputs;
mod payload;
pub mod types;

pub use inputs::{
    FILTER_OPS_BOOL, FILTER_OPS_FLOAT, FILTER_OPS_INT, FILTER_OPS_STRING,
    build_typed_operator_inputs, operator_input_name_for,
};
pub use types::{ColumnScalar, ColumnTypeIndex};

lazy_static::lazy_static! {
    pub static ref LIST_FLOAT_TYPE: TypeRef = TypeRef::List(Box::new(TypeRef::named_nn(TypeRef::FLOAT)));
    pub static ref LIST_STRING_TYPE: TypeRef = TypeRef::List(Box::new(TypeRef::named_nn(TypeRef::STRING)));
}

fn to_transaction<'a>(parent_value: &'a FieldValue<'a>) -> &'a Transaction {
    parent_value
        .try_downcast_ref::<Transaction>()
        .expect("Failed to cast transaction in graphql schema.")
}

fn transaction_object<P: ProcessibleSerde>(payload_type_name: &str) -> Object {
    Object::new("Transaction")
        .description(
            "The transaction object, that contains customer provided data and all calculated data.",
        )
        .field(
            Field::new("id", TypeRef::named_nn(TypeRef::INT), |ctx| {
                FieldFuture::new(async move {
                    let tx = to_transaction(ctx.parent_value);
                    Ok(Some(Value::from(tx.id)))
                })
            })
            .description("The internal id of the transaction."),
        )
        .field(
            Field::new("payload", TypeRef::named_nn(payload_type_name), |ctx| {
                FieldFuture::new(async move {
                    let tx = to_transaction(ctx.parent_value);
                    let pl: P = <P as ProcessibleSerde>::from_json(tx.payload.clone())
                        .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                    Ok(Some(FieldValue::owned_any(pl)))
                })
            })
            .description("Customer provided data."),
        )
        .field(
            Field::new(
                "payload_number",
                TypeRef::named_nn(TypeRef::STRING),
                |ctx| {
                    FieldFuture::new(async move {
                        let tx = to_transaction(ctx.parent_value);
                        Ok(Some(Value::from(&tx.payload_number)))
                    })
                },
            )
            .description("Unique id of the customer provided data."),
        )
        .field(
            Field::new(
                "schema_version_major",
                TypeRef::named_nn(TypeRef::INT),
                |ctx| {
                    FieldFuture::new(async move {
                        let tx = to_transaction(ctx.parent_value);
                        Ok(Some(Value::from(tx.schema_version_major)))
                    })
                },
            )
            .description("The major version of the schema."),
        )
        .field(
            Field::new(
                "schema_version_minor",
                TypeRef::named_nn(TypeRef::INT),
                |ctx| {
                    FieldFuture::new(async move {
                        let tx = to_transaction(ctx.parent_value);
                        Ok(Some(Value::from(tx.schema_version_minor)))
                    })
                },
            )
            .description("The minor version of the schema."),
        )
        .field(
            Field::new("label_id", TypeRef::named(TypeRef::INT), |ctx| {
                FieldFuture::new(async move {
                    let tx = to_transaction(ctx.parent_value);
                    Ok(tx.label_id.map(Value::from))
                })
            })
            .description("The id of the label."),
        )
        .field(
            Field::new("comment", TypeRef::named(TypeRef::STRING), |ctx| {
                FieldFuture::new(async move {
                    let tx = to_transaction(ctx.parent_value);
                    Ok(tx.comment.as_ref().map(Value::from))
                })
            })
            .description("The comment of the transaction."),
        )
        .field(
            Field::new(
                "last_scoring_date",
                TypeRef::named(TypeRef::STRING),
                |ctx| {
                    FieldFuture::new(async move {
                        let tx = to_transaction(ctx.parent_value);
                        Ok(tx.last_scoring_date.map(|d| Value::from(d.to_string())))
                    })
                },
            )
            .description("The date of the last scoring."),
        )
        .field(
            Field::new(
                "processing_complete",
                TypeRef::named_nn(TypeRef::BOOLEAN),
                |ctx| {
                    FieldFuture::new(async move {
                        let tx = to_transaction(ctx.parent_value);
                        Ok(Some(Value::from(tx.processing_complete)))
                    })
                },
            )
            .description("The processing complete status."),
        )
        .field(
            Field::new("created_at", TypeRef::named_nn(TypeRef::STRING), |ctx| {
                FieldFuture::new(async move {
                    let tx = to_transaction(ctx.parent_value);
                    Ok(Some(Value::from(tx.created_at.to_string())))
                })
            })
            .description("The date of the creation."),
        )
}

pub fn schema<P: Processible + ProcessibleSerde + 'static>(
    common_storage: Arc<dyn CommonStorage>,
) -> Result<Schema, SchemaError> {
    let (payload, column_types) = payload::build_payload_and_types::<P>();

    let mut filters_input_object = InputObject::new("InputFilters");
    for (column, scalar) in column_types.iter() {
        let ops_type_name = inputs::operator_input_name_for(scalar.clone());
        filters_input_object =
            filters_input_object.field(InputValue::new(column, TypeRef::named(ops_type_name)));
    }

    let transaction = transaction_object::<P>(payload.type_name());

    let query = Object::new("Query")
        .description("The query object, that contains the transaction object and the payload object.")
        .field(
            Field::new("transaction", TypeRef::named_list(transaction.type_name()), move |ctx| {
                FieldFuture::new(async move {
                    let storage = ctx.data::<Arc<dyn CommonStorage>>()?;
                    let column_types = ctx.data::<Arc<ColumnTypeIndex>>()?;
                    let filters_object = ctx.args.try_get("filters")?.object()?;
                    let filters = filters::parse_filters::<P>(column_types, filters_object)?;

                    let t0 = Instant::now();
                    let txs = storage.filter_transactions(&filters).await?;
                    {
                        let h = histogram!("frida_backend_filter_seconds", "op" => "filter_transactions");
                        h.record(t0.elapsed().as_secs_f64());
                    }
                    let field_values_vec: Vec<FieldValue> = txs.into_iter().map(|tx| FieldValue::owned_any(tx)).collect();
                    Ok(Some(FieldValue::list(field_values_vec)))
                })
            })
            .argument(
                InputValue::new("filters", TypeRef::named(filters_input_object.type_name()))
            ),
        );

    let mut schema = Schema::build(query.type_name(), None, None)
        .register(transaction)
        .register(payload);

    for io in inputs::build_typed_operator_inputs() {
        schema = schema.register(io);
    }

    schema
        .register(filters_input_object)
        .register(query)
        .data(common_storage)
        .data(Arc::new(column_types))
        .finish()
}
