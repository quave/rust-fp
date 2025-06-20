use async_trait::async_trait;
use chrono::{serde::ts_seconds, DateTime, Utc};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    fmt::{Debug, Display},
    str::FromStr,
};

#[async_trait]
pub trait Processible: Sized + Send + Sync + Debug + Clone + Serialize + DeserializeOwned {
    type Id: Send + Sync + Debug + Clone + Serialize + DeserializeOwned + Display + FromStr;

    async fn extract_features(&self) -> Vec<Feature>;
    fn get_id(&self) -> Self::Id;
}

#[async_trait]
pub trait Importable: Send + Sync + Debug + Clone + Serialize + DeserializeOwned {
    // async fn save<P: Processible>(
    //     &self,
    //     storage: &dyn Storage<Self, P>,
    // ) -> Result<P, Box<dyn Error + Send + Sync>>;
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
    pub value: FeatureValue,
}

pub struct ScorerResult {
    pub score: f32,
    pub name: String,
}
