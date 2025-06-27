use std::error::Error;

use processing::{
    model::{ScorerResult, TriggeredRule},
    storage::CommonStorage,
};
use common::test_helpers::truncate_processing_tables;
use serde_json::json;

use super::setup::{get_test_storage, create_test_transaction};

#[tokio::test]
#[serial_test::serial]
async fn test_save_scores() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (pool, storage) = get_test_storage().await?;
    
    // Clean up any existing test data
    truncate_processing_tables(&pool).await?;
    
    let transaction_id = create_test_transaction(&storage).await?;
    
    // First create a model
    let model_id = common::test_helpers::create_test_model(&pool, "Test Model").await?;
    
    // Then create a channel
    let channel_id = common::test_helpers::create_test_channel(&pool, "Test Channel", model_id).await?;
    
    // Create scores
    let scores = vec![
        ScorerResult {
            name: "fraud_score".to_string(),
            score: 85,
        },
        ScorerResult {
            name: "risk_score".to_string(),
            score: 65,
        },
    ];
    
    // Calculate total score
    let total_score = scores.iter().map(|s| s.score).sum();
    
    // Create a scoring rule for each score
    let mut triggered_rules = Vec::new();
    for score in &scores {
        // Create a scoring rule
        let rule_id = common::test_helpers::create_test_scoring_rule(
            &pool,
            model_id,
            &score.name,
            &format!("Test rule for {}", score.name),
            json!({"test": "rule"}),
            score.score
        ).await?;
        
        triggered_rules.push(TriggeredRule {
            id: 0,
            scoring_events_id: 0,
            rule_id: rule_id,
        });
    }
    
    // Save scores
    storage.save_scores(transaction_id, channel_id, total_score, &triggered_rules).await?;
    
    // Verify scores were saved by querying the database directly
    let (scoring_event_id, saved_transaction_id, saved_channel_id, saved_total_score) = 
        common::test_helpers::get_scoring_event_by_transaction(&pool, transaction_id).await?;
    
    assert_eq!(saved_transaction_id, transaction_id);
    assert_eq!(saved_channel_id, channel_id);
    assert_eq!(saved_total_score, total_score);
    
    // Check the triggered rules
    let saved_rules = common::test_helpers::get_triggered_rules_for_scoring_event(&pool, scoring_event_id).await?;
    
    assert_eq!(saved_rules.len(), 2);
    
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn test_save_scores_with_empty_list() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (pool, storage) = get_test_storage().await?;
    let transaction_id = create_test_transaction(&storage).await?;
    
    // First create a model
    let model_id = common::test_helpers::create_test_model(&pool, "Test Model Empty").await?;
    
    // Then create a channel
    let channel_id = common::test_helpers::create_test_channel(&pool, "Test Channel Empty", model_id).await?;
    
    let total_score = 0; // No scores, so total is 0

    // Save empty scores list
    storage.save_scores(transaction_id, channel_id, total_score, &[]).await?;

    // Verify a scoring event was created but no rules were saved
    let (scoring_event_id, _, _, _) = 
        common::test_helpers::get_scoring_event_by_transaction(&pool, transaction_id).await?;
    
    let rules_count = common::test_helpers::count_triggered_rules_for_scoring_event(&pool, scoring_event_id).await?;

    assert_eq!(rules_count, 0);

    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn test_save_scores_with_duplicate_names() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (pool, storage) = get_test_storage().await?;
    
    // Clean up any existing test data
    truncate_processing_tables(&pool).await?;
    
    let transaction_id = create_test_transaction(&storage).await?;
    
    // First create a model
    let model_id = common::test_helpers::create_test_model(&pool, "Test Model Duplicate").await?;
    
    // Then create a channel
    let channel_id = common::test_helpers::create_test_channel(&pool, "Test Channel Duplicate", model_id).await?;

    // Create scores with duplicate names
    let scores = vec![
        ScorerResult {
            name: "duplicate_score".to_string(),
            score: 50,
        },
        ScorerResult {
            name: "duplicate_score".to_string(),
            score: 75,
        },
    ];
    
    let total_score = scores.iter().map(|s| s.score).sum();
    
    // Create a scoring rule for each score (potentially with the same name)
    let mut triggered_rules = Vec::new();
    for (i, score) in scores.iter().enumerate() {
        // Create a scoring rule with a unique name (add index to make it unique)
        let rule_id = common::test_helpers::create_test_scoring_rule(
            &pool,
            model_id,
            &format!("{}{}", score.name, i), // Make name unique
            &format!("Test rule for {}", score.name),
            json!({"test": "rule"}),
            score.score
        ).await?;
        
        triggered_rules.push(TriggeredRule {
            id: 0,
            scoring_events_id: 0,
            rule_id: rule_id,
        });
    }

    // Save scores
    storage.save_scores(transaction_id, channel_id, total_score, &triggered_rules).await?;

    // Verify both scores were saved
    let (scoring_event_id, _, _, _) = 
        common::test_helpers::get_scoring_event_by_transaction(&pool, transaction_id).await?;
    
    let saved_rules = common::test_helpers::get_triggered_rules_for_scoring_event(&pool, scoring_event_id).await?;

    assert_eq!(saved_rules.len(), 2);

    Ok(())
} 