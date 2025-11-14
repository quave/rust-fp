use std::error::Error;

use processing::{
    model::{ScorerResult, TriggeredRule},
    storage::CommonStorage,
};
use common::test_helpers::{truncate_processing_tables, create_test_transaction};
use serde_json::json;
use sqlx::PgPool;
use serde_json::Value;


use super::setup::get_test_storage;

/// Count triggered rules for a scoring event
async fn count_triggered_rules_for_scoring_event(pool: &PgPool, scoring_event_id: i64) -> Result<i64, Box<dyn Error + Send + Sync>> {
    let row = sqlx::query!("SELECT COUNT(*) as count FROM triggered_rules WHERE scoring_events_id = $1", scoring_event_id)
        .fetch_one(pool).await?;
    Ok(row.count.unwrap_or(0))
}

/// Get scoring event by transaction ID
async fn get_scoring_event_by_transaction(pool: &PgPool, transaction_id: i64) -> Result<(i64, i64, i64, i32), Box<dyn Error + Send + Sync>> {
    let row = sqlx::query!("SELECT id, transaction_id, channel_id, total_score FROM scoring_events WHERE transaction_id = $1", transaction_id)
        .fetch_one(pool).await?;
    Ok((row.id, row.transaction_id, row.channel_id, row.total_score))
}

/// Get triggered rules for a scoring event
async fn get_triggered_rules_for_scoring_event(pool: &PgPool, scoring_event_id: i64) -> Result<Vec<i64>, Box<dyn Error + Send + Sync>> {
    let rows = sqlx::query!("SELECT rule_id FROM triggered_rules WHERE scoring_events_id = $1", scoring_event_id)
        .fetch_all(pool).await?;
    Ok(rows.into_iter().map(|row| row.rule_id).collect())
}

/// Create a test scoring rule (complex case that needs manual implementation)
async fn create_test_scoring_rule(
    pool: &PgPool, 
    model_id: i64, 
    name: &str, 
    description: &str, 
    rule: Value, 
    score: i32
) -> Result<i64, Box<dyn Error + Send + Sync>> {
    let row = sqlx::query!(
        "INSERT INTO scoring_rules (model_id, name, description, rule, score) VALUES ($1, $2, $3, $4, $5) RETURNING id",
        model_id, name, description, rule, score
    ).fetch_one(pool).await?;
    Ok(row.id)
}

/// Create a test model and return its ID  
async fn create_test_model(pool: &PgPool, name: &str) -> Result<i64, Box<dyn Error + Send + Sync>> {
    let row = sqlx::query!("INSERT INTO scoring_models (name, features_schema_version_major, features_schema_version_minor) VALUES ($1, 1, 0) RETURNING id", name)
        .fetch_one(pool).await?;
    Ok(row.id)
}

/// Create a test channel and return its ID
async fn create_test_channel(pool: &PgPool, name: &str, model_id: i64) -> Result<i64, Box<dyn Error + Send + Sync>> {
    let row = sqlx::query!("INSERT INTO channels (name, model_id) VALUES ($1, $2) RETURNING id", name, model_id)
        .fetch_one(pool).await?;
    Ok(row.id)
}

#[tokio::test]
#[serial_test::serial]
async fn test_save_scores() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (pool, storage) = get_test_storage().await?;
    
    // Clean up any existing test data
    truncate_processing_tables(&pool).await?;
    
    let transaction_id = create_test_transaction(&pool).await?;
    
    // First create a model
    let model_id = create_test_model(&pool, "Test Model").await?;
    
    // Then create a channel
    let channel_id = create_test_channel(&pool, "Test Channel", model_id).await?;
    
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
        let rule_id = create_test_scoring_rule(
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
        get_scoring_event_by_transaction(&pool, transaction_id).await?;
    
    assert_eq!(saved_transaction_id, transaction_id);
    assert_eq!(saved_channel_id, channel_id);
    assert_eq!(saved_total_score, total_score);
    
    // Check the triggered rules
    let saved_rules = get_triggered_rules_for_scoring_event(&pool, scoring_event_id).await?;
    
    assert_eq!(saved_rules.len(), 2);
    
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn test_save_scores_with_empty_list() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (pool, storage) = get_test_storage().await?;
    let transaction_id = create_test_transaction(&pool).await?;
    
    // First create a model
    let model_id = create_test_model(&pool, "Test Model Empty").await?;
    
    // Then create a channel
    let channel_id = create_test_channel(&pool, "Test Channel Empty", model_id).await?;
    
    let total_score = 0; // No scores, so total is 0

    // Save empty scores list
    storage.save_scores(transaction_id, channel_id, total_score, &[]).await?;

    // Verify a scoring event was created but no rules were saved
    let (scoring_event_id, _, _, _) = get_scoring_event_by_transaction(&pool, transaction_id).await?;
    
    let rules_count = count_triggered_rules_for_scoring_event(&pool, scoring_event_id).await?;

    assert_eq!(rules_count, 0);

    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn test_save_scores_with_duplicate_names() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (pool, storage) = get_test_storage().await?;
    
    // Clean up any existing test data
    truncate_processing_tables(&pool).await?;
    
    let transaction_id = create_test_transaction(&pool).await?;
    
    // First create a model
    let model_id = create_test_model(&pool, "Test Model Duplicate").await?;
    
    // Then create a channel
    let channel_id = create_test_channel(&pool, "Test Channel Duplicate", model_id).await?;

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
        let rule_id = create_test_scoring_rule(
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
        get_scoring_event_by_transaction(&pool, transaction_id).await?;
    
    let saved_rules = get_triggered_rules_for_scoring_event(&pool, scoring_event_id).await?;

    assert_eq!(saved_rules.len(), 2);

    Ok(())
} 