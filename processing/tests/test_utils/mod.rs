use std::{collections::VecDeque, sync::Mutex, path::PathBuf};
use sqlx::PgPool;

use processing::{
    model::ModelId,
    queue::QueueService,
    storage::ProdCommonStorage,
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

/// Get the test database URL
pub fn get_test_database_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/frida_test".to_string())
}

/// Create a database connection for testing
pub async fn create_test_pool() -> Result<PgPool, Box<dyn std::error::Error + Send + Sync>> {
    let database_url = get_test_database_url();
    let pool = PgPool::connect(&database_url).await?;
    Ok(pool)
}

/// Create a common storage instance for testing
pub async fn create_test_common_storage() -> Result<ProdCommonStorage, Box<dyn std::error::Error + Send + Sync>> {
    let database_url = get_test_database_url();
    let storage = ProdCommonStorage::new(&database_url).await?;
    Ok(storage)
}

/// Drop all tables in the test database
pub async fn drop_all_tables(pool: &PgPool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // First truncate the migrations table
    sqlx::query("TRUNCATE TABLE _sqlx_migrations CASCADE")
        .execute(pool)
        .await?;
    
    // Drop tables in reverse order of dependencies
    let drop_statements = vec![
        "DROP TABLE IF EXISTS order_items CASCADE",
        "DROP TABLE IF EXISTS customers CASCADE",
        "DROP TABLE IF EXISTS billing_data CASCADE",
        "DROP TABLE IF EXISTS orders CASCADE",
        "DROP TABLE IF EXISTS features CASCADE",
        "DROP TABLE IF EXISTS triggered_rules CASCADE",
        "DROP TABLE IF EXISTS processing_queue CASCADE",
        "DROP TABLE IF EXISTS transactions CASCADE",
    ];

    for statement in drop_statements {
        sqlx::query(statement)
            .execute(pool)
            .await?;
    }

    Ok(())
}

/// Initialize database schema for testing using SQLx migrations
pub async fn initialize_test_schema(pool: &PgPool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // First drop all existing tables
    drop_all_tables(pool).await?;
    
    // Get the absolute path to the migrations directory
    let workspace_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf();
    let migrations_dir = workspace_dir.join("migrations");

    // Run migrations to create fresh tables
    let migrator = sqlx::migrate::Migrator::new(migrations_dir).await?;
    migrator.run(pool).await?;

    Ok(())
}

/// Global test setup function that should be called before running any tests
pub async fn setup_test_environment() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let pool = create_test_pool().await?;
    initialize_test_schema(&pool).await?;
    Ok(())
}

