use async_trait::async_trait;
use chrono::{serde::ts_seconds, DateTime, Utc};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{error::Error, fmt::Debug};

pub type ModelId = i64;

#[async_trait]
pub trait Processible: Send + Sync {
    fn id(&self) -> ModelId;
    fn tx_id(&self) -> ModelId;
    fn extract_features(&self) -> Vec<Feature>;
}

#[async_trait]
pub trait Importable: Send + Sync {
    fn validate(&self) -> Result<(), String>;
}

#[async_trait]
pub trait ImportableSerde: Importable + DeserializeOwned {
    fn as_json(&self) -> Result<String, Box<dyn Error + Send + Sync>>;
    fn from_json(json: &str) -> Result<Self, Box<dyn Error + Send + Sync>>;
}

#[async_trait]
pub trait WebTransaction: Send + Sync + Serialize {
    fn id(&self) -> ModelId;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggeredRule {
    pub id: ModelId,
    pub transaction_id: ModelId,
    pub rule_name: String,
    pub rule_score: i32,
    #[serde(with = "ts_seconds")]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: ModelId,
    #[serde(with = "ts_seconds")]
    pub created_at: DateTime<Utc>,
}

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
                map.serialize_entry("type", "number")?;
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
                map.serialize_entry("type", "number_array")?;
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
            "integer" => Ok(FeatureValue::Int(helper.value.as_i64().ok_or_else(|| D::Error::custom("invalid integer"))?)),
            "number" => Ok(FeatureValue::Double(helper.value.as_f64().ok_or_else(|| D::Error::custom("invalid number"))?)),
            "string" => Ok(FeatureValue::String(helper.value.as_str().ok_or_else(|| D::Error::custom("invalid string"))?.to_string())),
            "boolean" => Ok(FeatureValue::Bool(helper.value.as_bool().ok_or_else(|| D::Error::custom("invalid boolean"))?)),
            "datetime" => {
                let datetime_str = helper.value.as_str().ok_or_else(|| D::Error::custom("invalid datetime string"))?;
                Ok(FeatureValue::DateTime(DateTime::parse_from_rfc3339(datetime_str).map_err(D::Error::custom)?.with_timezone(&Utc)))
            }
            "integer_array" => {
                let array = helper.value.as_array().ok_or_else(|| D::Error::custom("invalid array"))?;
                let mut result = Vec::new();
                for v in array {
                    result.push(v.as_i64().ok_or_else(|| D::Error::custom("invalid integer in array"))?);
                }
                Ok(FeatureValue::IntList(result))
            }
            "number_array" => {
                let array = helper.value.as_array().ok_or_else(|| D::Error::custom("invalid array"))?;
                let mut result = Vec::new();
                for v in array {
                    result.push(v.as_f64().ok_or_else(|| D::Error::custom("invalid number in array"))?);
                }
                Ok(FeatureValue::DoubleList(result))
            }
            "string_array" => {
                let array = helper.value.as_array().ok_or_else(|| D::Error::custom("invalid array"))?;
                let mut result = Vec::new();
                for v in array {
                    result.push(v.as_str().ok_or_else(|| D::Error::custom("invalid string in array"))?.to_string());
                }
                Ok(FeatureValue::StringList(result))
            }
            "boolean_array" => {
                let array = helper.value.as_array().ok_or_else(|| D::Error::custom("invalid array"))?;
                let mut result = Vec::new();
                for v in array {
                    result.push(v.as_bool().ok_or_else(|| D::Error::custom("invalid boolean in array"))?);
                }
                Ok(FeatureValue::BoolList(result))
            }
            _ => Err(D::Error::custom(format!("unknown feature value type: {}", helper.type_))),
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
        let value_obj = value_json.as_object().ok_or_else(|| serde::ser::Error::custom("invalid value"))?;
        
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
        
        let value = serde_json::from_value(feature_value)
            .map_err(serde::de::Error::custom)?;
        
        Ok(Feature {
            name: helper.name,
            value: Box::new(value),
        })
    }
}

#[derive(Debug, Clone, Eq)]
pub struct ScorerResult {
    pub score: i32,
    pub name: String,
}

impl PartialEq for ScorerResult {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score && self.name == other.name
    }
}
