use crate::model::LabelSource;
use crate::model::expression_rule::Model as ExpressionRule;
use crate::model::processible::{ColumnValueTrait, Filter, ProcessibleSerde};
use crate::model::sea_orm_storage_model as entities;
use crate::model::*;
use crate::model::{Channel, Feature, MatcherConfig, ScoringEvent, TriggeredRule};
use crate::storage::common::CommonStorage;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use jsonschema::validate;
use sea_orm::entity::prelude::*;
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Database, DatabaseConnection, DbBackend, EntityTrait,
    FromQueryResult, IntoActiveModel, NotSet, QueryFilter, QueryOrder, Set, Statement,
    TransactionTrait,
};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::error::Error;
use std::marker::PhantomData;
use tracing::debug;

// Default implementation for MatcherConfig
fn default_matcher_config() -> MatcherConfig {
    (80, 50) // Default confidence and importance
}

#[derive(Clone)]
pub struct ProdCommonStorage<P: ProcessibleSerde> {
    pub db: DatabaseConnection,
    pub matcher_configs: HashMap<String, MatcherConfig>,
    pub _phantom: PhantomData<P>,
}

impl<P: ProcessibleSerde> ProdCommonStorage<P> {
    pub async fn new(database_url: &str) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let db = Database::connect(database_url).await?;
        Ok(Self {
            db,
            matcher_configs: Self::default_configs(),
            _phantom: PhantomData,
        })
    }

    pub async fn with_configs(
        database_url: &str,
        matcher_configs: HashMap<String, MatcherConfig>,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let db = Database::connect(database_url).await?;
        Ok(Self {
            db,
            matcher_configs,
            _phantom: PhantomData,
        })
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

    pub fn validate_features(
        &self,
        features: &[Feature],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let features_json = serde_json::to_value(features)?;
        debug!(
            "Raw features JSON string: {}",
            serde_json::to_string(&features_json)?
        );
        let validation_result = validate(&self.get_features_schema(), &features_json);
        if let Err(errors) = validation_result {
            debug!("Validation error details: {:?}", errors);
            return Err(format!("Feature validation failed: {:?}", errors).into());
        }
        Ok(())
    }

    fn get_filter_statement(&self, filters: &[Filter<Box<dyn ColumnValueTrait>>]) -> String {
        let columns = <P as ProcessibleSerde>::list_column_fields();

        let base_stmt = "SELECT * FROM transactions".to_string();
        if filters.is_empty() {
            return base_stmt;
        }

        let filters_stmt = filters
            .iter()
            .map(|filter| {
                let maybe_filter_statement = columns
                    .iter()
                    .find(|column| column.column == filter.column)
                    .expect(&format!("Column not found for filter {}", filter.column))
                    .filter_statement
                    .clone();

                let filter_statement = maybe_filter_statement.expect(&format!(
                    "Filter is not defined for column {}",
                    filter.column
                ));

                filter_statement(&filter)
            })
            .collect::<Vec<String>>()
            .join(" AND ");

        base_stmt + &" WHERE " + &filters_stmt
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
                    },
                    "type": {
                        "type": "string",
                        "enum": [
                            "integer",
                            "double",
                            "string",
                            "boolean",
                            "datetime",
                            "integer_array",
                            "double_array",
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
                                    "type": { "const": "integer" },
                                    "value": { "type": "number" }
                                }
                            },
                            {
                                "properties": {
                                    "type": { "const": "double" },
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
                                    "type": { "const": "double_array" },
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
impl<P: ProcessibleSerde> CommonStorage for ProdCommonStorage<P> {
    async fn insert_transaction(
        &self,
        payload_number: String,
        payload: serde_json::Value,
        schema_version: SchemaVersion,
    ) -> Result<ModelId, Box<dyn Error + Send + Sync>> {
        use sea_orm::{ActiveValue::Set, QueryOrder};

        let txn = self.db.begin().await?;
        let existing = entities::transaction::Entity::find()
            .filter(entities::transaction::Column::PayloadNumber.eq(payload_number.clone()))
            .order_by_desc(entities::transaction::Column::TransactionVersion)
            .one(&txn)
            .await?;

        let (next_version, carried_label, carried_comment) = if let Some(row) = existing {
            entities::transaction::Entity::update_many()
                .col_expr(entities::transaction::Column::IsLatest, Expr::value(false))
                .filter(entities::transaction::Column::PayloadNumber.eq(payload_number.clone()))
                .exec(&txn)
                .await?;
            (
                row.transaction_version + 1,
                row.label_id,
                row.comment.clone(),
            )
        } else {
            (1, None, None)
        };

        let now = chrono::Utc::now().naive_utc();
        let model = entities::transaction::ActiveModel {
            id: NotSet,
            payload_number: Set(payload_number),
            transaction_version: Set(next_version),
            is_latest: Set(true),
            payload: Set(payload),
            schema_version_major: Set(schema_version.0),
            schema_version_minor: Set(schema_version.1),
            label_id: Set(carried_label),
            comment: Set(carried_comment),
            last_scoring_date: Set(None),
            processing_complete: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
        };
        let res = model.insert(&txn).await?;
        txn.commit().await?;
        Ok(res.id)
    }

    async fn get_transaction(
        &self,
        transaction_id: ModelId,
    ) -> Result<Transaction, Box<dyn Error + Send + Sync>> {
        let model = entities::transaction::Entity::find_by_id(transaction_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| format!("Transaction not found: {}", transaction_id))?;
        Ok(model.into())
    }

    async fn get_latest_transaction_by_payload(
        &self,
        payload_number: &str,
    ) -> Result<Option<Transaction>, Box<dyn Error + Send + Sync>> {
        use sea_orm::QueryOrder;

        let model = entities::transaction::Entity::find()
            .filter(entities::transaction::Column::PayloadNumber.eq(payload_number.to_string()))
            .order_by_desc(entities::transaction::Column::TransactionVersion)
            .one(&self.db)
            .await?;
        Ok(model.map(|row| row.into()))
    }

    async fn list_transaction_versions(
        &self,
        payload_number: &str,
    ) -> Result<Vec<Transaction>, Box<dyn Error + Send + Sync>> {
        use sea_orm::QueryOrder;

        let models = entities::transaction::Entity::find()
            .filter(entities::transaction::Column::PayloadNumber.eq(payload_number.to_string()))
            .order_by_desc(entities::transaction::Column::TransactionVersion)
            .all(&self.db)
            .await?;

        Ok(models.into_iter().map(|row| row.into()).collect())
    }

    async fn filter_transactions(
        &self,
        filters: &[Filter<Box<dyn ColumnValueTrait>>],
    ) -> Result<Vec<Transaction>, Box<dyn Error + Send + Sync>> {
        let sql = self.get_filter_statement(filters);

        let transactions = self
            .db
            .query_all(Statement::from_string(DbBackend::Postgres, sql))
            .await?
            .into_iter()
            .map(|row| {
                Transaction::from_query_result(&row, "")
                    .expect("Failed to convert row to transaction")
            })
            .collect();

        Ok(transactions)
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
        let now = chrono::Utc::now().naive_utc();
        txn.processing_complete = Set(true);
        txn.last_scoring_date = Set(Some(now));
        txn.updated_at = Set(now);
        txn.update(&self.db).await?;

        Ok(())
    }

    async fn get_activation_by_channel_id(
        &self,
        channel_id: ModelId,
    ) -> Result<channel_model_activation::Model, Box<dyn Error + Send + Sync>> {
        let activation = entities::channel_model_activation::Entity::find()
            .filter(entities::channel_model_activation::Column::ChannelId.eq(channel_id))
            .order_by_desc(entities::channel_model_activation::Column::CreatedAt)
            .one(&self.db)
            .await?
            .ok_or_else(|| format!("Activation not found for channel: {}", channel_id))?;
        Ok(activation)
    }

    async fn get_expression_rules(
        &self,
        channel_id: ModelId,
    ) -> Result<Vec<ExpressionRule>, Box<dyn Error + Send + Sync>> {
        let rules = entities::expression_rule::Entity::find()
            .filter(entities::expression_rule::Column::ModelId.eq(channel_id))
            .all(&self.db)
            .await?;
        let rules: Vec<ExpressionRule> = rules
            .into_iter()
            .map(|row| ExpressionRule {
                id: row.id,
                model_id: row.model_id,
                name: row.name,
                description: row.description,
                rule: row.rule,
                score: row.score,
                created_at: row.created_at,
            })
            .collect();
        Ok(rules)
    }

    async fn get_channel_by_name(
        &self,
        name: &str,
    ) -> Result<Option<Channel>, Box<dyn Error + Send + Sync>> {
        let row = entities::channel::Entity::find()
            .filter(entities::channel::Column::Name.eq(name.to_string()))
            .one(&self.db)
            .await?;
        Ok(row.map(|r| Channel {
            id: r.id,
            name: r.name,
            model_id: r.model_id,
            created_at: r.created_at,
        }))
    }

    async fn save_scores(
        &self,
        transaction_id: ModelId,
        activation_id: ModelId,
        total_score: i32,
        triggered_rules: &[ModelId],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let txn = self.db.begin().await?;

        let scoring_am = entities::scoring_event::ActiveModel {
            id: NotSet,
            transaction_id: Set(transaction_id),
            activation_id: Set(activation_id),
            total_score: Set(total_score),
            created_at: Set(chrono::Utc::now().naive_utc()),
        };
        let scoring = scoring_am.insert(&txn).await?;

        for rule in triggered_rules {
            let tr = entities::triggered_rule::ActiveModel {
                id: NotSet,
                scoring_events_id: Set(scoring.id),
                rule_id: Set(*rule),
            };
            tr.insert(&txn).await?;
        }

        txn.commit().await?;
        Ok(())
    }

    async fn save_features<'a>(
        &self,
        transaction_id: ModelId,
        simple_features: &'a Option<&'a [Feature]>,
        graph_features: &'a [Feature],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Validate features against schema if one is provided

        self.validate_features(graph_features)?;

        let graph_json_value = serde_json::to_value(graph_features)?;
        if let Some(features) = simple_features {
            self.validate_features(features)?;
        }

        // Try find existing by transaction_id
        let existing = entities::feature::Entity::find()
            .filter(entities::feature::Column::TransactionId.eq(transaction_id))
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
                    schema_version_major: Set(1),
                    schema_version_minor: Set(0),
                    simple_features: Set(simple_features
                        .map(|f| serde_json::to_value(f).ok())
                        .flatten()),
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
            }
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
        filter_config: Option<serde_json::Value>,
        min_confidence: Option<i32>,
    ) -> Result<Vec<ConnectedTransaction>, Box<dyn Error + Send + Sync>> {
        // Convert Option parameters to SQL nulls if None
        let min_confidence_sql = min_confidence.unwrap_or(0);
        let filter_config_sql = match filter_config {
            Some(v) => {
                let mut s = v.to_string();
                // Escape single quotes for SQL literal
                s = s.replace('\'', "''");
                format!("'{}'::jsonb", s)
            }
            None => "NULL".to_string(),
        };

        // Call the SQL function with explicit type casts for PostgreSQL
        // The function now expects 6 parameters (added filter_config jsonb; min_created_at/max_created_at removed)
        let sql = format!(
            "SELECT transaction_id, path_matchers, path_values, depth, confidence, importance, created_at \
             FROM find_connected_transactions({}, {}, {}, {}, {}, {})",
            transaction_id,
            max_depth
                .map(|v| v.to_string())
                .unwrap_or("NULL".to_string()),
            limit_count
                .map(|v| v.to_string())
                .unwrap_or("NULL".to_string()),
            filter_config_sql,
            min_confidence_sql,
            1
        );
        let rows = self
            .db
            .query_all(Statement::from_string(DbBackend::Postgres, sql))
            .await?;

        // Map SQL results to Rust structs
        let mut connected_transactions = Vec::new();
        for row in rows {
            let transaction_id: Option<i64> = row.try_get("", "transaction_id").ok();
            let path_matchers: Option<Vec<String>> = row.try_get("", "path_matchers").ok();
            let path_values: Option<Vec<String>> = row.try_get("", "path_values").ok();
            let depth: Option<i32> = row.try_get("", "depth").ok();
            let confidence: Option<i32> = row.try_get("", "confidence").ok();
            let importance: Option<i32> = row.try_get("", "importance").ok();
            let created_at_naive: Option<chrono::NaiveDateTime> =
                row.try_get("", "created_at").ok();

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
        transaction_id: ModelId,
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
             WHERE t1.id = {} AND t2.id != {} AND t2.is_latest = TRUE AND t2.payload_number <> t1.payload_number \
             ORDER BY t2.id, mn.matcher",
            transaction_id, transaction_id
        );
        let rows = self
            .db
            .query_all(Statement::from_string(DbBackend::Postgres, sql))
            .await?;

        // Map SQL results to Rust structs
        let mut direct_connections = Vec::new();
        for row in rows {
            let transaction_id: i64 = row.try_get("", "transaction_id")?;
            let matcher: String = row.try_get("", "matcher")?;
            let confidence: i32 = row.try_get("", "confidence")?;
            let importance: i32 = row.try_get("", "importance")?;
            let created_at_naive: chrono::NaiveDateTime = row.try_get("", "created_at")?;
            let created_at = DateTime::<Utc>::from_naive_utc_and_offset(created_at_naive, Utc);

            direct_connections.push(DirectConnection {
                transaction_id,
                matcher,
                confidence,
                importance,
                created_at,
            });
        }

        Ok(direct_connections)
    }

    async fn save_matching_fields_with_timespace(
        &self,
        transaction_id: ModelId,
        matching_fields: &[MatchingField],
        datetime_alpha: Option<chrono::DateTime<chrono::Utc>>,
        datetime_beta: Option<chrono::DateTime<chrono::Utc>>,
        long_alpha: Option<f64>,
        lat_alpha: Option<f64>,
        long_beta: Option<f64>,
        lat_beta: Option<f64>,
        long_gamma: Option<f64>,
        lat_gamma: Option<f64>,
        long_delta: Option<f64>,
        lat_delta: Option<f64>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        if matching_fields.is_empty() {
            return Ok(());
        }

        let txn = self.db.begin().await?;

        for field in matching_fields {
            let (conf, imp) = self
                .matcher_configs
                .get(&field.matcher)
                .cloned()
                .unwrap_or_else(default_matcher_config);

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
                    datetime_alpha: Set(datetime_alpha.map(|dt| dt.naive_utc())),
                    datetime_beta: Set(datetime_beta.map(|dt| dt.naive_utc())),
                    long_alpha: Set(long_alpha),
                    lat_alpha: Set(lat_alpha),
                    long_beta: Set(long_beta),
                    lat_beta: Set(lat_beta),
                    long_gamma: Set(long_gamma),
                    lat_gamma: Set(lat_gamma),
                    long_delta: Set(long_delta),
                    lat_delta: Set(lat_delta),
                    created_at: Set(chrono::Utc::now().naive_utc()),
                };
                link.insert(&txn).await?;
            }
        }

        txn.commit().await?;
        debug!(
            "Successfully saved {} matching fields for transaction {}",
            matching_fields.len(),
            transaction_id
        );
        Ok(())
    }

    async fn get_channels(
        &self,
        model_id: ModelId,
    ) -> Result<Vec<Channel>, Box<dyn Error + Send + Sync>> {
        let rows = entities::channel::Entity::find()
            .filter(entities::channel::Column::ModelId.eq(model_id))
            .all(&self.db)
            .await?;

        let channels: Vec<Channel> = rows
            .into_iter()
            .map(|row| Channel {
                id: row.id,
                name: row.name,
                model_id: row.model_id,
                created_at: row.created_at,
            })
            .collect();

        Ok(channels)
    }

    async fn get_scoring_events(
        &self,
        transaction_id: ModelId,
    ) -> Result<Vec<ScoringEvent>, Box<dyn Error + Send + Sync>> {
        let rows = entities::scoring_event::Entity::find()
            .filter(entities::scoring_event::Column::TransactionId.eq(transaction_id))
            .order_by_desc(entities::scoring_event::Column::CreatedAt)
            .all(&self.db)
            .await?;
        let events: Vec<entities::scoring_event::Model> = rows.into_iter().map(|row| row).collect();

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
            .map(|row| TriggeredRule {
                id: row.id,
                scoring_events_id: row.scoring_events_id,
                rule_id: row.rule_id,
            })
            .collect();

        Ok(rules)
    }

    async fn label_transactions(
        &self,
        transaction_ids: &[ModelId],
        fraud_level: &FraudLevel,
        fraud_category: &String,
        label_source: &LabelSource,
        labeled_by: &String,
    ) -> Result<Vec<ModelId>, Box<dyn Error + Send + Sync>> {
        use sea_orm::QueryOrder;

        let mut new_transaction_ids = Vec::new();

        for &transaction_id in transaction_ids {
            let txn = self.db.begin().await?;

            let source = entities::transaction::Entity::find_by_id(transaction_id)
                .one(&txn)
                .await?
                .ok_or_else(|| format!("Transaction not found: {}", transaction_id))?;

            let latest_version = entities::transaction::Entity::find()
                .filter(
                    entities::transaction::Column::PayloadNumber.eq(source.payload_number.clone()),
                )
                .order_by_desc(entities::transaction::Column::TransactionVersion)
                .one(&txn)
                .await?
                .map(|row| row.transaction_version)
                .unwrap_or(0);

            let label_model = entities::label::ActiveModel {
                id: NotSet,
                fraud_level: Set(*fraud_level),
                fraud_category: Set(fraud_category.clone()),
                label_source: Set(*label_source),
                labeled_by: Set(labeled_by.clone()),
                created_at: Set(chrono::Utc::now().naive_utc()),
            }
            .insert(&txn)
            .await?;
            let label_id = label_model.id;

            entities::transaction::Entity::update_many()
                .col_expr(entities::transaction::Column::IsLatest, Expr::value(false))
                .filter(
                    entities::transaction::Column::PayloadNumber.eq(source.payload_number.clone()),
                )
                .exec(&txn)
                .await?;

            let now = chrono::Utc::now().naive_utc();
            let new_tx = entities::transaction::ActiveModel {
                id: NotSet,
                payload_number: Set(source.payload_number.clone()),
                transaction_version: Set(latest_version + 1),
                is_latest: Set(true),
                payload: Set(source.payload.clone()),
                schema_version_major: Set(source.schema_version_major),
                schema_version_minor: Set(source.schema_version_minor),
                label_id: Set(Some(label_id)),
                comment: Set(source.comment.clone()),
                last_scoring_date: Set(None),
                processing_complete: Set(false),
                created_at: Set(now),
                updated_at: Set(now),
            }
            .insert(&txn)
            .await?;

            txn.commit().await?;
            new_transaction_ids.push(new_tx.id);
        }

        Ok(new_transaction_ids)
    }
}
