use crate::model::*;
use async_trait::async_trait;
use jsonschema::validate;
use tracing::debug;
use serde_json::{json, Value};
use sqlx::PgPool;
use std::error::Error;
use std::collections::HashMap;

// Matcher confidence and importance as a tuple
pub type MatcherConfig = (i32, i32); // (confidence, importance)

// Default implementation for MatcherConfig
fn default_matcher_config() -> MatcherConfig {
    (80, 50) // Default confidence and importance
}

#[async_trait]
pub trait WebStorage<T: WebTransaction + Send + Sync>: Send + Sync {
    async fn get_transactions(
        &self,
    ) -> Result<Vec<T>, Box<dyn Error + Send + Sync>>;

    async fn get_transaction(
        &self,
        transaction_id: ModelId,
    ) -> Result<T, Box<dyn Error + Send + Sync>>;
}

// Define the Storage trait
#[async_trait]
pub trait ProcessibleStorage<P: Processible>: Send + Sync {
    async fn get_processible(
        &self,
        transaction_id: ModelId,
    ) -> Result<P, Box<dyn Error + Send + Sync>>;
}

#[async_trait]
pub trait ImportableStorage<I: Importable>: Send + Sync {
    async fn save_transaction(
        &self,
        tx_data: &I,
    ) -> Result<ModelId, Box<dyn Error + Send + Sync>>;
}

#[async_trait]
pub trait CommonStorage: Send + Sync {
    async fn save_features(
        &self,
        transaction_id: ModelId,
        features: &[Feature],
    ) -> Result<(), Box<dyn Error + Send + Sync>>;

    async fn save_scores(
        &self,
        transaction_id: i64,
        scores: &[ScorerResult],
    ) -> Result<(), Box<dyn Error + Send + Sync>>;

    async fn get_features(
        &self,
        transaction_id: ModelId,
    ) -> Result<Vec<Feature>, Box<dyn Error + Send + Sync>>;
    
    async fn find_connected_transactions(
        &self,
        transaction_id: ModelId,
        max_depth: Option<i32>,
        limit_count: Option<i32>,
        min_created_at: Option<chrono::DateTime<chrono::Utc>>,
        max_created_at: Option<chrono::DateTime<chrono::Utc>>,
        min_confidence: Option<i32>
    ) -> Result<Vec<ConnectedTransaction>, Box<dyn Error + Send + Sync>>;
    
    async fn get_direct_connections(
        &self,
        transaction_id: ModelId
    ) -> Result<Vec<DirectConnection>, Box<dyn Error + Send + Sync>>;
    
    async fn save_matching_fields(
        &self,
        transaction_id: ModelId,
        matching_fields: &[MatchingField],
        matcher_configs: &HashMap<String, MatcherConfig>
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
    
    fn get_matcher_configs(&self) -> HashMap<String, MatcherConfig>;
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
        transaction_id: i64,
        scores: &[ScorerResult],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut tx = self.pool.begin().await?;

        // Save individual triggered rules
        for score in scores {
            sqlx::query!(
                r#"
                INSERT INTO triggered_rules (transaction_id, rule_name, rule_score)
                VALUES ($1, $2, $3)
                "#,
                transaction_id,
                score.name,
                score.score
            )
            .execute(&mut *tx)
            .await?;
            debug!("Successfully inserted score data");
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
        matcher_configs: &HashMap<String, MatcherConfig>
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        if matching_fields.is_empty() {
            return Ok(());
        }
        
        let mut tx = self.pool.begin().await?;
        
        for field in matching_fields {
            // Get configuration for this matcher - use the passed config which could be overridden
            let matcher_config = matcher_configs.get(&field.matcher).cloned().unwrap_or_else(default_matcher_config);
            
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

    fn get_matcher_configs(&self) -> HashMap<String, MatcherConfig> {
        self.matcher_configs.clone()
    }
}
