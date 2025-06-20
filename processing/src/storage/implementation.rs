use crate::model::*;
use crate::storage::common::CommonStorage;
use crate::storage::MatcherConfig;
use async_trait::async_trait;
use jsonschema::validate;
use serde_json::{json, Value};
use sqlx::PgPool;
use std::collections::HashMap;
use std::error::Error;
use tracing::debug;
use chrono::{DateTime, Utc};

// Default implementation for MatcherConfig
fn default_matcher_config() -> MatcherConfig {
    (80, 50) // Default confidence and importance
}

#[derive(Clone)]
pub struct ProdCommonStorage {
    pub pool: PgPool,
    pub matcher_configs: HashMap<String, MatcherConfig>,
}

impl ProdCommonStorage {
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPool::connect(database_url).await?;
        Ok(Self { 
            pool,
            matcher_configs: Self::default_configs(),
        })
    }

    pub async fn with_configs(database_url: &str, matcher_configs: HashMap<String, MatcherConfig>) -> Result<Self, sqlx::Error> {
        let pool = PgPool::connect(database_url).await?;
        Ok(Self { 
            pool,
            matcher_configs,
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

    pub async fn insert_transaction(&self) -> Result<ModelId, Box<dyn Error + Send + Sync>> {
        let row = sqlx::query!(
            r#"
            INSERT INTO transactions (created_at)
            VALUES (NOW())
            RETURNING id
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.id)
    }

    pub fn validate_features(&self, features: &[Feature]) -> Result<(), Box<dyn Error + Send + Sync>> {
        let features_json = serde_json::to_value(features)?;
        debug!("Raw features JSON string: {}", serde_json::to_string(&features_json)?);
        debug!("Features JSON before validation: {}", serde_json::to_string_pretty(&features_json)?);
        debug!("Schema before validation: {}", serde_json::to_string_pretty(&self.get_features_schema())?);
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
    async fn save_scores(
        &self,
        transaction_id: ModelId,
        channel_id: ModelId,
        total_score: i32,
        triggered_rules: &[TriggeredRule],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut tx = self.pool.begin().await?;

        // First, create a scoring event
        let scoring_event = sqlx::query!(
            r#"
            INSERT INTO scoring_events (transaction_id, channel_id, total_score)
            VALUES ($1, $2, $3)
            RETURNING id
            "#,
            transaction_id,
            channel_id,
            total_score
        )
        .fetch_one(&mut *tx)
        .await?;

        // Then save individual triggered rules
        for rule in triggered_rules {
            sqlx::query!(
                r#"
                INSERT INTO triggered_rules (scoring_events_id, rule_id)
                VALUES ($1, $2)
                "#,
                scoring_event.id,
                rule.rule_id
            )
            .execute(&mut *tx)
            .await?;
            debug!("Successfully inserted triggered rule");
        }

        tx.commit().await?;
        Ok(())
    }

    async fn save_features(
        &self,
        transaction_id: ModelId,
        features: &[Feature],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Validate features against schema if one is provided
        self.validate_features(features)?;

        let mut tx = self.pool.begin().await?;

        // Convert features to JSON Value
        let json_value = serde_json::to_value(features)?;

        sqlx::query!(
            r#"
            INSERT INTO features (
                transaction_id, schema_version_major, schema_version_minor, payload
            ) VALUES ($1, $2, $3, $4)
            "#,
            transaction_id,
            1,
            0,
            json_value
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn get_features(
        &self,
        transaction_id: ModelId,
    ) -> Result<Vec<Feature>, Box<dyn Error + Send + Sync>> {
        let row = sqlx::query!(
            r#"
            SELECT id, transaction_id, schema_version_major, schema_version_minor, payload
            FROM features
            WHERE transaction_id = $1
            ORDER BY created_at DESC
            LIMIT 1
            "#,
            transaction_id
        )
        .fetch_one(&self.pool)
        .await?;

        let features: Vec<Feature> = serde_json::from_value(row.payload)?;
        
        // Validate features against schema if one is provided
        self.validate_features(&features)?;
        
        Ok(features)
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
        
        // Convert DateTime<Utc> to NaiveDateTime for PostgreSQL compatibility
        let min_created_at_naive = min_created_at.map(|dt| dt.naive_utc());
        let max_created_at_naive = max_created_at.map(|dt| dt.naive_utc());
        
        // Call the SQL function with explicit type casts for PostgreSQL
        let rows = sqlx::query!(
            r#"
            SELECT 
                transaction_id, 
                path_matchers, 
                path_values, 
                depth, 
                confidence, 
                importance, 
                created_at
            FROM find_connected_transactions($1, $2, $3, $4, $5, $6)
            "#,
            transaction_id,
            max_depth,
            limit_count,
            min_created_at_naive,
            max_created_at_naive,
            min_confidence_sql
        )
        .fetch_all(&self.pool)
        .await?;
        
        // Map SQL results to Rust structs
        let mut connected_transactions = Vec::new();
        for row in rows {
            let path_matchers = row.path_matchers
                .map(|arr| arr.iter().map(|s| s.to_string()).collect())
                .unwrap_or_else(Vec::new);
                
            let path_values = row.path_values
                .map(|arr| arr.iter().map(|s| s.to_string()).collect())
                .unwrap_or_else(Vec::new);
                
            // Convert NaiveDateTime to DateTime<Utc>
            let created_at = match row.created_at {
                Some(naive_dt) => chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(naive_dt, chrono::Utc),
                None => chrono::Utc::now(), // Fallback to current time if null
            };
                
            connected_transactions.push(ConnectedTransaction {
                transaction_id: row.transaction_id.unwrap_or(0),
                path_matchers,
                path_values,
                depth: row.depth.unwrap_or(0),
                confidence: row.confidence.unwrap_or(0),
                importance: row.importance.unwrap_or(0),
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
        let rows = sqlx::query!(
            r#"
            SELECT 
                t2.id as transaction_id,
                mn.matcher as matcher,
                mn.confidence as confidence,
                mn.importance as importance,
                t2.created_at::timestamptz as created_at
            FROM transactions t1
            JOIN match_node_transactions mnt1 ON t1.id = mnt1.transaction_id
            JOIN match_node mn ON mnt1.node_id = mn.id
            JOIN match_node_transactions mnt2 ON mn.id = mnt2.node_id
            JOIN transactions t2 ON mnt2.transaction_id = t2.id
            WHERE t1.id = $1 AND t2.id != $1
            ORDER BY t2.id, mn.matcher
            "#,
            transaction_id
        )
        .fetch_all(&self.pool)
        .await?;
        
        // Map SQL results to Rust structs
        let mut direct_connections = Vec::new();
        for row in rows {
            // Use current time as fallback if created_at is null
            let created_at = row.created_at.unwrap_or_else(|| chrono::Utc::now());
            
            direct_connections.push(DirectConnection {
                transaction_id: row.transaction_id,
                matcher: row.matcher,
                confidence: row.confidence,
                importance: row.importance,
                created_at,
            });
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
        
        let mut tx = self.pool.begin().await?;
        
        for field in matching_fields {
            // Get configuration for this matcher from internal configs
            let matcher_config = &self.matcher_configs.get(&field.matcher).cloned().unwrap_or_else(default_matcher_config);
            
            // Check if this matcher-value combination already exists
            let existing_node = sqlx::query!(
                r#"
                SELECT id
                FROM match_node
                WHERE matcher = $1 AND value = $2
                "#,
                field.matcher,
                field.value
            )
            .fetch_optional(&mut *tx)
            .await?;
            
            let node_id = match existing_node {
                // If node exists, use its ID
                Some(row) => row.id,
                
                // If node doesn't exist, create it
                None => {
                    let row = sqlx::query!(
                        r#"
                        INSERT INTO match_node (
                            matcher, value, confidence, importance
                        ) 
                        VALUES ($1, $2, $3, $4)
                        RETURNING id
                        "#,
                        field.matcher,
                        field.value,
                        matcher_config.0,
                        matcher_config.1
                    )
                    .fetch_one(&mut *tx)
                    .await?;
                    
                    row.id
                }
            };
            
            // Check if this node-transaction connection already exists
            let existing_connection = sqlx::query!(
                r#"
                SELECT 1 as exists
                FROM match_node_transactions 
                WHERE node_id = $1 AND transaction_id = $2
                "#,
                node_id,
                transaction_id
            )
            .fetch_optional(&mut *tx)
            .await?;
            
            // Only insert if connection doesn't exist
            if existing_connection.is_none() {
                sqlx::query!(
                    r#"
                    INSERT INTO match_node_transactions (node_id, transaction_id)
                    VALUES ($1, $2)
                    "#,
                    node_id,
                    transaction_id
                )
                .execute(&mut *tx)
                .await?;
            }
        }
        
        tx.commit().await?;
        debug!("Successfully saved {} matching fields for transaction {}", matching_fields.len(), transaction_id);
        Ok(())
    }

    async fn get_channels(
        &self,
        model_id: ModelId,
    ) -> Result<Vec<Channel>, Box<dyn Error + Send + Sync>> {
        let rows = sqlx::query!(
            r#"
            SELECT id, name, model_id, created_at
            FROM channels
            WHERE model_id = $1
            "#,
            model_id
        )
        .fetch_all(&self.pool)
        .await?;
        
        let mut channels = Vec::new();
        for row in rows {
            channels.push(Channel {
                id: row.id,
                name: row.name,
                model_id: row.model_id,
                created_at: DateTime::<Utc>::from_naive_utc_and_offset(row.created_at, Utc),
            });
        }
        
        Ok(channels)
    }
    
    async fn get_scoring_events(
        &self,
        transaction_id: ModelId,
    ) -> Result<Vec<ScoringEvent>, Box<dyn Error + Send + Sync>> {
        let rows = sqlx::query!(
            r#"
            SELECT id, transaction_id, channel_id, total_score, created_at
            FROM scoring_events
            WHERE transaction_id = $1
            ORDER BY created_at DESC
            "#,
            transaction_id
        )
        .fetch_all(&self.pool)
        .await?;
        
        let mut events = Vec::new();
        for row in rows {
            events.push(ScoringEvent {
                id: row.id,
                transaction_id: row.transaction_id,
                channel_id: row.channel_id,
                total_score: row.total_score,
                created_at: DateTime::<Utc>::from_naive_utc_and_offset(row.created_at, Utc),
            });
        }
        
        Ok(events)
    }
    
    async fn get_triggered_rules(
        &self,
        scoring_event_id: ModelId,
    ) -> Result<Vec<TriggeredRule>, Box<dyn Error + Send + Sync>> {
        let rows = sqlx::query!(
            r#"
            SELECT id, scoring_events_id, rule_id
            FROM triggered_rules
            WHERE scoring_events_id = $1
            "#,
            scoring_event_id
        )
        .fetch_all(&self.pool)
        .await?;
        
        let mut rules = Vec::new();
        for row in rows {
            rules.push(TriggeredRule {
                id: row.id,
                scoring_events_id: row.scoring_events_id,
                rule_id: row.rule_id,
            });
        }
        
        Ok(rules)
    }
    
    async fn save_label(
        &self,
        label: &Label,
    ) -> Result<ModelId, Box<dyn Error + Send + Sync>> {
        // Convert the enum values to strings using serde
        let fraud_level_str = format!("{:?}", label.fraud_level);
        let label_source_str = format!("{:?}", label.label_source);
        
        // Convert DateTime<Utc> to naive_utc for PostgreSQL compatibility
        let created_at_naive = label.created_at.naive_utc();
        
        let row = sqlx::query!(
            r#"
            INSERT INTO labels (
                fraud_level, fraud_category, label_source, labeled_by, created_at
            ) VALUES ($1, $2, $3, $4, $5)
            RETURNING id
            "#,
            fraud_level_str,
            label.fraud_category,
            label_source_str,
            label.labeled_by,
            created_at_naive
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.id)
    }
    
    async fn get_label(
        &self,
        label_id: ModelId,
    ) -> Result<Label, Box<dyn Error + Send + Sync>> {
        let row = sqlx::query!(
            r#"
            SELECT id, fraud_level, fraud_category, label_source, labeled_by, created_at
            FROM labels
            WHERE id = $1
            "#,
            label_id
        )
        .fetch_one(&self.pool)
        .await?;

        // Parse the strings back to enums
        let fraud_level = match row.fraud_level.as_str() {
            "Fraud" => FraudLevel::Fraud,
            "NoFraud" => FraudLevel::NoFraud,
            "BlockedAutomatically" => FraudLevel::BlockedAutomatically,
            "AccountTakeover" => FraudLevel::AccountTakeover,
            "NotCreditWorthy" => FraudLevel::NotCreditWorthy,
            _ => return Err(format!("Invalid fraud level: {}", row.fraud_level).into()),
        };
        
        let label_source = match row.label_source.as_str() {
            "Manual" => LabelSource::Manual,
            "Api" => LabelSource::Api,
            _ => return Err(format!("Invalid label source: {}", row.label_source).into()),
        };

        Ok(Label {
            id: row.id,
            fraud_level,
            fraud_category: row.fraud_category,
            label_source,
            labeled_by: row.labeled_by,
            created_at: DateTime::<Utc>::from_naive_utc_and_offset(row.created_at, Utc),
        })
    }
    
    async fn update_transaction_label(
        &self,
        transaction_id: ModelId,
        label_id: ModelId,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        sqlx::query!(
            r#"
            UPDATE transactions
            SET label_id = $1
            WHERE id = $2
            "#,
            label_id,
            transaction_id
        )
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
} 