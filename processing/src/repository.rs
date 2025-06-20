use crate::{
    common::{
        config::Config,
        error::{Error, Result},
    },
    model::Order,
};
use sqlx::PgPool;

#[derive(Clone)]
pub struct OrderRepository {
    pool: PgPool,
}

impl OrderRepository {
    pub fn new(config: &Config) -> Result<Self> {
        let pool = PgPool::connect(&config.database_url)
            .map_err(|e| Error::Database(format!("Failed to create pool: {}", e)))?;
        Ok(Self { pool })
    }

    pub async fn get_all_orders(&self) -> Result<Vec<Order>> {
        let orders = sqlx::query_as!(
            Order,
            r#"
            SELECT id, customer_name, total_amount, status, created_at
            FROM orders
            ORDER BY created_at DESC
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(format!("Failed to fetch orders: {}", e)))?;

        Ok(orders)
    }
} 