use std::sync::Arc;
use std::error::Error;
use axum::Router;
use processing::{
    executable_utils::{label_transaction, AppState},
    model::{FraudLevel, LabelingResult, ModelId, WebTransaction},
    storage::{CommonStorage, WebStorage}, 
};
use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use seaography::Builder;

// Import and re-export centralized mocks for use in API tests
// Simple local mock implementations - TODO: Move to common when centralized
#[derive(Clone, serde::Serialize)]
pub struct MockWebTransaction {
    pub id: ModelId,
}

impl MockWebTransaction {
    pub fn new(id: ModelId) -> Self {
        Self { id }
    }
    
    pub fn get_id(&self) -> ModelId {
        self.id
    }
}

pub struct MockWebStorage {
    pub transactions: Vec<MockWebTransaction>,
}

impl MockWebStorage {
    pub fn new(transactions: Vec<MockWebTransaction>) -> Self {
        Self { transactions }
    }
    
    pub async fn get_transactions(&self) -> Result<Vec<MockWebTransaction>, Box<dyn Error + Send + Sync>> {
        Ok(self.transactions.clone())
    }
    
    pub async fn get_transaction(&self, transaction_id: ModelId) -> Result<MockWebTransaction, Box<dyn Error + Send + Sync>> {
        self.transactions.iter()
            .find(|t| t.id == transaction_id)
            .cloned()
            .ok_or_else(|| format!("Transaction {} not found", transaction_id).into())
    }
}

// Implement the required traits for MockWebTransaction
#[async_trait]
impl WebTransaction for MockWebTransaction {
    fn id(&self) -> ModelId {
        self.get_id()
    }
}

// Implement WebStorage trait for MockWebStorage
#[async_trait]
impl WebStorage<MockWebTransaction> for MockWebStorage {
    fn register_seaography_entities(&self, _builder: Builder) -> Builder {
        unimplemented!()
    }
    
    fn get_connection(&self) -> &DatabaseConnection {
        unimplemented!()
    }

    async fn get_web_transaction(&self, transaction_id: ModelId) -> Result<MockWebTransaction, Box<dyn Error + Send + Sync>> {
        self.get_transaction(transaction_id).await
    }
}

// Simple CommonStorage implementations for API testing
pub struct MockSuccessStorage {
    pub next_label_id: ModelId,
}

impl MockSuccessStorage {
    pub fn new(next_label_id: ModelId) -> Self {
        Self { next_label_id }
    }
}

#[async_trait::async_trait]
impl CommonStorage for MockSuccessStorage {
    async fn save_transaction(&self) -> Result<ModelId, Box<dyn Error + Send + Sync>> { unimplemented!() }
    async fn mark_transaction_processed(&self, _transaction_id: ModelId) -> Result<(), Box<dyn Error + Send + Sync>> { unimplemented!() }

    async fn update_transaction_label(&self, _transaction_id: ModelId, _label_id: ModelId) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(())
    }

    // Stub implementations for unused methods in API tests
    async fn save_features(&self, _transaction_id: ModelId, _simple_features: Option<&[processing::model::Feature]>, _graph_features: &[processing::model::Feature]) -> Result<(), Box<dyn Error + Send + Sync>> { unimplemented!() }
    async fn save_scores(&self, _transaction_id: ModelId, _channel_id: ModelId, _total_score: i32, _triggered_rules: &[processing::model::TriggeredRule]) -> Result<(), Box<dyn Error + Send + Sync>> { unimplemented!() }
    async fn get_features(&self, _transaction_id: ModelId) -> Result<(Option<Vec<processing::model::Feature>>, Vec<processing::model::Feature>), Box<dyn Error + Send + Sync>> { unimplemented!() }
    async fn find_connected_transactions(&self, _transaction_id: ModelId, _max_depth: Option<i32>, _limit_count: Option<i32>, _min_created_at: Option<chrono::DateTime<chrono::Utc>>, _max_created_at: Option<chrono::DateTime<chrono::Utc>>, _min_confidence: Option<i32>) -> Result<Vec<processing::model::ConnectedTransaction>, Box<dyn Error + Send + Sync>> { unimplemented!() }
    async fn get_direct_connections(&self, _transaction_id: ModelId) -> Result<Vec<processing::model::DirectConnection>, Box<dyn Error + Send + Sync>> { unimplemented!() }
    async fn save_matching_fields(&self, _transaction_id: ModelId, _matching_fields: &[processing::model::MatchingField]) -> Result<(), Box<dyn Error + Send + Sync>> { unimplemented!() }
    async fn get_channels(&self, _model_id: ModelId) -> Result<Vec<processing::storage::sea_orm_storage_model::channel::Model>, Box<dyn Error + Send + Sync>> { unimplemented!() }
    async fn get_scoring_events(&self, _transaction_id: ModelId) -> Result<Vec<processing::storage::sea_orm_storage_model::scoring_event::Model>, Box<dyn Error + Send + Sync>> { unimplemented!() }
    async fn get_triggered_rules(&self, _scoring_event_id: ModelId) -> Result<Vec<processing::model::TriggeredRule>, Box<dyn Error + Send + Sync>> { unimplemented!() }
    // removed save_label/get_label from trait

    async fn label_transactions(&self, transaction_ids: &[ModelId], _fraud_level: FraudLevel, _fraud_category: String, _labeled_by: String) -> Result<LabelingResult, Box<dyn Error + Send + Sync>> {
        Ok(LabelingResult {
            label_id: self.next_label_id,
            success_count: transaction_ids.len(),
            failed_transaction_ids: vec![],
        })
    }
}

pub struct MockSaveLabelErrorStorage;

#[async_trait::async_trait]
impl CommonStorage for MockSaveLabelErrorStorage {
    async fn save_transaction(&self) -> Result<ModelId, Box<dyn Error + Send + Sync>> { unimplemented!() }
    async fn mark_transaction_processed(&self, _transaction_id: ModelId) -> Result<(), Box<dyn Error + Send + Sync>> { unimplemented!() }

    async fn update_transaction_label(&self, _transaction_id: ModelId, _label_id: ModelId) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(())
    }

    // Stub implementations for unused methods
    async fn save_features(&self, _transaction_id: ModelId, _simple_features: Option<&[processing::model::Feature]>, _graph_features: &[processing::model::Feature]) -> Result<(), Box<dyn Error + Send + Sync>> { unimplemented!() }
    async fn save_scores(&self, _transaction_id: ModelId, _channel_id: ModelId, _total_score: i32, _triggered_rules: &[processing::model::TriggeredRule]) -> Result<(), Box<dyn Error + Send + Sync>> { unimplemented!() }
    async fn get_features(&self, _transaction_id: ModelId) -> Result<(Option<Vec<processing::model::Feature>>, Vec<processing::model::Feature>), Box<dyn Error + Send + Sync>> { unimplemented!() }
    async fn find_connected_transactions(&self, _transaction_id: ModelId, _max_depth: Option<i32>, _limit_count: Option<i32>, _min_created_at: Option<chrono::DateTime<chrono::Utc>>, _max_created_at: Option<chrono::DateTime<chrono::Utc>>, _min_confidence: Option<i32>) -> Result<Vec<processing::model::ConnectedTransaction>, Box<dyn Error + Send + Sync>> { unimplemented!() }
    async fn get_direct_connections(&self, _transaction_id: ModelId) -> Result<Vec<processing::model::DirectConnection>, Box<dyn Error + Send + Sync>> { unimplemented!() }
    async fn save_matching_fields(&self, _transaction_id: ModelId, _matching_fields: &[processing::model::MatchingField]) -> Result<(), Box<dyn Error + Send + Sync>> { unimplemented!() }
    async fn get_channels(&self, _model_id: ModelId) -> Result<Vec<processing::storage::sea_orm_storage_model::channel::Model>, Box<dyn Error + Send + Sync>> { unimplemented!() }
    async fn get_scoring_events(&self, _transaction_id: ModelId) -> Result<Vec<processing::storage::sea_orm_storage_model::scoring_event::Model>, Box<dyn Error + Send + Sync>> { unimplemented!() }
    async fn get_triggered_rules(&self, _scoring_event_id: ModelId) -> Result<Vec<processing::model::TriggeredRule>, Box<dyn Error + Send + Sync>> { unimplemented!() }
    // removed save_label/get_label from trait
    async fn label_transactions(&self, _transaction_ids: &[ModelId], _fraud_level: FraudLevel, _fraud_category: String, _labeled_by: String) -> Result<LabelingResult, Box<dyn Error + Send + Sync>> {
        Err("Failed to save label".into())
    }
}

pub struct MockPartialSuccessStorage {
    pub next_label_id: ModelId,
    pub successful_tx_id: ModelId,
}

impl MockPartialSuccessStorage {
    pub fn new(next_label_id: ModelId, successful_tx_id: ModelId) -> Self {
        Self { next_label_id, successful_tx_id }
    }
}

#[async_trait::async_trait]
impl CommonStorage for MockPartialSuccessStorage {
    async fn save_transaction(&self) -> Result<ModelId, Box<dyn Error + Send + Sync>> { unimplemented!() }
    async fn mark_transaction_processed(&self, _transaction_id: ModelId) -> Result<(), Box<dyn Error + Send + Sync>> { unimplemented!() }

    async fn update_transaction_label(&self, transaction_id: ModelId, _label_id: ModelId) -> Result<(), Box<dyn Error + Send + Sync>> {
        if transaction_id == self.successful_tx_id {
            Ok(())
        } else {
            Err("Update failed for this transaction".into())
        }
    }

    // Stub implementations for unused methods
    async fn save_features(&self, _transaction_id: ModelId, _simple_features: Option<&[processing::model::Feature]>, _graph_features: &[processing::model::Feature]) -> Result<(), Box<dyn Error + Send + Sync>> { unimplemented!() }
    async fn save_scores(&self, _transaction_id: ModelId, _channel_id: ModelId, _total_score: i32, _triggered_rules: &[processing::model::TriggeredRule]) -> Result<(), Box<dyn Error + Send + Sync>> { unimplemented!() }
    async fn get_features(&self, _transaction_id: ModelId) -> Result<(Option<Vec<processing::model::Feature>>, Vec<processing::model::Feature>), Box<dyn Error + Send + Sync>> { unimplemented!() }
    async fn find_connected_transactions(&self, _transaction_id: ModelId, _max_depth: Option<i32>, _limit_count: Option<i32>, _min_created_at: Option<chrono::DateTime<chrono::Utc>>, _max_created_at: Option<chrono::DateTime<chrono::Utc>>, _min_confidence: Option<i32>) -> Result<Vec<processing::model::ConnectedTransaction>, Box<dyn Error + Send + Sync>> { unimplemented!() }
    async fn get_direct_connections(&self, _transaction_id: ModelId) -> Result<Vec<processing::model::DirectConnection>, Box<dyn Error + Send + Sync>> { unimplemented!() }
    async fn save_matching_fields(&self, _transaction_id: ModelId, _matching_fields: &[processing::model::MatchingField]) -> Result<(), Box<dyn Error + Send + Sync>> { unimplemented!() }
    async fn get_channels(&self, _model_id: ModelId) -> Result<Vec<processing::storage::sea_orm_storage_model::channel::Model>, Box<dyn Error + Send + Sync>> { unimplemented!() }
    async fn get_scoring_events(&self, _transaction_id: ModelId) -> Result<Vec<processing::storage::sea_orm_storage_model::scoring_event::Model>, Box<dyn Error + Send + Sync>> { unimplemented!() }
    async fn get_triggered_rules(&self, _scoring_event_id: ModelId) -> Result<Vec<processing::model::TriggeredRule>, Box<dyn Error + Send + Sync>> { unimplemented!() }
    // removed save_label/get_label from trait
    async fn label_transactions(&self, transaction_ids: &[ModelId], _fraud_level: FraudLevel, _fraud_category: String, _labeled_by: String) -> Result<LabelingResult, Box<dyn Error + Send + Sync>> {
        let mut success_count = 0;
        let mut failed = Vec::new();
        for &id in transaction_ids {
            if id == self.successful_tx_id { success_count += 1; } else { failed.push(id); }
        }
        Ok(LabelingResult { label_id: self.next_label_id, success_count, failed_transaction_ids: failed })
    }
}

// Helper function to create test app - now uses centralized mocks
pub fn create_test_app(
    web_storage: Arc<dyn WebStorage<MockWebTransaction>>,
    common_storage: Arc<dyn CommonStorage>,
) -> Router {
    let app_state = AppState::new(web_storage, common_storage);
    
    Router::new()
        .route("/transaction/{transaction_id}/label", axum::routing::post(label_transaction::<MockWebTransaction>))
        .with_state(app_state)
}

pub async fn response_body_string(response: axum::response::Response) -> String {
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await
        .expect("Failed to read response body");
    String::from_utf8(body_bytes.to_vec())
        .expect("Response body is not valid UTF-8")
} 