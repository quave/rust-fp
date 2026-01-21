use chrono::NaiveDateTime;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::model::{FraudLevel, LabelSource, ScoringModelType};

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
        fn to() -> RelationDef {
            Relation::Transaction.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// Transactions
pub mod transaction {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "transactions")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub payload_number: String,
        pub transaction_version: i32,
        pub is_latest: bool,
        pub payload: serde_json::Value,
        pub schema_version_major: i32,
        pub schema_version_minor: i32,
        pub label_id: Option<i64>,
        pub comment: Option<String>,
        pub last_scoring_date: Option<NaiveDateTime>,
        pub processing_complete: bool,
        pub created_at: NaiveDateTime,
        pub updated_at: NaiveDateTime,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::label::Entity",
            from = "Column::LabelId",
            to = "super::label::Column::Id"
        )]
        Label,
        #[sea_orm(has_many = "super::scoring_event::Entity")]
        ScoringEvent,
        #[sea_orm(has_many = "super::feature::Entity")]
        Feature,
    }

    impl Related<super::label::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Label.def()
        }
    }

    impl Related<super::scoring_event::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::ScoringEvent.def()
        }
    }

    impl Related<super::feature::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Feature.def()
        }
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
        #[sea_orm(has_many = "super::expression_rule::Entity")]
        ScoringRule,
        #[sea_orm(has_many = "super::channel_model_activation::Entity")]
        ChannelModelActivation,
    }

    impl Related<super::channel::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Channel.def()
        }
    }
    impl Related<super::expression_rule::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::ScoringRule.def()
        }
    }
    impl Related<super::channel_model_activation::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::ChannelModelActivation.def()
        }
    }

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
        #[sea_orm(
            belongs_to = "super::scoring_model::Entity",
            from = "Column::ModelId",
            to = "super::scoring_model::Column::Id"
        )]
        Model,
        #[sea_orm(has_many = "super::channel_model_activation::Entity")]
        ChannelModelActivation,
    }

    impl Related<super::scoring_model::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Model.def()
        }
    }
    impl Related<super::channel_model_activation::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::ChannelModelActivation.def()
        }
    }

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
        pub activation_id: i64,
        pub total_score: i32,
        pub created_at: NaiveDateTime,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::transaction::Entity",
            from = "Column::TransactionId",
            to = "super::transaction::Column::Id"
        )]
        Transaction,
        #[sea_orm(
            belongs_to = "super::channel_model_activation::Entity",
            from = "Column::ActivationId",
            to = "super::channel_model_activation::Column::Id"
        )]
        ChannelModelActivation,
        #[sea_orm(has_many = "super::triggered_rule::Entity")]
        TriggeredRule,
    }

    impl Related<super::transaction::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Transaction.def()
        }
    }
    impl Related<super::channel_model_activation::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::ChannelModelActivation.def()
        }
    }
    impl Related<super::triggered_rule::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::TriggeredRule.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// Channel-Model Activations
pub mod channel_model_activation {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "channel_model_activations")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub channel_id: i64,
        pub model_id: i64,
        pub created_at: NaiveDateTime,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::channel::Entity",
            from = "Column::ChannelId",
            to = "super::channel::Column::Id"
        )]
        Channel,
        #[sea_orm(
            belongs_to = "super::scoring_model::Entity",
            from = "Column::ModelId",
            to = "super::scoring_model::Column::Id"
        )]
        ScoringModel,
        #[sea_orm(has_many = "super::scoring_event::Entity")]
        ScoringEvent,
    }

    impl Related<super::channel::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Channel.def()
        }
    }
    impl Related<super::scoring_model::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::ScoringModel.def()
        }
    }
    impl Related<super::scoring_event::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::ScoringEvent.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// Scoring Rules
pub mod expression_rule {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "expression_rules")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub model_id: i64,
        pub name: String,
        pub description: Option<String>,
        pub rule: String,
        pub score: i32,
        pub created_at: NaiveDateTime,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::scoring_model::Entity",
            from = "Column::ModelId",
            to = "super::scoring_model::Column::Id"
        )]
        Model,
        #[sea_orm(has_many = "super::triggered_rule::Entity")]
        TriggeredRule,
    }

    impl Related<super::scoring_model::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Model.def()
        }
    }
    impl Related<super::triggered_rule::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::TriggeredRule.def()
        }
    }

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
        #[sea_orm(
            belongs_to = "super::scoring_event::Entity",
            from = "Column::ScoringEventsId",
            to = "super::scoring_event::Column::Id"
        )]
        ScoringEvent,
        #[sea_orm(
            belongs_to = "super::expression_rule::Entity",
            from = "Column::RuleId",
            to = "super::expression_rule::Column::Id"
        )]
        ScoringRule,
    }

    impl Related<super::scoring_event::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::ScoringEvent.def()
        }
    }
    impl Related<super::expression_rule::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::ScoringRule.def()
        }
    }

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
        pub schema_version_major: i32,
        pub schema_version_minor: i32,
        pub simple_features: Option<serde_json::Value>,
        pub graph_features: serde_json::Value,
        pub created_at: NaiveDateTime,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::transaction::Entity",
            from = "Column::TransactionId",
            to = "super::transaction::Column::Id"
        )]
        Transaction,
    }

    impl Related<super::transaction::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Transaction.def()
        }
    }
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

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::match_node::Entity",
            from = "Column::NodeId",
            to = "super::match_node::Column::Id"
        )]
        MatchNode,
    }

    impl Related<super::match_node::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::MatchNode.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}
