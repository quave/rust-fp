use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use chrono::{NaiveDateTime, DateTime, Utc};
use evalexpr::Value as EvalValue;

use crate::model::{FeatureValue, ScoringModelType, FraudLevel, LabelSource};
 
impl Into<EvalValue> for FeatureValue {
    fn into(self) -> EvalValue {
        match self {
            FeatureValue::Int(v) => EvalValue::Int(v),
            FeatureValue::Double(v) => EvalValue::Float(v),
            FeatureValue::String(v) => EvalValue::String(v),
            FeatureValue::Bool(v) => EvalValue::Boolean(v),
            FeatureValue::DateTime(v) => EvalValue::Int(v.timestamp_millis()),
            FeatureValue::IntList(v) => EvalValue::Tuple(v.into_iter().map(|x| EvalValue::Int(x)).collect()),
            FeatureValue::DoubleList(v) => EvalValue::Tuple(v.into_iter().map(|x| EvalValue::Float(x)).collect()),
            FeatureValue::StringList(v) => EvalValue::Tuple(v.into_iter().map(|x| EvalValue::String(x)).collect()),
            FeatureValue::BoolList(v) => EvalValue::Tuple(v.into_iter().map(|x| EvalValue::Boolean(x)).collect()),
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
// NaiveDateTime already imported above

// Labels
pub mod label {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "labels")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub fraud_level: FraudLevel,
        pub fraud_category: String,
        pub label_source: LabelSource,
        pub labeled_by: String,
        pub created_at: NaiveDateTime,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(has_many = "super::transaction::Entity")]
        Transaction,
    }

    impl Related<super::transaction::Entity> for Entity {
        fn to() -> RelationDef { Relation::Transaction.def() }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// Transactions
pub mod transaction {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "transactions")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub payload_number: String,
        pub payload: serde_json::Value,
        pub schema_version_major: i32,
        pub schema_version_minor: i32,
        pub label_id: Option<i64>,
        pub comment: Option<String>,
        pub last_scoring_date: Option<NaiveDateTime>,
        pub processing_complete: bool,
        pub created_at: NaiveDateTime,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(belongs_to = "super::label::Entity", from = "Column::LabelId", to = "super::label::Column::Id")]
        Label,
        #[sea_orm(has_many = "super::scoring_event::Entity")]
        ScoringEvent,
        #[sea_orm(has_many = "super::feature::Entity")]
        Feature,
        #[sea_orm(has_many = "super::match_node_transactions::Entity")]
        MatchNodeTransactions,
    }

    impl Related<super::label::Entity> for Entity {
        fn to() -> RelationDef { Relation::Label.def() }
    }

    impl Related<super::scoring_event::Entity> for Entity {
        fn to() -> RelationDef { Relation::ScoringEvent.def() }
    }

    impl Related<super::feature::Entity> for Entity {
        fn to() -> RelationDef { Relation::Feature.def() }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// Models
pub mod scoring_model {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "scoring_models")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub name: String,
        pub features_schema_version_major: i32,
        pub features_schema_version_minor: i32,
        pub version: String,
        pub model_type: super::ScoringModelType,
        pub created_at: NaiveDateTime,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(has_many = "super::channel::Entity")]
        Channel,
        #[sea_orm(has_many = "super::scoring_rule::Entity")]
        ScoringRule,
    }

    impl Related<super::channel::Entity> for Entity { fn to() -> RelationDef { Relation::Channel.def() } }
    impl Related<super::scoring_rule::Entity> for Entity { fn to() -> RelationDef { Relation::ScoringRule.def() } }

    impl ActiveModelBehavior for ActiveModel {}
}

// Channels
pub mod channel {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "channels")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub name: String,
        pub model_id: i64,
        pub created_at: NaiveDateTime,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(belongs_to = "super::scoring_model::Entity", from = "Column::ModelId", to = "super::scoring_model::Column::Id")]
        Model,
        #[sea_orm(has_many = "super::scoring_event::Entity")]
        ScoringEvent,
    }

    impl Related<super::scoring_model::Entity> for Entity { fn to() -> RelationDef { Relation::Model.def() } }
    impl Related<super::scoring_event::Entity> for Entity { fn to() -> RelationDef { Relation::ScoringEvent.def() } }

    impl ActiveModelBehavior for ActiveModel {}
}

// Scoring Events
pub mod scoring_event {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "scoring_events")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub transaction_id: i64,
        pub channel_id: i64,
        pub total_score: i32,
        pub created_at: NaiveDateTime,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(belongs_to = "super::transaction::Entity", from = "Column::TransactionId", to = "super::transaction::Column::Id")]
        Transaction,
        #[sea_orm(belongs_to = "super::channel::Entity", from = "Column::ChannelId", to = "super::channel::Column::Id")]
        Channel,
        #[sea_orm(has_many = "super::triggered_rule::Entity")]
        TriggeredRule,
    }

    impl Related<super::transaction::Entity> for Entity { fn to() -> RelationDef { Relation::Transaction.def() } }
    impl Related<super::channel::Entity> for Entity { fn to() -> RelationDef { Relation::Channel.def() } }
    impl Related<super::triggered_rule::Entity> for Entity { fn to() -> RelationDef { Relation::TriggeredRule.def() } }

    impl ActiveModelBehavior for ActiveModel {}
}

// Scoring Rules
pub mod scoring_rule {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "scoring_rules")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub model_id: i64,
        pub name: String,
        pub description: Option<String>,
        pub rule: serde_json::Value,
        pub score: i32,
        pub created_at: NaiveDateTime,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(belongs_to = "super::scoring_model::Entity", from = "Column::ModelId", to = "super::scoring_model::Column::Id")]
        Model,
        #[sea_orm(has_many = "super::triggered_rule::Entity")]
        TriggeredRule,
    }

    impl Related<super::scoring_model::Entity> for Entity { fn to() -> RelationDef { Relation::Model.def() } }
    impl Related<super::triggered_rule::Entity> for Entity { fn to() -> RelationDef { Relation::TriggeredRule.def() } }

    impl ActiveModelBehavior for ActiveModel {}
}

// Triggered Rules
pub mod triggered_rule {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "triggered_rules")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub scoring_events_id: i64,
        pub rule_id: i64,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(belongs_to = "super::scoring_event::Entity", from = "Column::ScoringEventsId", to = "super::scoring_event::Column::Id")]
        ScoringEvent,
        #[sea_orm(belongs_to = "super::scoring_rule::Entity", from = "Column::RuleId", to = "super::scoring_rule::Column::Id")]
        ScoringRule,
    }

    impl Related<super::scoring_event::Entity> for Entity { fn to() -> RelationDef { Relation::ScoringEvent.def() } }
    impl Related<super::scoring_rule::Entity> for Entity { fn to() -> RelationDef { Relation::ScoringRule.def() } }

    impl ActiveModelBehavior for ActiveModel {}
}

// Features
pub mod feature {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "features")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub transaction_id: i64,
        pub transaction_version: i32,
        pub schema_version_major: i32,
        pub schema_version_minor: i32,
        pub simple_features: Option<serde_json::Value>,
        pub graph_features: serde_json::Value,
        pub created_at: NaiveDateTime,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(belongs_to = "super::transaction::Entity", from = "Column::TransactionId", to = "super::transaction::Column::Id")]
        Transaction,
    }

    impl Related<super::transaction::Entity> for Entity { fn to() -> RelationDef { Relation::Transaction.def() } }
    impl ActiveModelBehavior for ActiveModel {}
}

// Processing Queue
pub mod processing_queue {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "processing_queue")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub processable_id: i64,
        pub processed_at: Option<NaiveDateTime>,
        pub created_at: NaiveDateTime,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

// Recalculation Queue
pub mod recalculation_queue {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "recalculation_queue")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub processable_id: i64,
        pub processed_at: Option<NaiveDateTime>,
        pub created_at: NaiveDateTime,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

// Match Node
pub mod match_node {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "match_node")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub matcher: String,
        pub value: String,
        pub confidence: i32,
        pub importance: i32,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(has_many = "super::match_node_transactions::Entity")]
        MatchNodeTransactions,
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// Match Node Transactions (composite PK)
pub mod match_node_transactions {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "match_node_transactions")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub node_id: i64,
        #[sea_orm(primary_key)]
        pub transaction_id: i64,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(belongs_to = "super::match_node::Entity", from = "Column::NodeId", to = "super::match_node::Column::Id")]
        MatchNode,
        #[sea_orm(belongs_to = "super::transaction::Entity", from = "Column::TransactionId", to = "super::transaction::Column::Id")]
        Transaction,
    }

    impl Related<super::match_node::Entity> for Entity { fn to() -> RelationDef { Relation::MatchNode.def() } }
    impl Related<super::transaction::Entity> for Entity { fn to() -> RelationDef { Relation::Transaction.def() } }

    impl ActiveModelBehavior for ActiveModel {}
}


