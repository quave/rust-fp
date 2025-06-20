use std::sync::Arc;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::json;
use tower::ServiceExt;

use super::mocks::{
    MockWebTransaction, MockWebStorage, MockSuccessStorage, 
    create_test_app, response_body_string
};

#[tokio::test]
async fn test_label_transaction_successful_request() {
    // Arrange: Set up the mock state with test data using constructor methods
    let web_storage = Arc::new(MockWebStorage::new(vec![
        MockWebTransaction::new(1)
    ]));
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
        .uri("/transaction/1/label")
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
    // Arrange: Set up the mock state with multiple transactions using constructor methods
    let web_storage = Arc::new(MockWebStorage::new(vec![
        MockWebTransaction::new(1),
        MockWebTransaction::new(2),
        MockWebTransaction::new(3),
    ]));
    let common_storage = Arc::new(MockSuccessStorage { next_label_id: 42 });
    
    // Act: Send a request to label multiple transactions (test each one individually)
    for tx_id in [1, 2, 3] {
        // Create a new app for each request since oneshot consumes the app
        let app = create_test_app(web_storage.clone(), common_storage.clone());
        
        let request_body = json!({
            "transaction_ids": [tx_id],
            "fraud_level": "Fraud",
            "fraud_category": "Batch Test",
            "labeled_by": "test_user"
        });
        
        let request = Request::builder()
            .uri(&format!("/transaction/{}/label", tx_id))
            .method("POST")
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_string(&request_body).unwrap()))
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        
        // Assert: Verify the response is as expected
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response_body_string(response).await, "42");
    }
} 