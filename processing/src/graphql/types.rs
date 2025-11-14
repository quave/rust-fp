use std::collections::HashMap;
use async_graphql::dynamic::TypeRef;
pub use crate::model::processible::ColumnScalar;

pub type ColumnTypeIndex = HashMap<String, ColumnScalar>;

pub fn scalar_to_typeref(s: &ColumnScalar) -> TypeRef {
    match s {
        ColumnScalar::String => TypeRef::named(TypeRef::STRING),
        ColumnScalar::Int => TypeRef::named(TypeRef::INT),
        ColumnScalar::Float => TypeRef::named(TypeRef::FLOAT),
        ColumnScalar::Boolean => TypeRef::named(TypeRef::BOOLEAN),
        ColumnScalar::List(inner) => {
            let nn_inner = scalar_base_type_name(inner);
            TypeRef::List(Box::new(TypeRef::named_nn(nn_inner)))
        }
    }
}

pub fn scalar_base_type_name(s: &ColumnScalar) -> &'static str {
    match s {
        ColumnScalar::String => TypeRef::STRING,
        ColumnScalar::Int => TypeRef::INT,
        ColumnScalar::Float => TypeRef::FLOAT,
        ColumnScalar::Boolean => TypeRef::BOOLEAN,
        ColumnScalar::List(inner) => scalar_base_type_name(inner),
    }
}

pub fn base_scalar_for_ops(s: &ColumnScalar) -> ColumnScalar {
    match s {
        ColumnScalar::List(inner) => base_scalar_for_ops(inner),
        ColumnScalar::String => ColumnScalar::String,
        ColumnScalar::Int => ColumnScalar::Int,
        ColumnScalar::Float => ColumnScalar::Float,
        ColumnScalar::Boolean => ColumnScalar::Boolean,
    }
}


