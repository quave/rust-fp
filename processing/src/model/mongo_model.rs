use chrono::NaiveDateTime;
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

use crate::model::{ExpressionRule, Feature, FraudLevel, LabelSource};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Label {
    pub fraud_level: FraudLevel,
    pub fraud_category: String,
    pub label_source: LabelSource,
    pub labeled_by: String,
    pub created_at: NaiveDateTime,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Transaction {
    pub _id: ObjectId,
    pub payload_number: String,
    pub transaction_version: i32,
    pub is_latest: bool,
    pub payload: serde_json::Value,
    pub schema_version_major: i32,
    pub schema_version_minor: i32,
    pub label: Option<Label>,
    pub features_set: Option<FeaturesSet>,
    pub comment: Option<String>,
    pub last_scoring_date: Option<NaiveDateTime>,
    pub processing_complete: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScoringModel {
    pub name: String,
    pub features_schema_version_major: i32,
    pub features_schema_version_minor: i32,
    pub version: String,
    //TODO: should be made model type agnostic payload
    pub expression_rules: Vec<ExpressionRule>,
    pub model_type: super::ScoringModelType,
    pub created_at: NaiveDateTime,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScoringEvent {
    pub _id: ObjectId,
    pub transaction_id: ObjectId,
    pub channel_id: ObjectId,
    pub triggered_rules: Vec<String>,
    pub created_at: NaiveDateTime,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScoringChannel {
    pub _id: ObjectId,
    pub channel_name: String,
    pub model: ScoringModel,
    pub is_active: bool,
    pub created_at: NaiveDateTime,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FeaturesSet {
    pub schema_version_major: i32,
    pub schema_version_minor: i32,
    pub simple_features: Vec<Feature>,
    pub graph_features: Vec<Feature>,
    pub created_at: NaiveDateTime,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MatchNode {
    pub _id: ObjectId,
    pub transaction_data: Vec<MatchNodeTransaction>,
    pub payload_numbers: Vec<String>,
    pub matcher: String,
    pub value: String,
    pub confidence: i32,
    pub importance: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MatchNodeTransaction {
    pub payload_number: String,
    pub datetime_alpha: Option<NaiveDateTime>,
    pub datetime_beta: Option<NaiveDateTime>,
    pub long_alpha: Option<f64>,
    pub lat_alpha: Option<f64>,
    pub long_beta: Option<f64>,
    pub lat_beta: Option<f64>,
    pub long_gamma: Option<f64>,
    pub lat_gamma: Option<f64>,
    pub long_delta: Option<f64>,
    pub lat_delta: Option<f64>,
    pub created_at: NaiveDateTime,
}
