use super::types::ColumnScalar;
use super::types::base_scalar_for_ops;
use crate::model::processible::ColumnValueTrait;
use async_graphql::dynamic::{InputObject, InputValue, TypeRef};

pub const FILTER_OPS_STRING: &str = "FilterOperatorsStringInput";
pub const FILTER_OPS_INT: &str = "FilterOperatorsIntInput";
pub const FILTER_OPS_FLOAT: &str = "FilterOperatorsFloatInput";
pub const FILTER_OPS_BOOL: &str = "FilterOperatorsBooleanInput";

use crate::model::processible::FilterOperator;

pub fn build_typed_operator_inputs() -> Vec<InputObject> {
    // String operators: Equal, NotEqual, Contains, In, NotIn, IsNull, NotNull
    let mut string_ops = InputObject::new(FILTER_OPS_STRING)
        .description("String filter operators")
        .oneof();
    string_ops = string_ops
        .field(InputValue::new(
            FilterOperator::Equal(String::default()).to_string(),
            TypeRef::named(TypeRef::STRING),
        ))
        .field(InputValue::new(
            FilterOperator::NotEqual(String::default()).to_string(),
            TypeRef::named(TypeRef::STRING),
        ))
        .field(InputValue::new(
            FilterOperator::Contains(String::default()).to_string(),
            TypeRef::named(TypeRef::STRING),
        ))
        .field(InputValue::new(
            FilterOperator::<Box<dyn ColumnValueTrait>>::In(vec![]).to_string(),
            TypeRef::named_list(TypeRef::STRING),
        ))
        .field(InputValue::new(
            FilterOperator::<Box<dyn ColumnValueTrait>>::NotIn(vec![]).to_string(),
            TypeRef::named_list(TypeRef::STRING),
        ))
        .field(InputValue::new(
            FilterOperator::<Box<dyn ColumnValueTrait>>::IsNull.to_string(),
            TypeRef::named(TypeRef::BOOLEAN),
        ))
        .field(InputValue::new(
            FilterOperator::<Box<dyn ColumnValueTrait>>::NotNull.to_string(),
            TypeRef::named(TypeRef::BOOLEAN),
        ));

    // Int operators: equality, ordering, between, in/not-in, null checks
    let mut int_ops = InputObject::new(FILTER_OPS_INT)
        .description("Int filter operators")
        .oneof();
    int_ops = int_ops
        .field(InputValue::new(
            FilterOperator::Equal(String::default()).to_string(),
            TypeRef::named(TypeRef::INT),
        ))
        .field(InputValue::new(
            FilterOperator::NotEqual(String::default()).to_string(),
            TypeRef::named(TypeRef::INT),
        ))
        .field(InputValue::new(
            FilterOperator::GreaterThan(String::default()).to_string(),
            TypeRef::named(TypeRef::INT),
        ))
        .field(InputValue::new(
            FilterOperator::GreaterThanOrEqual(String::default()).to_string(),
            TypeRef::named(TypeRef::INT),
        ))
        .field(InputValue::new(
            FilterOperator::LessThan(String::default()).to_string(),
            TypeRef::named(TypeRef::INT),
        ))
        .field(InputValue::new(
            FilterOperator::LessThanOrEqual(String::default()).to_string(),
            TypeRef::named(TypeRef::INT),
        ))
        .field(InputValue::new(
            FilterOperator::Between(String::default(), String::default()).to_string(),
            TypeRef::named_list(TypeRef::INT),
        ))
        .field(InputValue::new(
            FilterOperator::<Box<dyn ColumnValueTrait>>::In(vec![]).to_string(),
            TypeRef::named_list(TypeRef::INT),
        ))
        .field(InputValue::new(
            FilterOperator::<Box<dyn ColumnValueTrait>>::NotIn(vec![]).to_string(),
            TypeRef::named_list(TypeRef::INT),
        ))
        .field(InputValue::new(
            FilterOperator::<Box<dyn ColumnValueTrait>>::IsNull.to_string(),
            TypeRef::named(TypeRef::BOOLEAN),
        ))
        .field(InputValue::new(
            FilterOperator::<Box<dyn ColumnValueTrait>>::NotNull.to_string(),
            TypeRef::named(TypeRef::BOOLEAN),
        ));

    // Float operators: equality, ordering, between, in/not-in, null checks
    let mut float_ops = InputObject::new(FILTER_OPS_FLOAT)
        .description("Float filter operators")
        .oneof();
    float_ops = float_ops
        .field(InputValue::new(
            FilterOperator::Equal(String::default()).to_string(),
            TypeRef::named(TypeRef::FLOAT),
        ))
        .field(InputValue::new(
            FilterOperator::NotEqual(String::default()).to_string(),
            TypeRef::named(TypeRef::FLOAT),
        ))
        .field(InputValue::new(
            FilterOperator::GreaterThan(String::default()).to_string(),
            TypeRef::named(TypeRef::FLOAT),
        ))
        .field(InputValue::new(
            FilterOperator::GreaterThanOrEqual(String::default()).to_string(),
            TypeRef::named(TypeRef::FLOAT),
        ))
        .field(InputValue::new(
            FilterOperator::LessThan(String::default()).to_string(),
            TypeRef::named(TypeRef::FLOAT),
        ))
        .field(InputValue::new(
            FilterOperator::LessThanOrEqual(String::default()).to_string(),
            TypeRef::named(TypeRef::FLOAT),
        ))
        .field(InputValue::new(
            FilterOperator::Between(String::default(), String::default()).to_string(),
            TypeRef::named_list(TypeRef::FLOAT),
        ))
        .field(InputValue::new(
            FilterOperator::<Box<dyn ColumnValueTrait>>::In(vec![]).to_string(),
            TypeRef::named_list(TypeRef::FLOAT),
        ))
        .field(InputValue::new(
            FilterOperator::<Box<dyn ColumnValueTrait>>::NotIn(vec![]).to_string(),
            TypeRef::named_list(TypeRef::FLOAT),
        ))
        .field(InputValue::new(
            FilterOperator::<Box<dyn ColumnValueTrait>>::IsNull.to_string(),
            TypeRef::named(TypeRef::BOOLEAN),
        ))
        .field(InputValue::new(
            FilterOperator::<Box<dyn ColumnValueTrait>>::NotNull.to_string(),
            TypeRef::named(TypeRef::BOOLEAN),
        ));

    // Boolean operators: Equal, NotEqual, IsNull, NotNull
    let mut bool_ops = InputObject::new(FILTER_OPS_BOOL)
        .description("Boolean filter operators")
        .oneof();
    bool_ops = bool_ops
        .field(InputValue::new(
            FilterOperator::Equal(String::default()).to_string(),
            TypeRef::named(TypeRef::BOOLEAN),
        ))
        .field(InputValue::new(
            FilterOperator::NotEqual(String::default()).to_string(),
            TypeRef::named(TypeRef::BOOLEAN),
        ))
        .field(InputValue::new(
            FilterOperator::<Box<dyn ColumnValueTrait>>::IsNull.to_string(),
            TypeRef::named(TypeRef::BOOLEAN),
        ))
        .field(InputValue::new(
            FilterOperator::<Box<dyn ColumnValueTrait>>::NotNull.to_string(),
            TypeRef::named(TypeRef::BOOLEAN),
        ));

    vec![string_ops, int_ops, float_ops, bool_ops]
}

pub fn operator_input_name_for(scalar: ColumnScalar) -> &'static str {
    match base_scalar_for_ops(&scalar) {
        ColumnScalar::String => FILTER_OPS_STRING,
        ColumnScalar::Int => FILTER_OPS_INT,
        ColumnScalar::Float => FILTER_OPS_FLOAT,
        ColumnScalar::Boolean => FILTER_OPS_BOOL,
        ColumnScalar::List(_) => unreachable!("base_scalar_for_ops never returns List"),
    }
}
