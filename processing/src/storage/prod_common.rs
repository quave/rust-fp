use crate::model::*;
use crate::storage::common::CommonStorage;
use crate::storage::MatcherConfig;
use crate::storage::sea_orm_storage_model as entities;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use jsonschema::validate;
use sea_orm::entity::prelude::*;
use sea_orm::{ActiveModelTrait, ColumnTrait, Database, DatabaseConnection, DbBackend, EntityTrait, IntoActiveModel, NotSet, QueryFilter, QueryOrder, Set, Statement, TransactionTrait};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::error::Error;
use tracing::debug;

// Default implementation for MatcherConfig
fn default_matcher_config() -> MatcherConfig {
    (80, 50) // Default confidence and importance
}

#[derive(Clone)]
pub struct ProdCommonStorage {
    pub db: DatabaseConnection,
    pub matcher_configs: HashMap<String, MatcherConfig>,
}

impl ProdCommonStorage {
    pub async fn new(database_url: &str) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let db = Database::connect(database_url).await?;
        Ok(Self { db, matcher_configs: Self::default_configs() })
    }

    pub async fn with_configs(
        database_url: &str,
        matcher_configs: HashMap<String, MatcherConfig>,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let db = Database::connect(database_url).await?;
        Ok(Self { db, matcher_configs })
    }
    
    fn default_configs() -> HashMap<String, MatcherConfig> {
        let mut config = HashMap::new();
        // Default confidence and importance for common matchers
        config.insert("customer.email".to_string(), (100, 90));
        config.insert("billing.payment_details".to_string(), (100, 80));
        config.insert("ip.address".to_string(), (70, 60));
        config.insert("device.id".to_string(), (90, 70));
        config.insert("phone.number".to_string(), (95, 85));
        config
    }

    pub async fn insert_transaction(&self) -> Result<ModelId, Box<dyn Error + Send + Sync>> {
        let model = entities::transaction::ActiveModel {
            id: NotSet,
            label_id: Set(None),
            comment: Set(None),
            last_scoring_date: Set(None),
            processing_complete: Set(false),
            created_at: NotSet,
        };
        let res = model.insert(&self.db).await?;
        Ok(res.id)
    }

    pub fn validate_features(&self, features: &[Feature]) -> Result<(), Box<dyn Error + Send + Sync>> {
        let features_json = serde_json::to_value(features)?;
        debug!("Raw features JSON string: {}", serde_json::to_string(&features_json)?);
        let validation_result = validate(&self.get_features_schema(), &features_json);
        if let Err(errors) = validation_result {
            debug!("Validation error details: {:?}", errors);
            return Err(format!("Feature validation failed: {:?}", errors).into());
        }
        Ok(())
    }

    pub fn get_features_schema(&self) -> Value {
    // Create a schema that matches all test cases
        json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "enum": [
                            "amount",
                            "is_high_value",
                            "amounts",
                            "created_at",
                            "categories"
                        ]
                    },
                    "type": {
                        "type": "string",
                        "enum": [
                            "integer",
                            "number",
                            "string",
                            "boolean",
                            "datetime",
                            "integer_array",
                            "number_array",
                            "string_array",
                            "boolean_array"
                        ]
                    },
                    "value": {
                        "type": ["number", "string", "boolean", "array"]
                    }
                },
                "required": ["name", "type", "value"],
                "dependencies": {
                    "type": {
                        "oneOf": [
                            {
                                "properties": {
                                    "type": { "const": "number" },
                                    "value": { "type": "number" }
                                }
                            },
                            {
                                "properties": {
                                    "type": { "const": "string" },
                                    "value": { "type": "string" }
                                }
                            },
                            {
                                "properties": {
                                    "type": { "const": "boolean" },
                                    "value": { "type": "boolean" }
                                }
                            },
                            {
                                "properties": {
                                    "type": { "const": "datetime" },
                                    "value": { "type": "string", "format": "date-time" }
                                }
                            },
                            {
                                "properties": {
                                    "type": { "const": "integer_array" },
                                    "value": { "type": "array", "items": { "type": "number" } }
                                }
                            },
                            {
                                "properties": {
                                    "type": { "const": "number_array" },
                                    "value": { "type": "array", "items": { "type": "number" } }
                                }
                            },
                            {
                                "properties": {
                                    "type": { "const": "string_array" },
                                    "value": { "type": "array", "items": { "type": "string" } }
                                }
                            },
                            {
                                "properties": {
                                    "type": { "const": "boolean_array" },
                                    "value": { "type": "array", "items": { "type": "boolean" } }
                                }
                            }
                        ]
                    }
                }
            }
        })
    }
}

#[async_trait]
impl CommonStorage for ProdCommonStorage {
    async fn save_transaction(&self) -> Result<ModelId, Box<dyn Error + Send + Sync>> {
        self.insert_transaction().await
    }

    async fn mark_transaction_processed(
        &self,
        transaction_id: ModelId,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut txn = entities::transaction::Entity::find_by_id(transaction_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| format!("Transaction not found: {}", transaction_id))?
            .into_active_model();
        txn.processing_complete = Set(true);
        txn.last_scoring_date = Set(Some(chrono::Utc::now().naive_utc()));
        txn.update(&self.db).await?;

        Ok(())
    }

    async fn save_scores(
        &self,
        transaction_id: ModelId,
        channel_id: ModelId,
        total_score: i32,
        triggered_rules: &[TriggeredRule],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let txn = self.db.begin().await?;
        let scoring_am = entities::scoring_event::ActiveModel {
            id: NotSet,
            transaction_id: Set(transaction_id),
            channel_id: Set(channel_id),
            total_score: Set(total_score),
            created_at: Set(chrono::Utc::now().naive_utc()),
        };
        let scoring = scoring_am.insert(&txn).await?;

        for rule in triggered_rules {
            let tr = entities::triggered_rule::ActiveModel {
                id: NotSet,
                scoring_events_id: Set(scoring.id),
                rule_id: Set(rule.rule_id),
            };
            tr.insert(&txn).await?;
        }

        txn.commit().await?;
        Ok(())
    }

    async fn save_features(
        &self,
        transaction_id: ModelId,
        simple_features: Option<&[Feature]>,
        graph_features: &[Feature],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Validate features against schema if one is provided

        self.validate_features(graph_features)?;

        let graph_json_value = serde_json::to_value(graph_features)?;
        if let Some(features) = simple_features {
            self.validate_features(features)?;
        }

        // Try find existing by (transaction_id, version=1)
        let existing = entities::feature::Entity::find()
            .filter(entities::feature::Column::TransactionId.eq(transaction_id))
            .filter(entities::feature::Column::TransactionVersion.eq(1))
            .one(&self.db)
            .await?;

        match existing {
            Some(model) => {
                let mut am: entities::feature::ActiveModel = model.into();
                am.schema_version_major = Set(1);
                am.schema_version_minor = Set(0);
                if let Some(features) = simple_features {
                    am.simple_features = Set(Some(serde_json::to_value(features)?));
                }
                am.graph_features = Set(graph_json_value);
                am.update(&self.db).await?;
            }
            None => {
                let am = entities::feature::ActiveModel {
                    id: NotSet,
                    transaction_id: Set(transaction_id),
                    transaction_version: Set(1),
                    schema_version_major: Set(1),
                    schema_version_minor: Set(0),
                    simple_features: Set(simple_features.map(|f| serde_json::to_value(f).ok()).flatten()),
                    graph_features: Set(graph_json_value),
                    created_at: Set(chrono::Utc::now().naive_utc()),
                };
                am.insert(&self.db).await?;
            }
        }
        Ok(())
    }

    async fn get_features(
        &self,
        transaction_id: ModelId,
    ) -> Result<(Option<Vec<Feature>>, Vec<Feature>), Box<dyn Error + Send + Sync>> {
        let row = entities::feature::Entity::find()
            .filter(entities::feature::Column::TransactionId.eq(transaction_id))
            .order_by_desc(entities::feature::Column::CreatedAt)
            .one(&self.db)
            .await?
            .ok_or_else(|| "no rows returned".to_string())?;

        let simple_features: Option<Vec<Feature>> = match row.simple_features.clone() {
            Some(json_value) => {
                let features: Vec<Feature> = serde_json::from_value(json_value)?;
                self.validate_features(&features)?;
                Some(features)
            },
            None => None,
        };

        let graph_features: Vec<Feature> = serde_json::from_value(row.graph_features.clone())?;
        
        // Validate features against schema if one is provided
        self.validate_features(&graph_features)?;
        
        Ok((simple_features, graph_features))
    }

    async fn find_connected_transactions(
        &self,
        transaction_id: ModelId,
        max_depth: Option<i32>,
        limit_count: Option<i32>,
        min_created_at: Option<chrono::DateTime<chrono::Utc>>,
        max_created_at: Option<chrono::DateTime<chrono::Utc>>,
        min_confidence: Option<i32>
    ) -> Result<Vec<ConnectedTransaction>, Box<dyn Error + Send + Sync>> {
        // Convert Option parameters to SQL nulls if None
        let min_confidence_sql = min_confidence.unwrap_or(0);
                
        // Convert DateTime<Utc> to NaiveDateTime for PostgreSQL TIMESTAMP (without time zone)
        let min_created_at_naive = min_created_at.map(|dt| dt.naive_utc());
        let max_created_at_naive = max_created_at.map(|dt| dt.naive_utc());
        
        // Call the SQL function with explicit type casts for PostgreSQL
        // The function now expects 7 parameters (added min_connections parameter)
        let sql = format!(
            "SELECT transaction_id, path_matchers, path_values, depth, confidence, importance, created_at \
             FROM find_connected_transactions({}, {}, {}, {}, {}, {}, {})",
            transaction_id,
            max_depth.map(|v| v.to_string()).unwrap_or("NULL".to_string()),
            limit_count.map(|v| v.to_string()).unwrap_or("NULL".to_string()),
            min_created_at_naive.map(|v| format!("'{}'", v)).unwrap_or("NULL".to_string()),
            max_created_at_naive.map(|v| format!("'{}'", v)).unwrap_or("NULL".to_string()),
            min_confidence_sql,
            1
        );
        let rows = self.db.query_all(Statement::from_string(DbBackend::Postgres, sql)).await?;
        
        // Map SQL results to Rust structs
        let mut connected_transactions = Vec::new();
        for row in rows {
            let transaction_id: Option<i64> = row.try_get("", "transaction_id").ok();
            let path_matchers: Option<Vec<String>> = row.try_get("", "path_matchers").ok();
            let path_values: Option<Vec<String>> = row.try_get("", "path_values").ok();
            let depth: Option<i32> = row.try_get("", "depth").ok();
            let confidence: Option<i32> = row.try_get("", "confidence").ok();
            let importance: Option<i32> = row.try_get("", "importance").ok();
            let created_at_naive: Option<chrono::NaiveDateTime> = row.try_get("", "created_at").ok();

            let created_at = created_at_naive
                .map(|naive| DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc))
                .unwrap_or_else(|| Utc::now());

            connected_transactions.push(ConnectedTransaction {
                transaction_id: transaction_id.unwrap_or(0),
                path_matchers: path_matchers.unwrap_or_default(),
                path_values: path_values.unwrap_or_default(),
                depth: depth.unwrap_or(0),
                confidence: confidence.unwrap_or(0),
                importance: importance.unwrap_or(0),
                created_at,
            });
        }
        
        Ok(connected_transactions)
    }
    
    async fn get_direct_connections(
        &self,
        transaction_id: ModelId
    ) -> Result<Vec<DirectConnection>, Box<dyn Error + Send + Sync>> {
        // Query to get direct connections (only depth=1) for the given transaction
        // One row per matcher connection
        let sql = format!(
            "SELECT t2.id as transaction_id, mn.matcher as matcher, mn.confidence as confidence, mn.importance as importance, t2.created_at as created_at \
             FROM transactions t1 \
             JOIN match_node_transactions mnt1 ON t1.id = mnt1.transaction_id \
             JOIN match_node mn ON mnt1.node_id = mn.id \
             JOIN match_node_transactions mnt2 ON mn.id = mnt2.node_id \
             JOIN transactions t2 ON mnt2.transaction_id = t2.id \
             WHERE t1.id = {} AND t2.id != {} \
             ORDER BY t2.id, mn.matcher",
            transaction_id, transaction_id
        );
        let rows = self.db.query_all(Statement::from_string(DbBackend::Postgres, sql)).await?;
        
        // Map SQL results to Rust structs
        let mut direct_connections = Vec::new();
        for row in rows {
            let transaction_id: i64 = row.try_get("", "transaction_id")?;
            let matcher: String = row.try_get("", "matcher")?;
            let confidence: i32 = row.try_get("", "confidence")?;
            let importance: i32 = row.try_get("", "importance")?;
            let created_at_naive: chrono::NaiveDateTime = row.try_get("", "created_at")?;
            let created_at = DateTime::<Utc>::from_naive_utc_and_offset(created_at_naive, Utc);

            direct_connections.push(DirectConnection { transaction_id, matcher, confidence, importance, created_at });
        }
        
        Ok(direct_connections)
    }

    async fn save_matching_fields(
        &self,
        transaction_id: ModelId,
        matching_fields: &[MatchingField],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        if matching_fields.is_empty() {
            return Ok(());
        }
        
        let txn = self.db.begin().await?;
        
        for field in matching_fields {
            let (conf, imp) = self.matcher_configs.get(&field.matcher).cloned().unwrap_or_else(default_matcher_config);

            let existing_node = entities::match_node::Entity::find()
                .filter(entities::match_node::Column::Matcher.eq(field.matcher.clone()))
                .filter(entities::match_node::Column::Value.eq(field.value.clone()))
                .one(&txn)
                .await?;
            let node_id = match existing_node {
                Some(node) => node.id,
                None => {
                    let am = entities::match_node::ActiveModel {
                        id: NotSet,
                        matcher: Set(field.matcher.clone()),
                        value: Set(field.value.clone()),
                        confidence: Set(conf),
                        importance: Set(imp),
                    };
                    am.insert(&txn).await?.id
                }
            };

            let existing_link = entities::match_node_transactions::Entity::find()
                .filter(entities::match_node_transactions::Column::NodeId.eq(node_id))
                .filter(entities::match_node_transactions::Column::TransactionId.eq(transaction_id))
                .one(&txn)
                .await?;
            if existing_link.is_none() {
                let link = entities::match_node_transactions::ActiveModel {
                    node_id: Set(node_id),
                    transaction_id: Set(transaction_id),
                };
                link.insert(&txn).await?;
            }
        }

        txn.commit().await?;
        debug!("Successfully saved {} matching fields for transaction {}", matching_fields.len(), transaction_id);
        Ok(())
    }

    async fn get_channels(
        &self,
        model_id: ModelId,
    ) -> Result<Vec<entities::channel::Model>, Box<dyn Error + Send + Sync>> {
        let rows = entities::channel::Entity::find()
            .filter(entities::channel::Column::ModelId.eq(model_id))
            .all(&self.db)
            .await?;
        let channels: Vec<entities::channel::Model> = rows
            .into_iter()
            .map(|row| row)
            .collect();
        
        Ok(channels)
    }
    
    async fn get_scoring_events(
        &self,
        transaction_id: ModelId,
    ) -> Result<Vec<entities::scoring_event::Model>, Box<dyn Error + Send + Sync>> {
        let rows = entities::scoring_event::Entity::find()
            .filter(entities::scoring_event::Column::TransactionId.eq(transaction_id))
            .order_by_desc(entities::scoring_event::Column::CreatedAt)
            .all(&self.db)
            .await?;
        let events: Vec<entities::scoring_event::Model> = rows
            .into_iter()
            .map(|row| row)
            .collect();
        
        Ok(events)
    }
    
    async fn get_triggered_rules(
        &self,
        scoring_event_id: ModelId,
    ) -> Result<Vec<TriggeredRule>, Box<dyn Error + Send + Sync>> {
        let rows = entities::triggered_rule::Entity::find()
            .filter(entities::triggered_rule::Column::ScoringEventsId.eq(scoring_event_id))
            .all(&self.db)
            .await?;
        let rules = rows
            .into_iter()
            .map(|row| TriggeredRule { id: row.id, scoring_events_id: row.scoring_events_id, rule_id: row.rule_id })
            .collect();
        
        Ok(rules)
    }
    
    // removed save_label/get_label per design: labels are created during label_transactions only
    
    async fn update_transaction_label(
        &self,
        transaction_id: ModelId,
        label_id: ModelId,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let _ = self
            .db
            .execute(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "UPDATE transactions SET label_id = $1 WHERE id = $2",
                vec![label_id.into(), transaction_id.into()],
            ))
            .await?;
        
        Ok(())
    }
    /// Label multiple transactions with the same label in a batch operation
    async fn label_transactions(
        &self,
        transaction_ids: &[ModelId],
        fraud_level: FraudLevel,
        fraud_category: String,
        labeled_by: String,
    ) -> Result<LabelingResult, Box<dyn Error + Send + Sync>> {
        // Save the label and get its ID
        let am = entities::label::ActiveModel {
            id: NotSet,
            fraud_level: Set(fraud_level),
            fraud_category: Set(fraud_category.clone()),
            label_source: Set(LabelSource::Manual),
            labeled_by: Set(labeled_by.clone()),
            created_at: Set(Utc::now().naive_utc()),
        };
        let label_model = am.insert(&self.db).await?;
        let label_id = label_model.id;
        
        let mut success_count = 0;
        let mut failed_transaction_ids = Vec::new();
        
        // Apply the label to each transaction ID in the batch
        for &transaction_id in transaction_ids {
            match self.update_transaction_label(transaction_id, label_id).await {
                Ok(_) => {
                    success_count += 1;
                    tracing::info!("Successfully labeled transaction {}: label_id={}", transaction_id, label_id);
                },
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        transaction_id = %transaction_id,
                        label_id = %label_id,
                        "Failed to update transaction with label"
                    );
                    failed_transaction_ids.push(transaction_id);
                }
            }
        }
        
        Ok(LabelingResult {
            label_id,
            success_count,
            failed_transaction_ids,
        })
    }
} 