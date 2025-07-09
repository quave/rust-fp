/// Ecom-specific test helpers
/// 
/// This module provides test utilities specific to ecom crate that ensure
/// proper test isolation by using unique transaction IDs.

use std::error::Error;
use ecom::ecom_import_model::ImportOrder;
use ecom::ecom_order_storage::EcomOrderStorage;
use ecom::models::Order;
use processing::storage::{ImportableStorage, ProcessibleStorage};
use common::test_helpers::{get_test_database_url, cleanup_ecom_transaction};
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
    
    /// Save a transaction using the standard ImportableStorage interface
    /// This uses the public interface to save transactions
    pub async fn save_test_transaction(&self, order: &ImportOrder) -> Result<i64, Box<dyn Error + Send + Sync>> {
        // Use the standard ImportableStorage trait method
        self.storage.save_transaction(order).await
    }
    
    /// Get transaction using the underlying storage
    pub async fn get_transaction(&self, id: i64) -> Result<Order, Box<dyn Error + Send + Sync>> {
        self.storage.get_processible(id).await
    }
    
    /// Clean up test transaction
    pub async fn cleanup_test_transaction(&self, transaction_id: i64) -> Result<(), Box<dyn Error + Send + Sync>> {
        cleanup_ecom_transaction(&self.pool, transaction_id).await
    }
} 