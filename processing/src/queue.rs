use async_trait::async_trait;
use sqlx::PgPool;
use std::error::Error;
use std::sync::Arc;
use tracing::info;

use crate::model::ModelId;

// Queue service interface
#[async_trait]
pub trait QueueService: Send + Sync {
    async fn fetch_next(&self) -> Result<Option<ModelId>, Box<dyn Error + Send + Sync>>;
    async fn mark_processed(&self, id: ModelId) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn enqueue(&self, id: ModelId) -> Result<(), Box<dyn Error + Send + Sync>>;
}

pub struct ProdQueue {
    pool: PgPool,
}

impl ProdQueue {
    pub async fn new(database_url: &str) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let pool = PgPool::connect(database_url).await?;
        Ok(Self {
            pool,
        })
    }
}

#[async_trait]
impl QueueService for ProdQueue {
    async fn enqueue(&self, id: ModelId) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!("Enqueuing transaction to the db: {:?}", id);
        sqlx::query("INSERT INTO processing_queue (processable_id) VALUES ($1)")
            .bind(&id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn fetch_next(&self) -> Result<Option<ModelId>, Box<dyn Error + Send + Sync>> {
        let mut tx = self.pool.begin().await?;

        let record = sqlx::query!(
            r#"
            SELECT id, processable_id
            FROM processing_queue
            WHERE processed_at IS NULL
            ORDER BY created_at ASC
            FOR UPDATE SKIP LOCKED
            LIMIT 1
            "#
        )
        .fetch_optional(&mut *tx)
        .await?;

        if let Some(record) = record {
            tx.commit().await?;
            Ok(Some(record.processable_id))
        } else {
            Ok(None)
        }
    }

    async fn mark_processed(&self, id: ModelId) -> Result<(), Box<dyn Error + Send + Sync>> {
        sqlx::query!(
            r#"
            UPDATE processing_queue
            SET processed_at = NOW()
            WHERE processable_id = $1
            "#,
            id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

pub struct InMemoryQueue {
    queue: Arc<tokio::sync::Mutex<Vec<ModelId>>>,
}

impl InMemoryQueue {
    pub fn new() -> Self {
        Self {
            queue: Arc::new(tokio::sync::Mutex::new(Vec::new())),
        }
    }
}

#[async_trait::async_trait]
impl QueueService for InMemoryQueue {
    async fn enqueue(&self, id: ModelId) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut queue = self.queue.lock().await;
        queue.push(id);
        info!("Enqueued transaction {:?}", id);
        Ok(())
    }

    async fn fetch_next(&self) -> Result<Option<ModelId>, Box<dyn Error + Send + Sync>> {
        let mut queue = self.queue.lock().await;
        if queue.is_empty() {
            Ok(None)
        } else {
            let id = queue.remove(0);
            Ok(Some(id))
        }
    }

    async fn mark_processed(&self, id: ModelId) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut queue = self.queue.lock().await;
        if let Some(pos) = queue.iter().position(|&x| x == id) {
            queue.remove(pos);
            info!("Dequeued transaction {:?}", id);
        }
        Ok(())
    }
}
