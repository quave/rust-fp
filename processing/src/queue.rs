use async_trait::async_trait;
use common::config::CommonConfig;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement};
use serde::{Serialize, de::DeserializeOwned};
use std::{error::Error, str::FromStr};
use pgmq::{Message, PGMQueue};
use strum_macros::Display;

// Queue service interface
#[async_trait]
pub trait QueueService<ID: Send + Sync + Serialize + DeserializeOwned>: Send + Sync + 'static {
    async fn fetch_next(&self, number: i32) -> Result<Vec<(ID, i64)>, Box<dyn Error + Send + Sync>>;
    async fn mark_processed(&self, id: i64) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn enqueue(&self, ids: &[ID]) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn is_enqueued(&self, ids: &[ID]) -> Result<Vec<ID>, Box<dyn Error + Send + Sync>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display)]
pub enum QueueName {
    #[strum(to_string = "processing_queue")]
    Processing,
    #[strum(to_string = "recalculation_queue")]
    Recalculation,
}

pub struct ProdQueue {
    queue: PGMQueue,
    queue_name: QueueName,
    db: DatabaseConnection,
}

impl ProdQueue {
    pub async fn new(config: &CommonConfig, queue_name: QueueName) -> Result<Self, Box<dyn Error + Send + Sync>> {
        println!("Trying to connect to db for queue: {:?}", queue_name);
        let db = Database::connect(config.database_url.clone())
            .await
            .expect("Failed to connect to database");

        let index_sql = format!("create index if not exists idx_q_{}_message on pgmq.q_{}(message)",
            queue_name.to_string().to_lowercase(), queue_name.to_string().to_lowercase());
        db.execute(Statement::from_string(DbBackend::Postgres, index_sql)).await?;

        let queue: PGMQueue = PGMQueue::new(config.database_url.clone())
            .await
            .expect("Failed to connect to postgres");
        println!("Connected to db for prod_queue");

        println!("Creating a queue '{:?}'", queue_name);
        queue.create(&queue_name.to_string())
            .await
            .expect("Failed to create queue");
        Ok(Self { queue, queue_name, db})
    }
}

#[async_trait]
impl<ID: Send + Sync + Serialize + DeserializeOwned + ToString + FromStr> QueueService<ID> for ProdQueue {
    async fn enqueue(&self, ids: &[ID]) -> Result<(), Box<dyn Error + Send + Sync>> {
        if ids.is_empty() {
            return Ok(());
        }

        self.queue.send_batch::<ID>(&self.queue_name.to_string(), &ids)
            .await
            .expect("Failed to send message to queue");
        Ok(())
    }

    async fn fetch_next(&self, _number: i32) -> Result<Vec<(ID, i64)>, Box<dyn Error + Send + Sync>> {
        let visibility_timeout_seconds: i32 = 30;

        let received_message: Option<Message<ID>> = self
            .queue
            .read::<ID>(&self.queue_name.to_string(), Some(visibility_timeout_seconds))
            .await?;

        Ok(received_message.map(|msg| vec![(msg.message, msg.msg_id)]).unwrap_or_default())
    }

    async fn mark_processed(&self, msg_id: i64) -> Result<(), Box<dyn Error + Send + Sync>> {
        let _ = self.queue.archive(&self.queue_name.to_string(), msg_id)
            .await
            .expect("Failed to archive message");
        Ok(())
    }

    async fn is_enqueued(&self, ids: &[ID]) -> Result<Vec<ID>, Box<dyn Error + Send + Sync>> {
        if ids.is_empty() {
            return Ok(vec![]);
        }

        let ids_str = ids.iter().map(|id| format!("'{}'::jsonb", id.to_string())).collect::<Vec<String>>().join(",");
        let sql = format!("select message from pgmq.q_recalculation_queue where message in ({});", ids_str);
        let result: Vec<ID> = self
            .db
            .query_all(Statement::from_string(DbBackend::Postgres, sql))
            .await?
            .iter()
            .map(|row| 
                row
                    .try_get::<String>("", "message")
                    .expect("Failed to get message")
                    .parse::<ID>()
                    .map_err(|_| "Failed to parse ID")
                    .expect("Failed to parse ID"))
            .collect::<Vec<ID>>();

        Ok(result)
    }
}
