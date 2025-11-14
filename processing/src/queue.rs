use async_trait::async_trait;
use sqlx::PgPool;
use std::error::Error;
use tracing::info;

use crate::model::ModelId;

// Queue service interface
#[async_trait]
pub trait QueueService: Send + Sync {
    async fn fetch_next(&self, number: i32) -> Result<Vec<ModelId>, Box<dyn Error + Send + Sync>>;
    async fn mark_processed(&self, id: ModelId) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn enqueue(&self, id: ModelId) -> Result<(), Box<dyn Error + Send + Sync>>;
}

pub struct ProdQueue {
    pool: PgPool,
}

impl ProdQueue {
    pub async fn new(database_url: &str) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let pool = PgPool::connect(database_url).await?;
        Ok(Self { pool })
    }
}

#[async_trait]
impl QueueService for ProdQueue {
    async fn enqueue(&self, id: ModelId) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!("Enqueuing transaction to the db: {:?}", id);
        sqlx::query("INSERT INTO processing_queue (transaction_id) VALUES ($1)")
            .bind(&id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn fetch_next(&self, number: i32) -> Result<Vec<ModelId>, Box<dyn Error + Send + Sync>> {
        let mut tx = self.pool.begin().await?;

        sqlx::query_scalar!(
            r#"
            SELECT transaction_id
            FROM processing_queue
            WHERE processed_at IS NULL
            ORDER BY created_at ASC
            FOR UPDATE SKIP LOCKED
            LIMIT $1
            "#
        , number as i64)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| e.into())
    }

    async fn mark_processed(&self, id: ModelId) -> Result<(), Box<dyn Error + Send + Sync>> {
        sqlx::query!(
            r#"
            UPDATE processing_queue
            SET processed_at = NOW()
            WHERE transaction_id = $1
            "#,
            id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}


pub struct RecalcQueue {
    pool: PgPool,
}

impl RecalcQueue {
    pub async fn new(database_url: &str) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let pool = PgPool::connect(database_url).await?;
        Ok(Self {
            pool,
        })
    }
}

#[async_trait]
impl QueueService for RecalcQueue {
    async fn enqueue(&self, id: ModelId) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!("Enqueuing transaction to the db: {:?}", id);
        sqlx::query("INSERT INTO recalculation_queue (transaction_id) VALUES ($1)")
            .bind(&id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn fetch_next(&self, number: i32) -> Result<Vec<ModelId>, Box<dyn Error + Send + Sync>> {
        let mut tx = self.pool.begin().await?;

        sqlx::query_scalar!(
            r#"
            SELECT transaction_id
            FROM recalculation_queue
            WHERE processed_at IS NULL
            ORDER BY created_at ASC
            FOR UPDATE SKIP LOCKED
            LIMIT $1
            "#
        , number as i64)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| e.into())
    }

    async fn mark_processed(&self, id: ModelId) -> Result<(), Box<dyn Error + Send + Sync>> {
        sqlx::query!(
            r#"
            UPDATE recalculation_queue
            SET processed_at = NOW()
            WHERE transaction_id = $1
            "#,
            id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

