use chrono::{serde::ts_seconds, DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use sea_orm::entity::prelude::*;

pub type ModelId = i64;

// Submodules for trait definitions
pub mod processible;
pub mod importable;
pub mod importable_serde;
pub mod web_transaction;
pub mod sea_orm_storage_model;

// Re-export traits for downstream crates
pub use processible::Processible;
pub use importable::Importable;
pub use importable_serde::ImportableSerde;
pub use web_transaction::WebTransaction;
pub use sea_orm_storage_model::*;
pub use sea_orm_storage_model::triggered_rule::Model as TriggeredRule;
pub use sea_orm_storage_model::channel::Model as Channel;
pub use sea_orm_storage_model::scoring_model::Model as ScoringModel;
pub use sea_orm_storage_model::scoring_event::Model as ScoringEvent;

// Re-export MatcherConfig
pub type MatcherConfig = (i32, i32); // (confidence, importance) 

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchingField {
    pub matcher: String,
    pub value: String,
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
        self.transaction_id == other.transaction_id &&
        self.path_matchers == other.path_matchers &&
        self.path_values == other.path_values &&
        self.depth == other.depth &&
        self.confidence == other.confidence &&
        self.importance == other.importance &&
        self.created_at == other.created_at
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
