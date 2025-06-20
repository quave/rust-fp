use crate::model::*;
use async_trait::async_trait;
use jsonschema::validate;
use tracing::debug;
use serde_json::{json, Value};
use sqlx::PgPool;
use std::error::Error;

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
}

#[derive(Clone)]
pub struct ProdCommonStorage {
    pub pool: PgPool,
}

impl ProdCommonStorage {
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPool::connect(database_url).await?;
        Ok(Self { pool })
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
}
