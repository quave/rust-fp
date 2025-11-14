use async_graphql::dynamic::{ListAccessor, ValueAccessor};

#[inline]
pub fn access_string(accessor: ValueAccessor) -> String {
    accessor
        .string()
        .expect("Failed to get string in graphql schema.")
        .to_string()
}

#[inline]
pub fn access_int(accessor: ValueAccessor) -> i64 {
    accessor
        .i64()
        .expect("Failed to get int in graphql schema.")
}

#[inline]
pub fn access_float(accessor: ValueAccessor) -> f64 {
    accessor
        .f64()
        .expect("Failed to get float in graphql schema.")
}

#[inline]
pub fn access_bool(accessor: ValueAccessor) -> bool {
    accessor
        .boolean()
        .expect("Failed to get boolean in graphql schema.")
}

#[inline]
pub fn access_array(accessor: ValueAccessor) -> Vec<String> {
    let list_acc: ListAccessor = accessor
        .list()
        .expect("Failed to get list in graphql schema.");
    
    list_acc.iter().map(|v| {
        v.string()
        .expect("Failed to convert list element to string in graphql schema.")
        .to_string()
    }).collect::<Vec<String>>()
}

pub fn access_list_by<T, F: Fn(ValueAccessor) -> T>(accessor: ValueAccessor, f: F) -> Vec<T> {
    let list_acc: ListAccessor = accessor
        .list()
        .expect("Failed to get list in graphql schema.");
    list_acc.iter().map(|v| f(v)).collect::<Vec<T>>()
}


