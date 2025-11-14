use std::sync::Arc;
use async_graphql::dynamic::{Field, FieldFuture, Object};
use crate::model::ProcessibleSerde;
use super::types::{ColumnTypeIndex, scalar_to_typeref};

pub fn build_payload_and_types<P: ProcessibleSerde>() -> (Object, ColumnTypeIndex) {
    let mut payload = Object::new("Payload").description("Customer provided data.");
    let columns_list = <P as ProcessibleSerde>::list_column_fields();

    let mut column_types: ColumnTypeIndex = ColumnTypeIndex::new();

    for field in columns_list {
        payload = payload.field(
            Field::new(&field.column, scalar_to_typeref(&field.scalar), move |ctx| {
                let resolver = Arc::clone(&field.resolver);
                FieldFuture::new(async move {
                    let payload = ctx.parent_value.try_downcast_ref::<P>()
                        .expect("Failed to cast payload to P in graphql schema.");
                    let value = resolver(&payload);
                    Ok(Some(value))
                })
            })
            .description(&field.help_text)
        );

        if field.filter_statement.is_some() {
            column_types.insert(field.column.clone(), field.scalar.clone());
        }
    }

    (payload, column_types)
}


