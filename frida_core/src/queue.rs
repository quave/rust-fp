use async_trait::async_trait;
use log::info;
use sqlx::{Pool, Sqlite};
use std::error::Error;
use std::marker::PhantomData;

use crate::model::{ModelId, Processible};

// Queue service interface
#[async_trait]
pub trait QueueService<P: Processible>: Send + Sync {
    async fn enqueue(&self, id: ModelId) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn dequeue(&self) -> Result<Option<ModelId>, Box<dyn Error + Send + Sync>>;
}

pub struct ProdQueue<P: Processible> {
    pool: Pool<Sqlite>,
    _phantom: PhantomData<P>,
}

impl<P: Processible> ProdQueue<P> {
    pub async fn new(database_url: &str) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let pool = sqlx::SqlitePool::connect(database_url).await?;
        Ok(Self {
            pool,
            _phantom: PhantomData,
        })
    }
}

#[async_trait]
impl<P: Processible> QueueService<P> for ProdQueue<P> {
    async fn enqueue(&self, id: ModelId) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!("Enqueuing transaction to SQLite: {:?}", id);
        sqlx::query("INSERT INTO processing_queue (processable_id) VALUES (?)")
            .bind(serde_json::to_string(&id)?)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn dequeue(&self) -> Result<Option<ModelId>, Box<dyn Error + Send + Sync>> {
        let result = sqlx::query!(
            r#"
            UPDATE processing_queue 
            SET processed_at = CURRENT_TIMESTAMP
            WHERE id = (
                SELECT id FROM processing_queue 
                WHERE processed_at IS NULL 
                ORDER BY created_at ASC 
                LIMIT 1
            )
            RETURNING processable_id as "processable_id: i64"
            "#
        )
        .fetch_optional(&self.pool)
        .await?;

        match result {
            Some(record) => {
                info!(
                    "Dequeued transaction from SQLite: {}",
                    record.processable_id
                );
                let id: ModelId =
                    serde_json::from_str::<ModelId>(&record.processable_id.to_string())?;
                Ok(Some(id))
            }
            None => Ok(None),
        }
    }
}
