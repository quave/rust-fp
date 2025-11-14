use std::sync::Arc;
use async_graphql::dynamic::ObjectAccessor;
use crate::model::{ProcessibleSerde, processible::{Filter, FilterOperator, ColumnValueTrait}};
use super::types::{ColumnTypeIndex, ColumnScalar, base_scalar_for_ops};
use super::accessors::{access_string, access_array, access_int, access_float, access_bool, access_list_by};

pub fn parse_filters<P: ProcessibleSerde>(
    column_types: &Arc<ColumnTypeIndex>,
    filters_accsessor: ObjectAccessor
) -> Result<Vec<Filter<Box<dyn ColumnValueTrait>>>, async_graphql::Error> {
    let column_names = filters_accsessor.keys();

    let filters: Vec<Result<Filter<Box<dyn ColumnValueTrait>>, async_graphql::Error>> = column_names.map(|col_name| {
        let filter_object = filters_accsessor
            .get(col_name)
            .expect("Failed to get filter object in graphql schema.")
            .object()?;

        let op_name = filter_object
            .keys()
            .next()
            .expect("Failed to get operator in graphql schema.")
            .to_string();
        
        let op_object = filter_object
            .get(&op_name)
            .expect(&format!("Failed to get operator {} in graphql schema.", op_name));

        let scalar = column_types.get(&col_name.to_string()).cloned().unwrap_or(ColumnScalar::String);
        let base = base_scalar_for_ops(&scalar);
        let op_value: FilterOperator<Box<dyn ColumnValueTrait>> = match base {
            ColumnScalar::String => match op_name {
                v if v == FilterOperator::Equal(String::default()).to_string() =>
                    FilterOperator::<Box<dyn ColumnValueTrait>>::Equal(Box::new(access_string(op_object))),
                v if v == FilterOperator::NotEqual(String::default()).to_string() =>
                    FilterOperator::<Box<dyn ColumnValueTrait>>::NotEqual(Box::new(access_string(op_object))),
                v if v == FilterOperator::Contains(String::default()).to_string() =>
                    FilterOperator::<Box<dyn ColumnValueTrait>>::Contains(Box::new(access_string(op_object))),
                v if v == FilterOperator::<Box<dyn ColumnValueTrait>>::In(vec![]).to_string() =>
                    FilterOperator::<Box<dyn ColumnValueTrait>>::In(access_array(op_object).into_iter().map(|v| Box::new(v) as Box<dyn ColumnValueTrait>).collect()),
                v if v == FilterOperator::<Box<dyn ColumnValueTrait>>::NotIn(vec![]).to_string() =>
                    FilterOperator::<Box<dyn ColumnValueTrait>>::NotIn(access_array(op_object).into_iter().map(|v| Box::new(v) as Box<dyn ColumnValueTrait>).collect()),
                v if v == FilterOperator::<Box<dyn ColumnValueTrait>>::IsNull.to_string() =>
                    FilterOperator::<Box<dyn ColumnValueTrait>>::IsNull,
                v if v == FilterOperator::<Box<dyn ColumnValueTrait>>::NotNull.to_string() =>
                    FilterOperator::<Box<dyn ColumnValueTrait>>::NotNull,
                _ => return Err(async_graphql::Error::new("Invalid operator for String in graphql schema.")),
            },
            ColumnScalar::Int => match op_name {
                v if v == FilterOperator::Equal(String::default()).to_string() =>
                    FilterOperator::<Box<dyn ColumnValueTrait>>::Equal(Box::new(access_int(op_object))),
                v if v == FilterOperator::NotEqual(String::default()).to_string() =>
                    FilterOperator::<Box<dyn ColumnValueTrait>>::NotEqual(Box::new(access_int(op_object))),
                v if v == FilterOperator::GreaterThan(String::default()).to_string() =>
                    FilterOperator::<Box<dyn ColumnValueTrait>>::GreaterThan(Box::new(access_int(op_object))),
                v if v == FilterOperator::GreaterThanOrEqual(String::default()).to_string() =>
                    FilterOperator::<Box<dyn ColumnValueTrait>>::GreaterThanOrEqual(Box::new(access_int(op_object))),
                v if v == FilterOperator::LessThan(String::default()).to_string() =>
                    FilterOperator::<Box<dyn ColumnValueTrait>>::LessThan(Box::new(access_int(op_object))),
                v if v == FilterOperator::LessThanOrEqual(String::default()).to_string() =>
                    FilterOperator::<Box<dyn ColumnValueTrait>>::LessThanOrEqual(Box::new(access_int(op_object))),
                v if v == FilterOperator::Between(String::default(), String::default()).to_string() => {
                    let list: Vec<i64> = access_list_by(op_object, access_int);
                    FilterOperator::<Box<dyn ColumnValueTrait>>::Between(
                        Box::new(*list.get(0).expect("Failed to get first element in graphql schema.")),
                        Box::new(*list.get(1).expect("Failed to get second element in graphql schema."))
                    )
                },
                v if v == FilterOperator::<Box<dyn ColumnValueTrait>>::In(vec![]).to_string() => {
                    let list: Vec<i64> = access_list_by(op_object, access_int);
                    FilterOperator::<Box<dyn ColumnValueTrait>>::In(list.into_iter().map(|v| Box::new(v) as Box<dyn ColumnValueTrait>).collect())
                },
                v if v == FilterOperator::<Box<dyn ColumnValueTrait>>::NotIn(vec![]).to_string() => {
                    let list: Vec<i64> = access_list_by(op_object, access_int);
                    FilterOperator::<Box<dyn ColumnValueTrait>>::NotIn(list.into_iter().map(|v| Box::new(v) as Box<dyn ColumnValueTrait>).collect())
                },
                v if v == FilterOperator::<Box<dyn ColumnValueTrait>>::IsNull.to_string() =>
                    FilterOperator::<Box<dyn ColumnValueTrait>>::IsNull,
                v if v == FilterOperator::<Box<dyn ColumnValueTrait>>::NotNull.to_string() =>
                    FilterOperator::<Box<dyn ColumnValueTrait>>::NotNull,
                _ => return Err(async_graphql::Error::new("Invalid operator for Int in graphql schema.")),
            },
            ColumnScalar::Float => match op_name {
                v if v == FilterOperator::Equal(String::default()).to_string() =>
                    FilterOperator::<Box<dyn ColumnValueTrait>>::Equal(Box::new(access_float(op_object))),
                v if v == FilterOperator::NotEqual(String::default()).to_string() =>
                    FilterOperator::<Box<dyn ColumnValueTrait>>::NotEqual(Box::new(access_float(op_object))),
                v if v == FilterOperator::GreaterThan(String::default()).to_string() =>
                    FilterOperator::<Box<dyn ColumnValueTrait>>::GreaterThan(Box::new(access_float(op_object))),
                v if v == FilterOperator::GreaterThanOrEqual(String::default()).to_string() =>
                    FilterOperator::<Box<dyn ColumnValueTrait>>::GreaterThanOrEqual(Box::new(access_float(op_object))),
                v if v == FilterOperator::LessThan(String::default()).to_string() =>
                    FilterOperator::<Box<dyn ColumnValueTrait>>::LessThan(Box::new(access_float(op_object))),
                v if v == FilterOperator::LessThanOrEqual(String::default()).to_string() =>
                    FilterOperator::<Box<dyn ColumnValueTrait>>::LessThanOrEqual(Box::new(access_float(op_object))),
                v if v == FilterOperator::Between(String::default(), String::default()).to_string() => {
                    let list: Vec<f64> = access_list_by(op_object, access_float);
                    FilterOperator::<Box<dyn ColumnValueTrait>>::Between(
                        Box::new(*list.get(0).expect("Failed to get first element in graphql schema.")),
                        Box::new(*list.get(1).expect("Failed to get second element in graphql schema."))
                    )
                },
                v if v == FilterOperator::<Box<dyn ColumnValueTrait>>::In(vec![]).to_string() => {
                    let list: Vec<f64> = access_list_by(op_object, access_float);
                    FilterOperator::<Box<dyn ColumnValueTrait>>::In(list.into_iter().map(|v| Box::new(v) as Box<dyn ColumnValueTrait>).collect())
                },
                v if v == FilterOperator::<Box<dyn ColumnValueTrait>>::NotIn(vec![]).to_string() => {
                    let list: Vec<f64> = access_list_by(op_object, access_float);
                    FilterOperator::<Box<dyn ColumnValueTrait>>::NotIn(list.into_iter().map(|v| Box::new(v) as Box<dyn ColumnValueTrait>).collect())
                },
                v if v == FilterOperator::<Box<dyn ColumnValueTrait>>::IsNull.to_string() =>
                    FilterOperator::<Box<dyn ColumnValueTrait>>::IsNull,
                v if v == FilterOperator::<Box<dyn ColumnValueTrait>>::NotNull.to_string() =>
                    FilterOperator::<Box<dyn ColumnValueTrait>>::NotNull,
                _ => return Err(async_graphql::Error::new("Invalid operator for Float in graphql schema.")),
            },
            ColumnScalar::Boolean => match op_name {
                v if v == FilterOperator::Equal(String::default()).to_string() =>
                    FilterOperator::<Box<dyn ColumnValueTrait>>::Equal(Box::new(access_bool(op_object))),
                v if v == FilterOperator::NotEqual(String::default()).to_string() =>
                    FilterOperator::<Box<dyn ColumnValueTrait>>::NotEqual(Box::new(access_bool(op_object))),
                v if v == FilterOperator::<Box<dyn ColumnValueTrait>>::IsNull.to_string() =>
                    FilterOperator::<Box<dyn ColumnValueTrait>>::IsNull,
                v if v == FilterOperator::<Box<dyn ColumnValueTrait>>::NotNull.to_string() =>
                    FilterOperator::<Box<dyn ColumnValueTrait>>::NotNull,
                _ => return Err(async_graphql::Error::new("Invalid operator for Boolean in graphql schema.")),
            },
            ColumnScalar::List(_) => unreachable!("base_scalar_for_ops never returns List"),
        };

        return Ok(Filter {
            column: col_name.to_string(),
            operator_value: op_value,
        });

        }).collect::<Vec<Result<Filter<Box<dyn ColumnValueTrait>>, async_graphql::Error>>>();

    filters
        .into_iter()
        .fold(Ok(Vec::new()), |acc, f| {
            match acc {
                Ok(mut filters) => {
                    filters.push(f?);
                    Ok(filters)
                }
                Err(e) => Err(e),
            }
        })
}


