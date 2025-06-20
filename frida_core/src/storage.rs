use crate::model::*;
use async_trait::async_trait;
use log::{debug, error};
use sqlx::{PgPool, types::Json};
use std::error::Error;
use serde_json::Value;

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
    pool: PgPool,
}

impl ProdCommonStorage {
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPool::connect(database_url).await?;
        Ok(Self { pool })
    }

    pub async fn initialize_schema(&self) -> Result<(), sqlx::Error> {
        sqlx::query(include_str!("../resources/core_schema.sql"))
            .execute(&self.pool)
            .await?;
        Ok(())
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
            match sqlx::query!(
                r#"
                INSERT INTO triggered_rules (transaction_id, rule_name, rule_score)
                VALUES ($1, $2, $3)
                "#,
                transaction_id,
                score.name,
                score.score
            )
            .execute(&mut *tx)
            .await
            {
                Ok(_) => {
                    debug!("Successfully inserted customer data");
                }
                Err(e) => {
                    error!("Failed to insert customer data: {}", e);
                }
            }
        }

        tx.commit().await?;
        Ok(())
    }

    async fn save_features(
        &self,
        transaction_id: ModelId,
        features: &[Feature],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
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
        Ok(features)
    }
}
