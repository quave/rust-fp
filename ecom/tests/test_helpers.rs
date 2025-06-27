/// Ecom-specific test helpers
/// 
/// This module provides test utilities specific to ecom crate that ensure
/// proper test isolation by using unique transaction IDs.

use std::error::Error;
use ecom::ecom_import_model::ImportOrder;
use ecom::ecom_order_storage::EcomOrderStorage;
use ecom::ecom_db_model::Order;
use processing::storage::WebStorage;
use common::test_helpers::{generate_unique_test_id, get_test_database_url, cleanup_ecom_transaction};
use sqlx::PgPool;

/// Test-specific wrapper for EcomOrderStorage that ensures unique transaction IDs
pub struct TestEcomOrderStorage {
    pub storage: EcomOrderStorage,
    pub pool: PgPool,
}

impl TestEcomOrderStorage {
    pub async fn new() -> Result<Self, Box<dyn Error + Send + Sync>> {
        let database_url = get_test_database_url();
        let storage = EcomOrderStorage::new(&database_url).await?;
        let pool = common::test_helpers::create_test_pool().await?;
        
        Ok(Self {
            storage,
            pool,
        })
    }
    
    /// Save a transaction using unique test ID generation
    /// This bypasses the normal save_transaction to ensure unique IDs
    pub async fn save_test_transaction(&self, order: &ImportOrder) -> Result<i64, Box<dyn Error + Send + Sync>> {
        // Create transaction with unique ID first
        let unique_id = generate_unique_test_id() as i64;
        
        // Start a transaction manually and insert with unique ID
        let mut tx = self.storage.pool.begin().await?;
        
        // Insert transaction with unique ID
        sqlx::query!(
            r#"
            INSERT INTO transactions (id, created_at)
            VALUES ($1, NOW())
            "#,
            unique_id
        )
        .execute(&mut *tx)
        .await?;
        
        // Insert the order data using the existing helper methods
        let order_id = self.storage.insert_order(unique_id, order, &mut tx).await?;
        let _ = self.storage.insert_customer(order_id, &order.customer, &mut tx).await?;
        let _ = self.storage.insert_order_items(order_id, &order.items, &mut tx).await?;
        let _ = self.storage.insert_billing(order_id, &order.billing, &mut tx).await?;
        
        tx.commit().await?;
        
        Ok(unique_id)
    }
    
    /// Get transaction using the underlying storage
    pub async fn get_transaction(&self, id: i64) -> Result<Order, Box<dyn Error + Send + Sync>> {
        self.storage.get_transaction(id).await
    }
    
    /// Clean up test transaction
    pub async fn cleanup_test_transaction(&self, transaction_id: i64) -> Result<(), Box<dyn Error + Send + Sync>> {
        cleanup_ecom_transaction(&self.pool, transaction_id).await
    }
} 