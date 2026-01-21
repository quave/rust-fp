use std::error::Error;

use common::test_helpers::{create_test_transaction, truncate_processing_tables};
use processing::model::sea_orm_storage_model as entities;
use processing::storage::CommonStorage;
use sea_orm::ActiveValue::NotSet;
use sea_orm::entity::prelude::*;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

use super::setup::get_test_storage;

/// Count triggered rules for a scoring event (SeaORM)
async fn count_triggered_rules_for_scoring_event(
    db: &DatabaseConnection,
    scoring_event_id: i64,
) -> Result<i64, Box<dyn Error + Send + Sync>> {
    let count = entities::triggered_rule::Entity::find()
        .filter(entities::triggered_rule::Column::ScoringEventsId.eq(scoring_event_id))
        .count(db)
        .await? as i64;
    Ok(count)
}

/// Get scoring event by transaction ID (SeaORM)
async fn get_scoring_event_by_transaction(
    db: &DatabaseConnection,
    transaction_id: i64,
) -> Result<(i64, i64, i64, i32), Box<dyn Error + Send + Sync>> {
    let row = entities::scoring_event::Entity::find()
        .filter(entities::scoring_event::Column::TransactionId.eq(transaction_id))
        .one(db)
        .await?
        .ok_or("scoring_event not found")?;
    Ok((
        row.id,
        row.transaction_id,
        row.activation_id,
        row.total_score,
    ))
}

/// Get triggered rules for a scoring event (SeaORM)
async fn get_triggered_rules_for_scoring_event(
    db: &DatabaseConnection,
    scoring_event_id: i64,
) -> Result<Vec<i64>, Box<dyn Error + Send + Sync>> {
    let rows = entities::triggered_rule::Entity::find()
        .filter(entities::triggered_rule::Column::ScoringEventsId.eq(scoring_event_id))
        .all(db)
        .await?;
    Ok(rows.into_iter().map(|row| row.rule_id).collect())
}

/// Create a test scoring rule (SeaORM)
async fn create_test_scoring_rule(
    db: &DatabaseConnection,
    model_id: i64,
    name: &str,
    description: &str,
    rule: &str,
    score: i32,
) -> Result<i64, Box<dyn Error + Send + Sync>> {
    let am = entities::expression_rule::ActiveModel {
        id: NotSet,
        model_id: Set(model_id),
        name: Set(name.to_string()),
        description: Set(Some(description.to_string())),
        rule: Set(rule.to_string()),
        score: Set(score),
        created_at: NotSet,
    };
    let inserted = am.insert(db).await?;
    Ok(inserted.id)
}

/// Create a test model and return its ID (SeaORM)
async fn create_test_model(
    db: &DatabaseConnection,
    name: &str,
) -> Result<i64, Box<dyn Error + Send + Sync>> {
    let am = entities::scoring_model::ActiveModel {
        id: NotSet,
        name: Set(name.to_string()),
        features_schema_version_major: Set(1),
        features_schema_version_minor: Set(0),
        version: NotSet,
        model_type: NotSet,
        created_at: NotSet,
    };
    let inserted = am.insert(db).await?;
    Ok(inserted.id)
}

/// Create a test channel and return its ID (SeaORM)
async fn create_test_channel(
    db: &DatabaseConnection,
    name: &str,
    model_id: i64,
) -> Result<i64, Box<dyn Error + Send + Sync>> {
    let am = entities::channel::ActiveModel {
        id: NotSet,
        name: Set(name.to_string()),
        model_id: Set(model_id),
        created_at: NotSet,
    };
    let inserted = am.insert(db).await?;
    Ok(inserted.id)
}

#[tokio::test]
#[serial_test::serial]
async fn test_save_scores() -> Result<(), Box<dyn Error + Send + Sync>> {
    let storage = get_test_storage().await?;
    let db = &storage.db;

    // Clean up any existing test data
    truncate_processing_tables(db).await?;

    let transaction_id = create_test_transaction(&storage.db).await?;

    // First create a model
    let model_id = create_test_model(db, "Test Model").await?;

    // Then create a channel
    let channel_id = create_test_channel(db, "Test Channel", model_id).await?;

    // Define scores (name, score)
    let score_defs = vec![
        ("fraud_score".to_string(), 85),
        ("risk_score".to_string(), 65),
    ];
    let total_score: i32 = score_defs.iter().map(|(_, s)| *s).sum();
    // Create rules and collect their IDs
    let mut triggered_rule_ids: Vec<i64> = Vec::new();
    for (name, score) in &score_defs {
        let rule_id = create_test_scoring_rule(
            db,
            model_id,
            name,
            &format!("Test rule for {}", name),
            "account_number == 1234567890",
            *score,
        )
        .await?;
        triggered_rule_ids.push(rule_id);
    }
    // Create activation for (channel_id, model_id)
    let activation_id = entities::channel_model_activation::ActiveModel {
        id: NotSet,
        channel_id: Set(channel_id),
        model_id: Set(model_id),
        created_at: NotSet,
    }
    .insert(db)
    .await?
    .id;

    // Save scores
    storage
        .save_scores(
            transaction_id,
            activation_id,
            total_score,
            &triggered_rule_ids,
        )
        .await?;

    // Verify scores were saved by querying the database directly
    let (scoring_event_id, saved_transaction_id, saved_activation_id, saved_total_score) =
        get_scoring_event_by_transaction(db, transaction_id).await?;

    assert_eq!(saved_transaction_id, transaction_id);
    // Verify activation points to the same channel
    let activation = entities::channel_model_activation::Entity::find_by_id(saved_activation_id)
        .one(db)
        .await?
        .expect("activation not found");
    assert_eq!(activation.channel_id, channel_id);
    assert_eq!(saved_total_score, total_score);

    // Check the triggered rules
    let saved_rules = get_triggered_rules_for_scoring_event(db, scoring_event_id).await?;

    assert_eq!(saved_rules.len(), 2);

    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn test_save_scores_with_empty_list() -> Result<(), Box<dyn Error + Send + Sync>> {
    let storage = get_test_storage().await?;
    let db = &storage.db;
    let transaction_id = create_test_transaction(&storage.db).await?;

    // First create a model
    let model_id = create_test_model(db, "Test Model Empty").await?;

    // Then create a channel
    let channel_id = create_test_channel(db, "Test Channel Empty", model_id).await?;

    let total_score = 0; // No scores, so total is 0

    // Create activation for (channel_id, model_id)
    let activation_id = entities::channel_model_activation::ActiveModel {
        id: NotSet,
        channel_id: Set(channel_id),
        model_id: Set(model_id),
        created_at: NotSet,
    }
    .insert(db)
    .await?
    .id;

    // Save empty scores list
    storage
        .save_scores(transaction_id, activation_id, total_score, &[])
        .await?;

    // Verify a scoring event was created but no rules were saved
    let (scoring_event_id, _, _, _) = get_scoring_event_by_transaction(db, transaction_id).await?;

    let rules_count = count_triggered_rules_for_scoring_event(db, scoring_event_id).await?;

    assert_eq!(rules_count, 0);

    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn test_save_scores_with_duplicate_names() -> Result<(), Box<dyn Error + Send + Sync>> {
    let storage = get_test_storage().await?;
    let db = &storage.db;

    // Clean up any existing test data
    truncate_processing_tables(db).await?;

    let transaction_id = create_test_transaction(&storage.db).await?;

    // First create a model
    let model_id = create_test_model(db, "Test Model Duplicate").await?;

    // Then create a channel
    let channel_id = create_test_channel(db, "Test Channel Duplicate", model_id).await?;

    // Create scores with duplicate names (unique by index)
    let score_defs = vec![
        ("duplicate_score".to_string(), 50),
        ("duplicate_score".to_string(), 75),
    ];
    let total_score: i32 = score_defs.iter().map(|(_, s)| *s).sum();
    // Create rules and collect IDs
    let mut triggered_rule_ids: Vec<i64> = Vec::new();
    for (i, (name, score)) in score_defs.iter().enumerate() {
        let unique_name = format!("{}{}", name, i);
        let rule_id = create_test_scoring_rule(
            db,
            model_id,
            &unique_name,
            &format!("Test rule for {}", name),
            "account_number == 1234567890",
            *score,
        )
        .await?;
        triggered_rule_ids.push(rule_id);
    }
    // Create activation for (channel_id, model_id)
    let activation_id = entities::channel_model_activation::ActiveModel {
        id: NotSet,
        channel_id: Set(channel_id),
        model_id: Set(model_id),
        created_at: NotSet,
    }
    .insert(db)
    .await?
    .id;

    // Save scores
    storage
        .save_scores(
            transaction_id,
            activation_id,
            total_score,
            &triggered_rule_ids,
        )
        .await?;

    // Verify both scores were saved
    let (scoring_event_id, _, _, _) = get_scoring_event_by_transaction(db, transaction_id).await?;

    let saved_rules = get_triggered_rules_for_scoring_event(db, scoring_event_id).await?;

    assert_eq!(saved_rules.len(), 2);

    Ok(())
}
