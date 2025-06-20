use std::sync::Arc;
use std::error::Error;
use axum::{
    body::Body,
    http::{Request, StatusCode},
    response::Response,
    Router,
};
use processing::{
    executable_utils::{label_transaction, AppState},
    model::{Label, ModelId, ModelRegistryProvider, WebTransaction},
    storage::{CommonStorage, WebStorage}, ui_model::ModelRegistry,
};
use serde_json::json;
use tower::ServiceExt;
use http_body_util::BodyExt;

// Mock WebTransaction implementation
#[derive(Debug, Clone, serde::Serialize)]
struct MockWebTransaction {
    id: ModelId,
    label_id: Option<ModelId>,
}

impl WebTransaction for MockWebTransaction {
    fn id(&self) -> ModelId {
        self.id
    }
}
impl ModelRegistryProvider for MockWebTransaction {
    fn get_registry() -> &'static ModelRegistry {
        unimplemented!("Not used in label_transaction tests")
    }
}

// Mock web storage
struct MockWebStorage {
    transactions: Vec<MockWebTransaction>,
}

#[async_trait::async_trait]
impl WebStorage<MockWebTransaction> for MockWebStorage {
    async fn get_transactions(&self, _filter: processing::ui_model::FilterRequest) -> Result<Vec<MockWebTransaction>, Box<dyn Error + Send + Sync>> {
        Ok(self.transactions.clone())
    }
    
    async fn get_transaction(
        &self,
        transaction_id: ModelId,
    ) -> Result<MockWebTransaction, Box<dyn Error + Send + Sync>> {
        self.transactions
            .iter()
            .find(|t| t.id == transaction_id)
            .cloned()
            .ok_or_else(|| format!("Transaction not found: {}", transaction_id).into())
    }
}

// Mock CommonStorage that succeeds
struct MockSuccessStorage {
    next_label_id: ModelId,
}

#[async_trait::async_trait]
impl CommonStorage for MockSuccessStorage {
    // Implement only the methods used by the label_transaction endpoint
    async fn save_label(
        &self,
        _label: &Label,
    ) -> Result<ModelId, Box<dyn Error + Send + Sync>> {
        Ok(self.next_label_id)
    }

    async fn update_transaction_label(
        &self,
        _transaction_id: ModelId,
        _label_id: ModelId,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(())
    }

    // Provide stub implementations for other required methods
    async fn save_features(
        &self,
        _transaction_id: ModelId,
        _features: &[processing::model::Feature],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        unimplemented!("Not used in label_transaction tests")
    }

    async fn save_scores(
        &self,
        _transaction_id: ModelId,
        _channel_id: ModelId,
        _total_score: i32,
        _triggered_rules: &[processing::model::TriggeredRule],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        unimplemented!("Not used in label_transaction tests")
    }

    async fn get_features(
        &self,
        _transaction_id: ModelId,
    ) -> Result<Vec<processing::model::Feature>, Box<dyn Error + Send + Sync>> {
        unimplemented!("Not used in label_transaction tests")
    }

    async fn find_connected_transactions(
        &self,
        _transaction_id: ModelId,
        _max_depth: Option<i32>,
        _limit_count: Option<i32>,
        _min_created_at: Option<chrono::DateTime<chrono::Utc>>,
        _max_created_at: Option<chrono::DateTime<chrono::Utc>>,
        _min_confidence: Option<i32>,
    ) -> Result<Vec<processing::model::ConnectedTransaction>, Box<dyn Error + Send + Sync>> {
        unimplemented!("Not used in label_transaction tests")
    }

    async fn get_direct_connections(
        &self,
        _transaction_id: ModelId,
    ) -> Result<Vec<processing::model::DirectConnection>, Box<dyn Error + Send + Sync>> {
        unimplemented!("Not used in label_transaction tests")
    }

    async fn save_matching_fields(
        &self,
        _transaction_id: ModelId,
        _matching_fields: &[processing::model::MatchingField],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        unimplemented!("Not used in label_transaction tests")
    }

    async fn get_channels(
        &self,
        _model_id: ModelId,
    ) -> Result<Vec<processing::model::Channel>, Box<dyn Error + Send + Sync>> {
        unimplemented!("Not used in label_transaction tests")
    }

    async fn get_scoring_events(
        &self,
        _transaction_id: ModelId,
    ) -> Result<Vec<processing::model::ScoringEvent>, Box<dyn Error + Send + Sync>> {
        unimplemented!("Not used in label_transaction tests")
    }

    async fn get_triggered_rules(
        &self,
        _scoring_event_id: ModelId,
    ) -> Result<Vec<processing::model::TriggeredRule>, Box<dyn Error + Send + Sync>> {
        unimplemented!("Not used in label_transaction tests")
    }

    async fn get_label(
        &self,
        _label_id: ModelId,
    ) -> Result<Label, Box<dyn Error + Send + Sync>> {
        unimplemented!("Not used in label_transaction tests")
    }
}

// Mock CommonStorage that fails when saving labels
struct MockSaveLabelErrorStorage {}

#[async_trait::async_trait]
impl CommonStorage for MockSaveLabelErrorStorage {
    async fn save_label(
        &self,
        _label: &Label,
    ) -> Result<ModelId, Box<dyn Error + Send + Sync>> {
        Err("Failed to save label".into())
    }

    // All other methods are stubs, same as in MockSuccessStorage
    async fn update_transaction_label(
        &self,
        _transaction_id: ModelId,
        _label_id: ModelId,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        unimplemented!("Not reached in this test")
    }

    async fn save_features(
        &self,
        _transaction_id: ModelId,
        _features: &[processing::model::Feature],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        unimplemented!("Not used in label_transaction tests")
    }

    async fn save_scores(
        &self,
        _transaction_id: ModelId,
        _channel_id: ModelId,
        _total_score: i32,
        _triggered_rules: &[processing::model::TriggeredRule],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        unimplemented!("Not used in label_transaction tests")
    }

    async fn get_features(
        &self,
        _transaction_id: ModelId,
    ) -> Result<Vec<processing::model::Feature>, Box<dyn Error + Send + Sync>> {
        unimplemented!("Not used in label_transaction tests")
    }

    async fn find_connected_transactions(
        &self,
        _transaction_id: ModelId,
        _max_depth: Option<i32>,
        _limit_count: Option<i32>,
        _min_created_at: Option<chrono::DateTime<chrono::Utc>>,
        _max_created_at: Option<chrono::DateTime<chrono::Utc>>,
        _min_confidence: Option<i32>,
    ) -> Result<Vec<processing::model::ConnectedTransaction>, Box<dyn Error + Send + Sync>> {
        unimplemented!("Not used in label_transaction tests")
    }

    async fn get_direct_connections(
        &self,
        _transaction_id: ModelId,
    ) -> Result<Vec<processing::model::DirectConnection>, Box<dyn Error + Send + Sync>> {
        unimplemented!("Not used in label_transaction tests")
    }

    async fn save_matching_fields(
        &self,
        _transaction_id: ModelId,
        _matching_fields: &[processing::model::MatchingField],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        unimplemented!("Not used in label_transaction tests")
    }

    async fn get_channels(
        &self,
        _model_id: ModelId,
    ) -> Result<Vec<processing::model::Channel>, Box<dyn Error + Send + Sync>> {
        unimplemented!("Not used in label_transaction tests")
    }

    async fn get_scoring_events(
        &self,
        _transaction_id: ModelId,
    ) -> Result<Vec<processing::model::ScoringEvent>, Box<dyn Error + Send + Sync>> {
        unimplemented!("Not used in label_transaction tests")
    }

    async fn get_triggered_rules(
        &self,
        _scoring_event_id: ModelId,
    ) -> Result<Vec<processing::model::TriggeredRule>, Box<dyn Error + Send + Sync>> {
        unimplemented!("Not used in label_transaction tests")
    }

    async fn get_label(
        &self,
        _label_id: ModelId,
    ) -> Result<Label, Box<dyn Error + Send + Sync>> {
        unimplemented!("Not used in label_transaction tests")
    }
}

// Mock CommonStorage for partial success
struct MockPartialSuccessStorage {
    next_label_id: ModelId,
    successful_tx_id: ModelId,
}

#[async_trait::async_trait]
impl CommonStorage for MockPartialSuccessStorage {
    async fn save_label(
        &self,
        _label: &Label,
    ) -> Result<ModelId, Box<dyn Error + Send + Sync>> {
        Ok(self.next_label_id)
    }

    async fn update_transaction_label(
        &self,
        transaction_id: ModelId,
        _label_id: ModelId,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        if transaction_id == self.successful_tx_id {
            Ok(())
        } else {
            Err(format!("Failed to update transaction {}", transaction_id).into())
        }
    }

    // All other methods are stubs
    async fn save_features(
        &self,
        _transaction_id: ModelId,
        _features: &[processing::model::Feature],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        unimplemented!("Not used in label_transaction tests")
    }

    async fn save_scores(
        &self,
        _transaction_id: ModelId,
        _channel_id: ModelId,
        _total_score: i32,
        _triggered_rules: &[processing::model::TriggeredRule],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        unimplemented!("Not used in label_transaction tests")
    }

    async fn get_features(
        &self,
        _transaction_id: ModelId,
    ) -> Result<Vec<processing::model::Feature>, Box<dyn Error + Send + Sync>> {
        unimplemented!("Not used in label_transaction tests")
    }

    async fn find_connected_transactions(
        &self,
        _transaction_id: ModelId,
        _max_depth: Option<i32>,
        _limit_count: Option<i32>,
        _min_created_at: Option<chrono::DateTime<chrono::Utc>>,
        _max_created_at: Option<chrono::DateTime<chrono::Utc>>,
        _min_confidence: Option<i32>,
    ) -> Result<Vec<processing::model::ConnectedTransaction>, Box<dyn Error + Send + Sync>> {
        unimplemented!("Not used in label_transaction tests")
    }

    async fn get_direct_connections(
        &self,
        _transaction_id: ModelId,
    ) -> Result<Vec<processing::model::DirectConnection>, Box<dyn Error + Send + Sync>> {
        unimplemented!("Not used in label_transaction tests")
    }

    async fn save_matching_fields(
        &self,
        _transaction_id: ModelId,
        _matching_fields: &[processing::model::MatchingField],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        unimplemented!("Not used in label_transaction tests")
    }

    async fn get_channels(
        &self,
        _model_id: ModelId,
    ) -> Result<Vec<processing::model::Channel>, Box<dyn Error + Send + Sync>> {
        unimplemented!("Not used in label_transaction tests")
    }

    async fn get_scoring_events(
        &self,
        _transaction_id: ModelId,
    ) -> Result<Vec<processing::model::ScoringEvent>, Box<dyn Error + Send + Sync>> {
        unimplemented!("Not used in label_transaction tests")
    }

    async fn get_triggered_rules(
        &self,
        _scoring_event_id: ModelId,
    ) -> Result<Vec<processing::model::TriggeredRule>, Box<dyn Error + Send + Sync>> {
        unimplemented!("Not used in label_transaction tests")
    }

    async fn get_label(
        &self,
        _label_id: ModelId,
    ) -> Result<Label, Box<dyn Error + Send + Sync>> {
        unimplemented!("Not used in label_transaction tests")
    }
}

// Helper function for creating an app with the correct router configuration
fn create_test_app(
    web_storage: Arc<dyn WebStorage<MockWebTransaction>>,
    common_storage: Arc<dyn CommonStorage>,
) -> Router {
    // SAFETY: This uses transmute to work around the private fields in AppState.
    // This is acceptable in tests but would be inappropriate in production code.
    let state = unsafe {
        std::mem::transmute::<_, AppState<MockWebTransaction>>((web_storage, common_storage))
    };
    
    Router::new()
        .route("/api/transactions/label", axum::routing::post(label_transaction::<MockWebTransaction>))
        .with_state(state)
}

// Helper function to extract body from response
async fn response_body_string(response: Response) -> String {
    let body = response.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(body.to_vec()).unwrap()
}

#[tokio::test]
async fn test_label_transaction_successful_request() {
    // Arrange: Set up the mock state with test data
    let web_storage = Arc::new(MockWebStorage {
        transactions: vec![
            MockWebTransaction { id: 1, label_id: None }
        ],
    });
    let common_storage = Arc::new(MockSuccessStorage { next_label_id: 42 });
    
    // Create app with the router
    let app = create_test_app(web_storage, common_storage);
    
    // Act: Send a request to label a transaction
    let request_body = json!({
        "transaction_ids": [1],
        "fraud_level": "Fraud",
        "fraud_category": "Test Fraud",
        "labeled_by": "test_user"
    });
    
    let request = Request::builder()
        .uri("/api/transactions/label")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    
    // Assert: Verify the response is as expected
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response_body_string(response).await, "42");
}

#[tokio::test]
async fn test_label_transaction_multiple_transactions() {
    // Arrange: Set up the mock state with multiple transactions
    let web_storage = Arc::new(MockWebStorage {
        transactions: vec![
            MockWebTransaction { id: 1, label_id: None },
            MockWebTransaction { id: 2, label_id: None },
            MockWebTransaction { id: 3, label_id: None },
        ],
    });
    let common_storage = Arc::new(MockSuccessStorage { next_label_id: 42 });
    
    // Create app with the router
    let app = create_test_app(web_storage, common_storage);
    
    // Act: Send a request to label multiple transactions
    let request_body = json!({
        "transaction_ids": [1, 2, 3],
        "fraud_level": "Fraud",
        "fraud_category": "Batch Test",
        "labeled_by": "test_user"
    });
    
    let request = Request::builder()
        .uri("/api/transactions/label")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    
    // Assert: Verify the response is as expected
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response_body_string(response).await, "42");
}

#[tokio::test]
async fn test_label_transaction_invalid_json() {
    // Arrange: Set up the mock state
    let web_storage = Arc::new(MockWebStorage {
        transactions: vec![MockWebTransaction { id: 1, label_id: None }],
    });
    let common_storage = Arc::new(MockSuccessStorage { next_label_id: 42 });
    
    // Create app with the router
    let app = create_test_app(web_storage, common_storage);
    
    // Act: Send a request with invalid JSON
    let request = Request::builder()
        .uri("/api/transactions/label")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from("{invalid json}"))
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    
    // Assert: Verify the response is a bad request
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_label_transaction_missing_required_fields() {
    // Arrange: Set up the mock state with a simple storage
    let web_storage = Arc::new(MockWebStorage {
        transactions: vec![MockWebTransaction { id: 1, label_id: None }],
    });
    let common_storage = Arc::new(MockSuccessStorage { next_label_id: 42 });
    
    let app = create_test_app(web_storage, common_storage);
    
    // Act: Send a request with missing required fields
    let request_body = json!({
        // Missing transaction_ids
        "fraud_level": "Fraud",
        "fraud_category": "Test Fraud",
        // Missing labeled_by
    });
    
    let request = Request::builder()
        .uri("/api/transactions/label")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    
    // Assert: Verify the response is an unprocessable entity due to missing fields
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_label_transaction_save_label_error() {
    // Arrange: Set up the mock state with a storage that fails to save labels
    let web_storage = Arc::new(MockWebStorage {
        transactions: vec![MockWebTransaction { id: 1, label_id: None }],
    });
    let common_storage = Arc::new(MockSaveLabelErrorStorage {});
    
    // Create app with the router
    let app = create_test_app(web_storage, common_storage);
    
    // Act: Send a request to label a transaction
    let request_body = json!({
        "transaction_ids": [1],
        "fraud_level": "Fraud",
        "fraud_category": "Test Fraud",
        "labeled_by": "test_user"
    });
    
    let request = Request::builder()
        .uri("/api/transactions/label")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    
    // Assert: Verify the response is an internal server error
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = response_body_string(response).await;
    assert!(body.contains("Failed to save label"), "Response should contain the error message");
}

#[tokio::test]
async fn test_label_transaction_partial_success() {
    // Arrange: Set up the mock state with a storage that succeeds for some transactions but fails for others
    let web_storage = Arc::new(MockWebStorage {
        transactions: vec![
            MockWebTransaction { id: 1, label_id: None },
            MockWebTransaction { id: 2, label_id: None },
        ],
    });
    let common_storage = Arc::new(MockPartialSuccessStorage { 
        next_label_id: 42,
        successful_tx_id: 1, // Only transaction 1 will be updated successfully
    });
    
    // Create app with the router
    let app = create_test_app(web_storage, common_storage);
    
    // Act: Send a request to label multiple transactions
    let request_body = json!({
        "transaction_ids": [1, 2],
        "fraud_level": "Fraud",
        "fraud_category": "Partial Test",
        "labeled_by": "test_user"
    });
    
    let request = Request::builder()
        .uri("/api/transactions/label")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    
    // Assert: Verify the response is a partial content status with details
    assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
    let body = response_body_string(response).await;
    assert!(body.contains("Partially successful"), "Response should indicate partial success");
    assert!(body.contains("1/2 transactions"), "Response should show the success ratio");
    assert!(body.contains("Failed IDs: [2]"), "Response should list the failed transaction IDs");
} 