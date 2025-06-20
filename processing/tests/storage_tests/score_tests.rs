use std::error::Error;

use processing::{
    model::{ScorerResult, TriggeredRule},
    storage::CommonStorage,
};
use serde_json::json;

use super::setup::get_test_storage;

#[tokio::test(flavor = "multi_thread")]
#[serial_test::serial]
async fn test_save_scores() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (pool, storage) = get_test_storage().await?;
    let transaction_id = storage.insert_transaction().await?;
    
    // First create a model
    let model_id = sqlx::query!(
        r#"
        INSERT INTO models (name, features_schema_version_major, features_schema_version_minor)
        VALUES ('Test Model', 1, 0)
        RETURNING id
        "#
    )
    .fetch_one(&pool)
    .await?
    .id;
    
    // Then create a channel
    let channel_id = sqlx::query!(
        r#"
        INSERT INTO channels (name, model_id)
        VALUES ('Test Channel', $1)
        RETURNING id
        "#,
        model_id
    )
    .fetch_one(&pool)
    .await?
    .id;
    
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
        let rule_id = sqlx::query!(
            r#"
            INSERT INTO scoring_rules (model_id, name, description, rule, score)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id
            "#,
            model_id,
            score.name,
            format!("Test rule for {}", score.name),
            json!({"test": "rule"}),
            score.score
        )
        .fetch_one(&pool)
        .await?
        .id;
        
        triggered_rules.push(TriggeredRule {
            id: 0,
            scoring_events_id: 0,
            rule_id: rule_id,
        });
    }
    
    // Save scores
    storage.save_scores(transaction_id, channel_id, total_score, &triggered_rules).await?;
    
    // Verify scores were saved by querying the database directly
    let saved_scoring_event = sqlx::query!(
        r#"
        SELECT id, transaction_id, channel_id, total_score
        FROM scoring_events
        WHERE transaction_id = $1
        "#,
        transaction_id
    )
    .fetch_one(&pool)
    .await?;
    
    assert_eq!(saved_scoring_event.transaction_id, transaction_id);
    assert_eq!(saved_scoring_event.channel_id, channel_id);
    assert_eq!(saved_scoring_event.total_score, total_score);
    
    // Check the triggered rules
    let saved_rules = sqlx::query!(
        r#"
        SELECT rule_id
        FROM triggered_rules
        WHERE scoring_events_id = $1
        "#,
        saved_scoring_event.id
    )
    .fetch_all(&pool)
    .await?;
    
    assert_eq!(saved_rules.len(), 2);
    
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[serial_test::serial]
async fn test_save_scores_with_empty_list() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (pool, storage) = get_test_storage().await?;
    let transaction_id = storage.insert_transaction().await?;
    
    // First create a model
    let model_id = sqlx::query!(
        r#"
        INSERT INTO models (name, features_schema_version_major, features_schema_version_minor)
        VALUES ('Test Model Empty', 1, 0)
        RETURNING id
        "#
    )
    .fetch_one(&pool)
    .await?
    .id;
    
    // Then create a channel
    let channel_id = sqlx::query!(
        r#"
        INSERT INTO channels (name, model_id)
        VALUES ('Test Channel Empty', $1)
        RETURNING id
        "#,
        model_id
    )
    .fetch_one(&pool)
    .await?
    .id;
    
    let total_score = 0; // No scores, so total is 0

    // Save empty scores list
    storage.save_scores(transaction_id, channel_id, total_score, &[]).await?;

    // Verify a scoring event was created but no rules were saved
    let saved_scoring_event = sqlx::query!(
        r#"
        SELECT id
        FROM scoring_events
        WHERE transaction_id = $1
        "#,
        transaction_id
    )
    .fetch_one(&pool)
    .await?;
    
    let saved_rules = sqlx::query!(
        r#"
        SELECT COUNT(*) as count
        FROM triggered_rules
        WHERE scoring_events_id = $1
        "#,
        saved_scoring_event.id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(saved_rules.count.unwrap_or(0), 0);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[serial_test::serial]
async fn test_save_scores_with_duplicate_names() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (pool, storage) = get_test_storage().await?;
    let transaction_id = storage.insert_transaction().await?;
    
    // First create a model
    let model_id = sqlx::query!(
        r#"
        INSERT INTO models (name, features_schema_version_major, features_schema_version_minor)
        VALUES ('Test Model Duplicate', 1, 0)
        RETURNING id
        "#
    )
    .fetch_one(&pool)
    .await?
    .id;
    
    // Then create a channel
    let channel_id = sqlx::query!(
        r#"
        INSERT INTO channels (name, model_id)
        VALUES ('Test Channel Duplicate', $1)
        RETURNING id
        "#,
        model_id
    )
    .fetch_one(&pool)
    .await?
    .id;

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
        let rule_id = sqlx::query!(
            r#"
            INSERT INTO scoring_rules (model_id, name, description, rule, score)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id
            "#,
            model_id,
            format!("{}{}", score.name, i), // Make name unique
            format!("Test rule for {}", score.name),
            json!({"test": "rule"}),
            score.score
        )
        .fetch_one(&pool)
        .await?
        .id;
        
        triggered_rules.push(TriggeredRule {
            id: 0,
            scoring_events_id: 0,
            rule_id: rule_id,
        });
    }

    // Save scores
    storage.save_scores(transaction_id, channel_id, total_score, &triggered_rules).await?;

    // Verify both scores were saved
    let saved_scoring_event = sqlx::query!(
        r#"
        SELECT id
        FROM scoring_events
        WHERE transaction_id = $1
        "#,
        transaction_id
    )
    .fetch_one(&pool)
    .await?;
    
    let saved_rules = sqlx::query!(
        r#"
        SELECT rule_id
        FROM triggered_rules
        WHERE scoring_events_id = $1
        "#,
        saved_scoring_event.id
    )
    .fetch_all(&pool)
    .await?;

    assert_eq!(saved_rules.len(), 2);

    Ok(())
} 