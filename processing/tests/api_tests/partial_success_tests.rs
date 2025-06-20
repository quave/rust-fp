use std::sync::Arc;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::json;
use tower::ServiceExt;

use super::mocks::{
    MockWebTransaction, MockWebStorage, MockPartialSuccessStorage,
    create_test_app, response_body_string
};

#[tokio::test]
async fn test_label_transaction_partial_success() {
    // Arrange: Set up the mock state with partial success storage using constructor methods
    let web_storage = Arc::new(MockWebStorage::new(vec![
        MockWebTransaction::new(1),
        MockWebTransaction::new(2),
    ]));
    let common_storage = Arc::new(MockPartialSuccessStorage {
        next_label_id: 42,
        successful_tx_id: 1, // Only transaction 1 will succeed
    });
    
    // Create app with the router
    let app = create_test_app(web_storage, common_storage);
    
    // Act: Send a request to label transaction 1 (should succeed)
    let request_body = json!({
        "transaction_ids": [1],
        "fraud_level": "Fraud",
        "fraud_category": "Test Fraud",
        "labeled_by": "test_user"
    });
    
    let request = Request::builder()
        .uri("/transaction/1/label")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    
    // Assert: Verify the response is successful
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response_body_string(response).await, "42");
} 