use chrono::{serde::ts_seconds, DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

pub type ModelId = i64;

// Submodules for trait definitions
pub mod processible;
pub mod importable;
pub mod importable_serde;
pub mod web_transaction;

// Re-export traits for downstream crates
pub use processible::Processible;
pub use importable::Importable;
pub use importable_serde::ImportableSerde;
pub use web_transaction::WebTransaction;

pub use crate::storage::sea_orm_storage_model::triggered_rule::Model as TriggeredRule;

pub use crate::storage::sea_orm_storage_model::{FraudLevel, LabelSource};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchingField {
    pub matcher: String,
    pub value: String,
}

// Storage-backed Transaction is represented by SeaORM entity in
// `crate::storage::sea_orm_storage_model::transaction::Model`

// Storage-backed Label is represented by SeaORM entity in
// `crate::storage::sea_orm_storage_model::label::Model`

// Storage-backed ScoringEvent is represented by SeaORM entity in
// `crate::storage::sea_orm_storage_model::scoring_event::Model`

// Storage-backed Channel is represented by SeaORM entity in
// `crate::storage::sea_orm_storage_model::channel::Model`

// Storage-backed Model is represented by SeaORM entity in
// `crate::storage::sea_orm_storage_model::model::Model`

pub use crate::storage::sea_orm_storage_model::{Feature, FeatureValue};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelingResult {
    pub label_id: ModelId,
    pub success_count: usize,
    pub failed_transaction_ids: Vec<ModelId>,
}

impl LabelingResult {
    pub fn is_complete_success(&self) -> bool {
        self.success_count > 0 && self.failed_transaction_ids.is_empty()
    }
    
    pub fn is_partial_success(&self) -> bool {
        self.success_count > 0 && !self.failed_transaction_ids.is_empty()
    }
    
    pub fn is_complete_failure(&self) -> bool {
        self.success_count == 0 && !self.failed_transaction_ids.is_empty()
    }
}
