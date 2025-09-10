use std::sync::Arc;
use axum::http::StatusCode;
use super::super::mocks::{MockWebTransaction, create_mock_web_storage, create_mock_common_storage};
use processing::{
    executable_utils::{label_transaction, AppState, LabelRequest}, model::{FraudLevel, LabelSource}, storage::CommonStorage
};

#[tokio::test]
async fn test_label_transaction_successful_request() {
    // Arrange: Set up the mock state with test data using constructor methods
    let web_storage = Arc::new(create_mock_web_storage(vec![
        MockWebTransaction::new(1)
    ]));
    let mut common_storage = create_mock_common_storage(Some(42), vec![]);
    common_storage.expect_label_transactions().returning(|_, _, _, _, _| {
        Ok(())
    });
    let app_state = AppState::new(web_storage.clone(), Arc::new(common_storage));

    // Act: Send a request to label a transaction
    let request_body: axum::Json<LabelRequest> = LabelRequest {
        transaction_ids: vec![1],
        fraud_level: FraudLevel::Fraud,
        fraud_category: "Test Fraud".to_string(),
        labeled_by: "test_user".to_string()
    }.into();

    
    let response = label_transaction::<MockWebTransaction>(axum::extract::State(app_state), request_body).await;
    
    // Assert: Verify the response is as expected
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_label_transaction_multiple_transactions() {
    // Arrange: Set up the mock state with multiple transactions using constructor methods
    let web_storage = Arc::new(create_mock_web_storage(vec![
        MockWebTransaction::new(1),
        MockWebTransaction::new(2),
        MockWebTransaction::new(3),
    ]));
    let mut common_storage = create_mock_common_storage(Some(42), vec![]);
    common_storage.expect_label_transactions().returning(|_, _, _, _, _| {
        Ok(())
    });

    let app_state = AppState::new(web_storage.clone(), Arc::new(common_storage));
    // Act: Send a request to label multiple transactions (test each one individually)
    for tx_id in [1, 2, 3] {
        
        let request_body: axum::Json<LabelRequest> = LabelRequest {
            transaction_ids: vec!(tx_id),
            fraud_level: FraudLevel::Fraud,
            fraud_category: "Batch Test".to_string(),
            labeled_by: "test_user".to_string()
        }.into();
        
        let response = label_transaction::<MockWebTransaction>(axum::extract::State(app_state.clone()), request_body).await;
        
        // Assert: Verify the response is as expected
        assert_eq!(response.status(), StatusCode::OK);
    }
} 


#[tokio::test]
async fn test_label_transaction_save_label_error() {
    // Arrange: Set up the mock state with a storage that fails to save labels using constructor methods
    let web_storage = Arc::new(create_mock_web_storage(vec![
        MockWebTransaction::new(1)
    ]));
    let mut common_storage = create_mock_common_storage(None, vec![]);
    common_storage.expect_label_transactions().returning(|_, _, _, _, _| {
        Err("Failed to save label".into())
    });
    println!("common_storage: {:?}", &common_storage.label_transactions(&[1], FraudLevel::Fraud, "Test Fraud".to_string(), LabelSource::Manual, "test_user".to_string()).await);
    let app_state = AppState::new(web_storage, Arc::new(common_storage));


    // Act: Send a request to label a transaction
    let request_body: axum::Json<LabelRequest> = LabelRequest {
        transaction_ids: vec![1],
        fraud_level: FraudLevel::Fraud,
        fraud_category: "Test Fraud".to_string(),
        labeled_by: "test_user".to_string()
    }.into();
    
    let response = label_transaction::<MockWebTransaction>(axum::extract::State(app_state), request_body).await;
    
    // Assert: Verify the response is an internal server error
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn test_label_transaction_invalid_transaction_id() {
    // Arrange: Set up mock with valid transaction using constructor
    let web_storage = Arc::new(create_mock_web_storage(vec![
        MockWebTransaction::new(1)
    ]));
    let mut common_storage = create_mock_common_storage(Some(42), vec![]);
    common_storage.expect_label_transactions().returning(|_, _, _, _, _| {
        Ok(())
    });
    let app_state = AppState::new(web_storage, Arc::new(common_storage));

    
    // Act: Send a request with invalid transaction ID (non-existent transaction)
    let request_body: axum::Json<LabelRequest> = LabelRequest {
        transaction_ids: vec![999], // This transaction doesn't exist
        fraud_level: FraudLevel::Fraud,
        fraud_category: "Test Fraud".to_string(),
        labeled_by: "test_user".to_string()
    }.into();
    
    let response = label_transaction::<MockWebTransaction>(axum::extract::State(app_state), request_body).await;
    
    // Assert: The API doesn't validate transaction existence, so it returns success
    // The business logic processes the request and returns the label_id
    assert_eq!(response.status(), StatusCode::OK);
}
