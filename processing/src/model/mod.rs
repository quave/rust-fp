use chrono::{DateTime, Utc, serde::ts_seconds};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::Debug;
use evalexpr::Value as EvalValue;
use strum_macros::Display as EnumDisplay;

pub type ModelId = i64;

// Submodules for trait definitions
pub mod processible;
pub mod sea_orm_queue_entities;
// pub mod sea_orm_storage_model;
pub mod mongo_model;

// Re-export traits for downstream crates
pub use processible::Processible;
pub use processible::ProcessibleSerde;

// Re-export MatcherConfig
pub type MatcherConfig = (i32, i32); // (confidence, importance) 
pub type SchemaVersion = (i32, i32);

pub type GenericError = Box<dyn Error + Send + Sync>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchingField {
    pub matcher: String,
    pub value: String,
    pub datetime_alpha: Option<DateTime<Utc>>,
    pub datetime_beta: Option<DateTime<Utc>>,
    pub long_alpha: Option<f64>,
    pub lat_alpha: Option<f64>,
    pub long_beta: Option<f64>,
    pub lat_beta: Option<f64>,
    pub long_gamma: Option<f64>,
    pub lat_gamma: Option<f64>,
    pub long_delta: Option<f64>,
    pub lat_delta: Option<f64>,
}

impl MatchingField {
    pub fn new_simple(matcher: String, value: String) -> Self {
        Self {
            matcher,
            value,
            datetime_alpha: None,
            datetime_beta: None,
            long_alpha: None,
            lat_alpha: None,
            long_beta: None,
            lat_beta: None,
            long_gamma: None,
            lat_gamma: None,
            long_delta: None,
            lat_delta: None,
        }
    }

    pub fn new_with_timespace(
        matcher: String,
        value: String,
        created_at: DateTime<Utc>,
        location: (f64, f64),
    ) -> Self {
        Self {
            matcher,
            value,
            datetime_alpha: Some(created_at),
            datetime_beta: None,
            long_alpha: Some(location.0),
            lat_alpha: Some(location.1),
            long_beta: None,
            lat_beta: None,
            long_gamma: None,
            lat_gamma: None,
            long_delta: None,
            lat_delta: None,
        }
    }

    pub fn new(
        matcher: String,
        value: String,
        datetime_alpha: Option<DateTime<Utc>>,
        datetime_beta: Option<DateTime<Utc>>,
        location_alpha: Option<(f64, f64)>,
        location_beta: Option<(f64, f64)>,
        location_gamma: Option<(f64, f64)>,
        location_delta: Option<(f64, f64)>,
    ) -> Self {
        Self {
            matcher,
            value,
            datetime_alpha,
            datetime_beta,
            long_alpha: location_alpha.map(|(l, _)| l),
            lat_alpha: location_alpha.map(|(_, l)| l),
            long_beta: location_beta.map(|(l, _)| l),
            lat_beta: location_beta.map(|(_, l)| l),
            long_gamma: location_gamma.map(|(l, _)| l),
            lat_gamma: location_gamma.map(|(_, l)| l),
            long_delta: location_delta.map(|(l, _)| l),
            lat_delta: location_delta.map(|(_, l)| l),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectedTransaction {
    pub payload_number: String,
    pub path: Vec<String>,
    pub total_confidence: i32,
}

impl PartialEq for ConnectedTransaction {
    fn eq(&self, other: &Self) -> bool {
        self.payload_number == other.payload_number
            && self.path == other.path
            && self.total_confidence == other.total_confidence
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectConnection {
    pub payload_number: String,
    pub matcher: String,
    pub confidence: i32,
    pub importance: i32,
}

pub trait ScoringResult: Send + Sync {
    fn get_total_score(&self) -> i32;
    fn get_result_payload(&self) -> serde_json::Value;
}

impl ScoringResult for Vec<ExpressionRule> {
    fn get_total_score(&self) -> i32 {
        self.iter().map(|rule| rule.score).sum()
    }
    fn get_result_payload(&self) -> serde_json::Value {
        serde_json::json!(
            self.iter().map(|rule| rule.name.clone()).collect::<Vec<String>>()
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExpressionRule {
    pub name: String,
    pub description: String,
    pub rule: String,
    pub score: i32,
}

// Shared enums used by entities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, EnumDisplay)]
pub enum FraudLevel {
    Fraud,
    NoFraud,
    BlockedAutomatically,
    AccountTakeover,
    NotCreditWorthy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, EnumDisplay)]
pub enum LabelSource {
    Manual,
    Api,
}

// Feature infrastructure (used alongside the `features` entity)

#[derive(Debug, Clone)]
pub enum FeatureValue {
    Int(i64),
    Double(f64),
    String(String),
    Bool(bool),
    DateTime(DateTime<Utc>),
    IntList(Vec<i64>),
    DoubleList(Vec<f64>),
    StringList(Vec<String>),
    BoolList(Vec<bool>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, EnumDisplay)]
pub enum ScoringModelType {
    ExpressionBased,
    MachineLearning,
}


impl Into<EvalValue> for FeatureValue {
    fn into(self) -> EvalValue {
        match self {
            FeatureValue::Int(v) => EvalValue::Int(v),
            FeatureValue::Double(v) => EvalValue::Float(v),
            FeatureValue::String(v) => EvalValue::String(v),
            FeatureValue::Bool(v) => EvalValue::Boolean(v),
            FeatureValue::DateTime(v) => EvalValue::Int(v.timestamp_millis()),
            FeatureValue::IntList(v) => {
                EvalValue::Tuple(v.into_iter().map(|x| EvalValue::Int(x)).collect())
            }
            FeatureValue::DoubleList(v) => {
                EvalValue::Tuple(v.into_iter().map(|x| EvalValue::Float(x)).collect())
            }
            FeatureValue::StringList(v) => {
                EvalValue::Tuple(v.into_iter().map(|x| EvalValue::String(x)).collect())
            }
            FeatureValue::BoolList(v) => {
                EvalValue::Tuple(v.into_iter().map(|x| EvalValue::Boolean(x)).collect())
            }
        }
    }
}

impl PartialEq for FeatureValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (FeatureValue::Int(a), FeatureValue::Int(b)) => a == b,
            (FeatureValue::Double(a), FeatureValue::Double(b)) => a == b,
            (FeatureValue::String(a), FeatureValue::String(b)) => a == b,
            (FeatureValue::Bool(a), FeatureValue::Bool(b)) => a == b,
            (FeatureValue::DateTime(a), FeatureValue::DateTime(b)) => a == b,
            (FeatureValue::IntList(a), FeatureValue::IntList(b)) => a == b,
            (FeatureValue::DoubleList(a), FeatureValue::DoubleList(b)) => a == b,
            (FeatureValue::StringList(a), FeatureValue::StringList(b)) => a == b,
            (FeatureValue::BoolList(a), FeatureValue::BoolList(b)) => a == b,
            _ => false,
        }
    }
}

impl Serialize for FeatureValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(2))?;
        match self {
            FeatureValue::Int(v) => {
                map.serialize_entry("type", "integer")?;
                map.serialize_entry("value", v)?;
            }
            FeatureValue::Double(v) => {
                map.serialize_entry("type", "double")?;
                map.serialize_entry("value", v)?;
            }
            FeatureValue::String(v) => {
                map.serialize_entry("type", "string")?;
                map.serialize_entry("value", v)?;
            }
            FeatureValue::Bool(v) => {
                map.serialize_entry("type", "boolean")?;
                map.serialize_entry("value", v)?;
            }
            FeatureValue::DateTime(v) => {
                map.serialize_entry("type", "datetime")?;
                map.serialize_entry("value", &v.to_rfc3339())?;
            }
            FeatureValue::IntList(v) => {
                map.serialize_entry("type", "integer_array")?;
                map.serialize_entry("value", v)?;
            }
            FeatureValue::DoubleList(v) => {
                map.serialize_entry("type", "double_array")?;
                map.serialize_entry("value", v)?;
            }
            FeatureValue::StringList(v) => {
                map.serialize_entry("type", "string_array")?;
                map.serialize_entry("value", v)?;
            }
            FeatureValue::BoolList(v) => {
                map.serialize_entry("type", "boolean_array")?;
                map.serialize_entry("value", v)?;
            }
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for FeatureValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        #[derive(Deserialize)]
        struct FeatureValueHelper {
            #[serde(rename = "type")]
            type_: String,
            value: serde_json::Value,
        }

        let helper = FeatureValueHelper::deserialize(deserializer)?;
        match helper.type_.as_str() {
            "integer" => Ok(FeatureValue::Int(
                helper
                    .value
                    .as_i64()
                    .ok_or_else(|| D::Error::custom("invalid integer"))?,
            )),
            "double" => Ok(FeatureValue::Double(
                helper
                    .value
                    .as_f64()
                    .ok_or_else(|| D::Error::custom("invalid double"))?,
            )),
            "string" => Ok(FeatureValue::String(
                helper
                    .value
                    .as_str()
                    .ok_or_else(|| D::Error::custom("invalid string"))?
                    .to_string(),
            )),
            "boolean" => Ok(FeatureValue::Bool(
                helper
                    .value
                    .as_bool()
                    .ok_or_else(|| D::Error::custom("invalid boolean"))?,
            )),
            "datetime" => {
                let datetime_str = helper
                    .value
                    .as_str()
                    .ok_or_else(|| D::Error::custom("invalid datetime string"))?;
                Ok(FeatureValue::DateTime(
                    DateTime::parse_from_rfc3339(datetime_str)
                        .map_err(D::Error::custom)?
                        .with_timezone(&Utc),
                ))
            }
            "integer_array" => {
                let array = helper
                    .value
                    .as_array()
                    .ok_or_else(|| D::Error::custom("invalid array"))?;
                let mut result = Vec::new();
                for v in array {
                    result.push(
                        v.as_i64()
                            .ok_or_else(|| D::Error::custom("invalid integer in array"))?,
                    );
                }
                Ok(FeatureValue::IntList(result))
            }
            "double_array" => {
                let array = helper
                    .value
                    .as_array()
                    .ok_or_else(|| D::Error::custom("invalid array"))?;
                let mut result = Vec::new();
                for v in array {
                    result.push(
                        v.as_f64()
                            .ok_or_else(|| D::Error::custom("invalid double in array"))?,
                    );
                }
                Ok(FeatureValue::DoubleList(result))
            }
            "string_array" => {
                let array = helper
                    .value
                    .as_array()
                    .ok_or_else(|| D::Error::custom("invalid array"))?;
                let mut result = Vec::new();
                for v in array {
                    result.push(
                        v.as_str()
                            .ok_or_else(|| D::Error::custom("invalid string in array"))?
                            .to_string(),
                    );
                }
                Ok(FeatureValue::StringList(result))
            }
            "boolean_array" => {
                let array = helper
                    .value
                    .as_array()
                    .ok_or_else(|| D::Error::custom("invalid array"))?;
                let mut result = Vec::new();
                for v in array {
                    result.push(
                        v.as_bool()
                            .ok_or_else(|| D::Error::custom("invalid boolean in array"))?,
                    );
                }
                Ok(FeatureValue::BoolList(result))
            }
            _ => Err(D::Error::custom(format!(
                "unknown feature value type: {}",
                helper.type_
            ))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Feature {
    pub name: String,
    pub value: Box<FeatureValue>,
}

impl PartialEq for Feature {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && *self.value == *other.value
    }
}

impl Serialize for Feature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(3))?;
        map.serialize_entry("name", &self.name)?;
        // Get type and value from FeatureValue
        let value_json = serde_json::to_value(&*self.value).map_err(serde::ser::Error::custom)?;
        let value_obj = value_json
            .as_object()
            .ok_or_else(|| serde::ser::Error::custom("invalid value"))?;
        if let Some(type_val) = value_obj.get("type") {
            map.serialize_entry("type", type_val)?;
        }
        if let Some(val) = value_obj.get("value") {
            map.serialize_entry("value", val)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for Feature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct FeatureHelper {
            name: String,
            #[serde(rename = "type")]
            type_: String,
            value: serde_json::Value,
        }

        let helper = FeatureHelper::deserialize(deserializer)?;
        // Create a combined value for FeatureValue
        let feature_value = serde_json::json!({
            "type": helper.type_,
            "value": helper.value
        });
        let value = serde_json::from_value(feature_value).map_err(serde::de::Error::custom)?;
        Ok(Feature {
            name: helper.name,
            value: Box::new(value),
        })
    }
}
