use async_trait::async_trait;
use chrono::Utc;
use sea_orm::ActiveValue::{NotSet, Set};
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, Database, DatabaseConnection, DbBackend,
    EntityTrait, QueryFilter, Statement, Value,
};
use std::error::Error;
use tracing::info;

use crate::model::ModelId;
use crate::model::sea_orm_queue_entities::{processing_queue, recalculation_queue};

// Queue service interface
#[async_trait]
pub trait QueueService: Send + Sync {
    async fn fetch_next(&self, number: i32) -> Result<Vec<ModelId>, Box<dyn Error + Send + Sync>>;
    async fn mark_processed(&self, id: ModelId) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn enqueue(&self, id: ModelId) -> Result<(), Box<dyn Error + Send + Sync>>;
}

pub struct ProdQueue {
    db: DatabaseConnection,
}

impl ProdQueue {
    pub async fn new(database_url: &str) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let db = Database::connect(database_url).await?;
        Ok(Self { db })
    }
}

#[async_trait]
impl QueueService for ProdQueue {
    async fn enqueue(&self, id: ModelId) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!("Queue[processing_queue]: Enqueuing transaction: {:?}", id);
        processing_queue::ActiveModel {
            id: NotSet,
            transaction_id: Set(id),
            processed_at: Set(None),
            created_at: Set(Utc::now().naive_utc()),
        }
        .insert(&self.db)
        .await?;
        Ok(())
    }

    async fn fetch_next(&self, number: i32) -> Result<Vec<ModelId>, Box<dyn Error + Send + Sync>> {
        let rows = self
            .db
            .query_all(Statement::from_sql_and_values(
                DbBackend::Postgres,
                r#"
                SELECT transaction_id
                FROM processing_queue
                WHERE processed_at IS NULL
                ORDER BY created_at ASC
                FOR UPDATE SKIP LOCKED
                LIMIT $1
                "#,
                vec![Value::from(number as i64)],
            ))
            .await?;

        let mut ids = Vec::with_capacity(rows.len());
        for row in rows {
            ids.push(row.try_get::<ModelId>("", "transaction_id")?);
        }
        Ok(ids)
    }

    async fn mark_processed(&self, id: ModelId) -> Result<(), Box<dyn Error + Send + Sync>> {
        processing_queue::Entity::update_many()
            .col_expr(
                processing_queue::Column::ProcessedAt,
                Expr::value(Utc::now().naive_utc()),
            )
            .filter(processing_queue::Column::TransactionId.eq(id))
            .exec(&self.db)
            .await?;
        Ok(())
    }
}

pub struct RecalcQueue {
    db: DatabaseConnection,
}

impl RecalcQueue {
    pub async fn new(database_url: &str) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let db = Database::connect(database_url).await?;
        Ok(Self { db })
    }
}

#[async_trait]
impl QueueService for RecalcQueue {
    async fn enqueue(&self, id: ModelId) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!(
            "Queue[recalculation_queue]: Enqueuing transaction: {:?}",
            id
        );
        recalculation_queue::ActiveModel {
            id: NotSet,
            transaction_id: Set(id),
            processed_at: Set(None),
            created_at: Set(Utc::now().naive_utc()),
        }
        .insert(&self.db)
        .await?;

        Ok(())
    }

    async fn fetch_next(&self, number: i32) -> Result<Vec<ModelId>, Box<dyn Error + Send + Sync>> {
        let rows = self
            .db
            .query_all(Statement::from_sql_and_values(
                DbBackend::Postgres,
                r#"
                SELECT transaction_id
                FROM recalculation_queue
                WHERE processed_at IS NULL
                ORDER BY created_at ASC
                FOR UPDATE SKIP LOCKED
                LIMIT $1
                "#,
                vec![Value::from(number as i64)],
            ))
            .await?;

        let mut ids = Vec::with_capacity(rows.len());
        for row in rows {
            ids.push(row.try_get::<ModelId>("", "transaction_id")?);
        }
        Ok(ids)
    }

    async fn mark_processed(&self, id: ModelId) -> Result<(), Box<dyn Error + Send + Sync>> {
        recalculation_queue::Entity::update_many()
            .col_expr(
                recalculation_queue::Column::ProcessedAt,
                Expr::value(Utc::now().naive_utc()),
            )
            .filter(recalculation_queue::Column::TransactionId.eq(id))
            .exec(&self.db)
            .await?;
        Ok(())
    }
}
