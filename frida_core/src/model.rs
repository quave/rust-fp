use async_trait::async_trait;
use chrono::{serde::ts_seconds, DateTime, Utc};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{error::Error, fmt::Debug};

pub type ModelId = i64;

#[async_trait]
pub trait Processible: Send + Sync {
    fn id(&self) -> ModelId;
    fn extract_features(&self) -> Vec<Feature>;
    fn as_json(&self) -> Result<String, Box<dyn Error + Send + Sync>>;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggeredRule {
    pub id: i64,
    pub order_id: i64,
    pub rule_name: String,
    pub rule_score: i32,
    #[serde(with = "ts_seconds")]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeatureValue {
    Int(i64),
    Double(f64),
    String(String),
    Bool(bool),
    #[serde(with = "ts_seconds")]
    DateTime(DateTime<Utc>),
    IntList(Vec<i64>),
    DoubleList(Vec<f64>),
    StringList(Vec<String>),
    BoolList(Vec<bool>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feature {
    pub name: String,
    pub value: Box<FeatureValue>,
}

pub struct ScorerResult {
    pub score: f32,
    pub name: String,
}
