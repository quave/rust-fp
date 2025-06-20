use crate::model::{ModelId, ModelRegistryProvider, WebTransaction};
use crate::ui_model::{build_query, FilterRequest};
use async_trait::async_trait;
use sqlx::{PgConnection, Row};
use tracing::debug;
use std::error::Error;

#[async_trait]
pub trait WebStorage<T: WebTransaction + ModelRegistryProvider + Send + Sync>: Send + Sync {
    /// Get all transactions, optionally filtered by the provided filter request
    async fn get_transactions(
        &self,
        filter: FilterRequest,
    ) -> Result<Vec<T>, Box<dyn Error + Send + Sync>>;

    /// Get a specific transaction by ID
    async fn get_transaction(
        &self,
        transaction_id: ModelId,
    ) -> Result<T, Box<dyn Error + Send + Sync>>;

    async fn filter_orders(&self, filters: FilterRequest, tx: &mut PgConnection) ->
        Result<Vec<ModelId>, Box<dyn Error + Send + Sync>> {
        let (query, args) = build_query::<T>(&filters)?;

        debug!("Executing filter query: {}", query);

        sqlx::query_with(&query, args)
            .fetch_all(&mut *tx)
            .await?
            .iter()
            .map(|row| Ok(row.try_get(0)?))
            .collect::<Result<Vec<_>, Box<dyn Error + Send + Sync>>>()
    }
} 