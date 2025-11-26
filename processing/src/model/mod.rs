use chrono::{DateTime, Utc, serde::ts_seconds};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

pub type ModelId = i64;

// Submodules for trait definitions
pub mod processible;
pub mod sea_orm_queue_entities;
pub mod sea_orm_storage_model;

// Re-export traits for downstream crates
pub use processible::Processible;
pub use processible::ProcessibleSerde;

pub use sea_orm_storage_model::channel::Model as Channel;
pub use sea_orm_storage_model::scoring_event::Model as ScoringEvent;
pub use sea_orm_storage_model::scoring_model::Model as ScoringModel;
pub use sea_orm_storage_model::transaction::Model as Transaction;
pub use sea_orm_storage_model::triggered_rule::Model as TriggeredRule;
pub use sea_orm_storage_model::*;

// Re-export MatcherConfig
pub type MatcherConfig = (i32, i32); // (confidence, importance) 
pub type SchemaVersion = (i32, i32);

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
    pub transaction_id: ModelId,
    pub path_matchers: Vec<String>,
    pub path_values: Vec<String>,
    pub depth: i32,
    pub confidence: i32,
    pub importance: i32,
    pub created_at: DateTime<Utc>,
}

impl PartialEq for ConnectedTransaction {
    fn eq(&self, other: &Self) -> bool {
        self.transaction_id == other.transaction_id
            && self.path_matchers == other.path_matchers
            && self.path_values == other.path_values
            && self.depth == other.depth
            && self.confidence == other.confidence
            && self.importance == other.importance
            && self.created_at == other.created_at
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectConnection {
    pub transaction_id: ModelId,
    pub matcher: String,
    pub confidence: i32,
    pub importance: i32,
    #[serde(with = "ts_seconds")]
    pub created_at: DateTime<Utc>,
}

// Shared enums used by entities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Text")]
pub enum FraudLevel {
    #[sea_orm(string_value = "Fraud")]
    Fraud,
    #[sea_orm(string_value = "NoFraud")]
    NoFraud,
    #[sea_orm(string_value = "BlockedAutomatically")]
    BlockedAutomatically,
    #[sea_orm(string_value = "AccountTakeover")]
    AccountTakeover,
    #[sea_orm(string_value = "NotCreditWorthy")]
    NotCreditWorthy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Text")]
pub enum LabelSource {
    #[sea_orm(string_value = "Manual")]
    Manual,
    #[sea_orm(string_value = "Api")]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Text")]
pub enum ScoringModelType {
    #[sea_orm(string_value = "RuleBased")]
    RuleBased,
    #[sea_orm(string_value = "MachineLearning")]
    MachineLearning,
}
