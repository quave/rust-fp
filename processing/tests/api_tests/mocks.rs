use std::sync::Arc;
use std::error::Error;
use axum::Router;
use processing::{
    executable_utils::{label_transaction, AppState},
    model::{Label, ModelId, ModelRegistryProvider, WebTransaction},
    storage::{CommonStorage, WebStorage}, 
    ui_model::ModelRegistry,
};
use async_trait::async_trait;

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

impl ModelRegistryProvider for MockWebTransaction {
    fn get_registry() -> &'static ModelRegistry {
        use std::sync::LazyLock;
        // Return a static registry for testing
        static REGISTRY: LazyLock<ModelRegistry> = LazyLock::new(|| ModelRegistry {
            models: std::collections::HashMap::new(),
            root_model: "test_transaction",
        });
        &REGISTRY
    }
}

// Implement WebStorage trait for MockWebStorage
#[async_trait]
impl WebStorage<MockWebTransaction> for MockWebStorage {
    async fn get_transactions(&self, _filter: processing::ui_model::FilterRequest) -> Result<Vec<MockWebTransaction>, Box<dyn Error + Send + Sync>> {
        self.get_transactions().await
    }
    
    async fn get_transaction(&self, transaction_id: ModelId) -> Result<MockWebTransaction, Box<dyn Error + Send + Sync>> {
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
    async fn save_label(&self, _label: &Label) -> Result<ModelId, Box<dyn Error + Send + Sync>> {
        Ok(self.next_label_id)
    }

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
    async fn get_channels(&self, _model_id: ModelId) -> Result<Vec<processing::model::Channel>, Box<dyn Error + Send + Sync>> { unimplemented!() }
    async fn get_scoring_events(&self, _transaction_id: ModelId) -> Result<Vec<processing::model::ScoringEvent>, Box<dyn Error + Send + Sync>> { unimplemented!() }
    async fn get_triggered_rules(&self, _scoring_event_id: ModelId) -> Result<Vec<processing::model::TriggeredRule>, Box<dyn Error + Send + Sync>> { unimplemented!() }
    async fn get_label(&self, _label_id: ModelId) -> Result<Label, Box<dyn Error + Send + Sync>> { 
        Ok(Label {
            id: _label_id,
            fraud_level: processing::model::FraudLevel::NoFraud,
            fraud_category: "test".to_string(),
            label_source: processing::model::LabelSource::Manual,
            labeled_by: "test".to_string(),
            created_at: chrono::Utc::now(),
        })
    }
}

pub struct MockSaveLabelErrorStorage;

#[async_trait::async_trait]
impl CommonStorage for MockSaveLabelErrorStorage {
    async fn save_label(&self, _label: &Label) -> Result<ModelId, Box<dyn Error + Send + Sync>> {
        Err("Failed to save label".into())
    }

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
    async fn get_channels(&self, _model_id: ModelId) -> Result<Vec<processing::model::Channel>, Box<dyn Error + Send + Sync>> { unimplemented!() }
    async fn get_scoring_events(&self, _transaction_id: ModelId) -> Result<Vec<processing::model::ScoringEvent>, Box<dyn Error + Send + Sync>> { unimplemented!() }
    async fn get_triggered_rules(&self, _scoring_event_id: ModelId) -> Result<Vec<processing::model::TriggeredRule>, Box<dyn Error + Send + Sync>> { unimplemented!() }
    async fn get_label(&self, _label_id: ModelId) -> Result<Label, Box<dyn Error + Send + Sync>> { unimplemented!() }
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
    async fn save_label(&self, _label: &Label) -> Result<ModelId, Box<dyn Error + Send + Sync>> {
        Ok(self.next_label_id)
    }

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
    async fn get_channels(&self, _model_id: ModelId) -> Result<Vec<processing::model::Channel>, Box<dyn Error + Send + Sync>> { unimplemented!() }
    async fn get_scoring_events(&self, _transaction_id: ModelId) -> Result<Vec<processing::model::ScoringEvent>, Box<dyn Error + Send + Sync>> { unimplemented!() }
    async fn get_triggered_rules(&self, _scoring_event_id: ModelId) -> Result<Vec<processing::model::TriggeredRule>, Box<dyn Error + Send + Sync>> { unimplemented!() }
    async fn get_label(&self, _label_id: ModelId) -> Result<Label, Box<dyn Error + Send + Sync>> { unimplemented!() }
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