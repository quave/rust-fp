use std::sync::Arc;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::json;
use tower::ServiceExt;
use common::test_helpers::{TestResult, test_utils};

use super::mocks::{
    MockWebTransaction, MockWebStorage, MockSuccessStorage, MockSaveLabelErrorStorage,
    create_test_app, response_body_string
};

#[tokio::test]
async fn test_label_transaction_invalid_json() -> TestResult {
    // Arrange: Set up the mock state using constructor methods
    let web_storage = Arc::new(MockWebStorage::new(vec![
        MockWebTransaction::new(1)
    ]));
    let common_storage = Arc::new(MockSuccessStorage { next_label_id: 42 });
    
    // Create app with the router
    let app = create_test_app(web_storage, common_storage);
    
    // Act: Send a request with invalid JSON using safe request builder
    let request = test_utils::build_request("POST", "/transaction/1/label", Some("{invalid json}".to_string()))?;
    let request = Request::from_parts(request.into_parts().0, Body::from(request.into_body()));
    
    let response = app.oneshot(request).await
        .map_err(|e| common::test_helpers::TestError::generic(format!("Request failed: {}", e)))?;
    
    // Assert: Verify the response is a bad request using safe status check
    test_utils::check_status_code(response.status(), StatusCode::BAD_REQUEST)?;
    
    Ok(())
}

#[tokio::test]
async fn test_label_transaction_missing_required_fields() {
    // Arrange: Set up the mock state with a simple storage using constructor methods
    let web_storage = Arc::new(MockWebStorage::new(vec![
        MockWebTransaction::new(1)
    ]));
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
        .uri("/transaction/1/label")
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
    // Arrange: Set up the mock state with a storage that fails to save labels using constructor methods
    let web_storage = Arc::new(MockWebStorage::new(vec![
        MockWebTransaction::new(1)
    ]));
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
        .uri("/transaction/1/label")
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
async fn test_label_transaction_invalid_transaction_id() {
    // Arrange: Set up mock with valid transaction using constructor
    let web_storage = Arc::new(MockWebStorage::new(vec![
        MockWebTransaction::new(1)
    ]));
    let common_storage = Arc::new(MockSuccessStorage { next_label_id: 42 });
    
    // Create app with the router
    let app = create_test_app(web_storage, common_storage);
    
    // Act: Send a request with invalid transaction ID (non-existent transaction)
    let request_body = json!({
        "transaction_ids": [999], // This transaction doesn't exist
        "fraud_level": "Fraud",
        "fraud_category": "Test Fraud",
        "labeled_by": "test_user"
    });
    
    let request = Request::builder()
        .uri("/transaction/999/label")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    
    // Assert: The API doesn't validate transaction existence, so it returns success
    // The business logic processes the request and returns the label_id
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_body_string(response).await;
    assert_eq!(body, "42"); // Should return the label_id from MockSuccessStorage
}

#[tokio::test]
async fn test_label_transaction_invalid_fraud_level() {
    // Arrange: Set up mock with valid transaction using constructor
    let web_storage = Arc::new(MockWebStorage::new(vec![
        MockWebTransaction::new(1)
    ]));
    let common_storage = Arc::new(MockSuccessStorage { next_label_id: 42 });
    
    // Create app with the router
    let app = create_test_app(web_storage, common_storage);
    
    // Act: Send a request with invalid fraud level
    let request_body = json!({
        "transaction_ids": [1],
        "fraud_level": "InvalidLevel", // Invalid fraud level
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
    
    // Assert: Invalid fraud level should cause JSON deserialization to fail
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_label_transaction_missing_required_field() {
    // Arrange: Set up mock with valid transaction using constructor
    let web_storage = Arc::new(MockWebStorage::new(vec![
        MockWebTransaction::new(1)
    ]));
    let common_storage = Arc::new(MockSuccessStorage { next_label_id: 42 });
    
    // Create app with the router
    let app = create_test_app(web_storage, common_storage);
    
    // Act: Send a request missing required field (fraud_category)
    let request_body = json!({
        "transaction_ids": [1],
        "fraud_level": "Fraud",
        // "fraud_category": "Test Fraud", // Missing required field
        "labeled_by": "test_user"
    });
    
    let request = Request::builder()
        .uri("/transaction/1/label")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    
    // Assert: Missing required field should cause JSON deserialization to fail
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
} 