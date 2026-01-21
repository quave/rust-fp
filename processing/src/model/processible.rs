use std::error::Error;
use std::fmt::Debug;
use std::str::FromStr;
use std::sync::Arc;

use crate::model::Feature;
use crate::model::{ConnectedTransaction, DirectConnection, MatchingField};
use async_graphql::Value;
use async_graphql::dynamic::TypeRef;
use async_trait::async_trait;
use seaography::itertools::Itertools;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fmt::Display;
use strum_macros::Display as EnumDisplay;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ColumnScalar {
    String,
    Int,
    Float,
    Boolean,
    List(Box<ColumnScalar>),
}

#[async_trait]
pub trait Processible: Send + Sync + Clone + Serialize + DeserializeOwned + 'static {
    type Id: Send + Sync + PartialEq + Serialize + DeserializeOwned + Clone + ToString + FromStr + 'static;

    fn validate(&self) -> Result<(), String>;

    fn payload_number(&self) -> String;

    fn schema_version(&self) -> (i32, i32);

    fn extract_simple_features(&self) -> Vec<Feature>;

    fn extract_graph_features(
        &self,
        connected_transactions: &[ConnectedTransaction],
        direct_connections: &[DirectConnection],
    ) -> Vec<Feature>;

    fn extract_matching_fields(&self) -> Vec<MatchingField>;
}

pub trait ColumnValueTrait: Display + Send + Sync + 'static {}

impl<T> ColumnValueTrait for T where T: Display + Send + Sync + 'static {}

#[derive(Debug, Clone)]
pub struct Filter<T: ColumnValueTrait> {
    pub column: String,
    pub operator_value: FilterOperator<T>,
}

#[derive(Clone)]
pub struct ColumnFilter<P: Processible> {
    pub column: String,
    pub help_text: String,
    pub scalar: ColumnScalar,
    pub resolver: Arc<dyn (Fn(&P) -> Value) + Send + Sync>,
    pub filter_statement:
        Option<Arc<dyn Fn(&Filter<Box<dyn ColumnValueTrait>>) -> String + Send + Sync>>,
}

#[async_trait]
//TODO: converto to Serialize + DeserializeOwned
pub trait ProcessibleSerde: Processible + DeserializeOwned {
    fn as_json(&self) -> Result<serde_json::Value, Box<dyn Error + Send + Sync>>;
    fn from_json(json: serde_json::Value) -> Result<Self, Box<dyn Error + Send + Sync>>;

    fn list_column_fields() -> Vec<ColumnFilter<Self>>;
}

#[derive(EnumDisplay, Debug, Clone)]
pub enum FilterOperator<T: ColumnValueTrait> {
    #[strum(to_string = "eq")]
    Equal(T),
    #[strum(to_string = "not_eq")]
    NotEqual(T),
    #[strum(to_string = "gt")]
    GreaterThan(T),
    #[strum(to_string = "gte")]
    GreaterThanOrEqual(T),
    #[strum(to_string = "lt")]
    LessThan(T),
    #[strum(to_string = "lte")]
    LessThanOrEqual(T),
    #[strum(to_string = "between")]
    Between(T, T),
    #[strum(to_string = "is_null")]
    IsNull,
    #[strum(to_string = "is_not_null")]
    NotNull,
    #[strum(to_string = "contains")]
    Contains(T),
    #[strum(to_string = "in")]
    In(Vec<T>),
    #[strum(to_string = "not_in")]
    NotIn(Vec<T>),
}

impl<T: ColumnValueTrait> FilterOperator<T> {
    pub fn to_plain_statement(&self, tpe: &str) -> String {
        match self {
            FilterOperator::Equal(v) => {
                if tpe == TypeRef::STRING {
                    format!("= '{}'", v.to_string())
                } else {
                    format!("= {}", v.to_string())
                }
            }
            FilterOperator::NotEqual(v) => {
                if tpe == TypeRef::STRING {
                    format!("!= '{}'", v.to_string())
                } else {
                    format!("!= {}", v.to_string())
                }
            }
            FilterOperator::GreaterThan(v) => format!("> {}", v.to_string()),
            FilterOperator::GreaterThanOrEqual(v) => format!(">= {}", v.to_string()),
            FilterOperator::LessThan(v) => format!("< {}", v.to_string()),
            FilterOperator::LessThanOrEqual(v) => format!("<= {}", v.to_string()),
            FilterOperator::Between(v1, v2) => {
                if tpe == TypeRef::STRING {
                    format!("between '{}' and '{}'", v1.to_string(), v2.to_string())
                } else {
                    format!("between {} and {}", v1.to_string(), v2.to_string())
                }
            }
            FilterOperator::IsNull => format!("is null"),
            FilterOperator::NotNull => format!("is not null"),
            FilterOperator::Contains(v) => format!("%{}%", v.to_string()),
            FilterOperator::In(v) => {
                if tpe == TypeRef::STRING {
                    format!(
                        "in ({})",
                        v.iter().map(|v| format!("'{}'", v.to_string())).join(", ")
                    )
                } else {
                    format!(
                        "in ({})",
                        v.iter().map(|v| format!("{}", v.to_string())).join(", ")
                    )
                }
            }
            FilterOperator::NotIn(v) => {
                if tpe == TypeRef::STRING {
                    format!(
                        "not in ({})",
                        v.iter().map(|v| format!("'{}'", v.to_string())).join(", ")
                    )
                } else {
                    format!(
                        "not in ({})",
                        v.iter().map(|v| format!("{}", v.to_string())).join(", ")
                    )
                }
            }
        }
    }

    pub fn to_json_path_statement(&self, tpe: &str) -> String {
        match self {
            FilterOperator::Equal(v) => {
                if tpe == TypeRef::STRING {
                    format!("= \"{}\"", v.to_string())
                } else {
                    format!("= {}", v.to_string())
                }
            }
            FilterOperator::NotEqual(v) => {
                if tpe == TypeRef::STRING {
                    format!("!= \"{}\"", v.to_string())
                } else {
                    format!("!= {}", v.to_string())
                }
            }
            FilterOperator::GreaterThan(v) => format!("> {}", v.to_string()),
            FilterOperator::GreaterThanOrEqual(v) => format!(">= {}", v.to_string()),
            FilterOperator::LessThan(v) => format!("< {}", v.to_string()),
            FilterOperator::LessThanOrEqual(v) => format!("<= {}", v.to_string()),
            FilterOperator::Between(v1, v2) => {
                if tpe == TypeRef::STRING {
                    format!("between \"{}\" and \"{}\"", v1.to_string(), v2.to_string())
                } else {
                    format!("between {} and {}", v1.to_string(), v2.to_string())
                }
            }
            FilterOperator::IsNull => format!("is null"),
            FilterOperator::NotNull => format!("is not null"),
            FilterOperator::Contains(v) => format!("%{}%", v.to_string()),
            FilterOperator::In(v) => {
                if tpe == TypeRef::STRING {
                    format!(
                        "in ({})",
                        v.iter()
                            .map(|v| format!("\"{}\"", v.to_string()))
                            .join(", ")
                    )
                } else {
                    format!(
                        "in ({})",
                        v.iter().map(|v| format!("{}", v.to_string())).join(", ")
                    )
                }
            }
            FilterOperator::NotIn(v) => {
                if tpe == TypeRef::STRING {
                    format!(
                        "not in ({})",
                        v.iter()
                            .map(|v| format!("\"{}\"", v.to_string()))
                            .join(", ")
                    )
                } else {
                    format!(
                        "not in ({})",
                        v.iter().map(|v| format!("{}", v.to_string())).join(", ")
                    )
                }
            }
        }
    }
}
