use std::{collections::VecDeque, sync::Mutex};
use sqlx::PgPool;


use crate::{
    model::ModelId,
    queue::QueueService,
};

// Mock queue implementation for tests
#[derive(Default)]
pub struct MockQueue {
    queue: Mutex<VecDeque<ModelId>>,
}

impl MockQueue {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
        }
    }
}

#[async_trait::async_trait]
impl QueueService for MockQueue {
    async fn enqueue(&self, id: ModelId) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.queue.lock().unwrap().push_back(id.clone());
        Ok(())
    }

    async fn fetch_next(&self) -> Result<Option<ModelId>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self.queue.lock().unwrap().pop_front())
    }

    async fn mark_processed(&self, _id: ModelId) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }
}

/// Initialize database schema for testing using SQLx migrations
pub async fn initialize_test_schema(pool: &PgPool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    sqlx::migrate!("../migrations")
        .run(pool)
        .await?;

    Ok(())
}

