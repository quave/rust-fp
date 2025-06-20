use crate::model::*;
use async_trait::async_trait;
use log::{debug, error};
use sqlx::SqlitePool;
use std::error::Error;

// Define the Storage trait
#[async_trait]
pub trait ProcessibleStorage<P: Processible>: Send + Sync {
    async fn get_transaction(
        &self,
        transaction_id: ModelId,
    ) -> Result<P, Box<dyn Error + Send + Sync>>;

    async fn save_features(
        &self,
        transaction_id: ModelId,
        features: &[Feature],
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
}

#[async_trait]
pub trait ImportableStorage<I: Importable>: Send + Sync {
    async fn initialize_schema(&self) -> Result<(), Box<dyn Error + Send + Sync>>;

    async fn save_transaction(
        &self,
        transaction: &I,
    ) -> Result<ModelId, Box<dyn Error + Send + Sync>>;
}

#[async_trait]
pub trait CommonStorage: Send + Sync {
    async fn save_scores(
        &self,
        transaction_id: i64,
        scores: &[ScorerResult],
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
}

#[derive(Clone)]
pub struct ProdCommonStorage {
    pool: SqlitePool,
}

impl ProdCommonStorage {
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = sqlx::SqlitePool::connect(database_url).await?;
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
                INSERT INTO triggered_rules (order_id, rule_name, rule_score) 
                VALUES (?, ?, ?)
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
}
