use crate::model::{
    Channel, ConnectedTransaction, DirectConnection, Feature, FraudLevel, Label, LabelingResult, LabelSource, MatchingField, ModelId, ScoringEvent,
    TriggeredRule,
};
use async_trait::async_trait;
use std::error::Error;
use chrono::Utc;

#[async_trait]
pub trait CommonStorage: Send + Sync {
    async fn save_features(
        &self,
        transaction_id: ModelId,
        simple_features: Option<&[Feature]>,
        graph_features: &[Feature],
    ) -> Result<(), Box<dyn Error + Send + Sync>>;

    async fn save_scores(
        &self,
        transaction_id: ModelId,
        channel_id: ModelId,
        total_score: i32,
        triggered_rules: &[TriggeredRule],
    ) -> Result<(), Box<dyn Error + Send + Sync>>;

    async fn get_features(
        &self,
        transaction_id: ModelId,
    ) -> Result<(Option<Vec<Feature>>, Vec<Feature>), Box<dyn Error + Send + Sync>>;
    
    async fn find_connected_transactions(
        &self,
        transaction_id: ModelId,
        max_depth: Option<i32>,
        limit_count: Option<i32>,
        min_created_at: Option<chrono::DateTime<chrono::Utc>>,
        max_created_at: Option<chrono::DateTime<chrono::Utc>>,
        min_confidence: Option<i32>
    ) -> Result<Vec<ConnectedTransaction>, Box<dyn Error + Send + Sync>>;
    
    async fn get_direct_connections(
        &self,
        transaction_id: ModelId
    ) -> Result<Vec<DirectConnection>, Box<dyn Error + Send + Sync>>;
    
    async fn save_matching_fields(
        &self,
        transaction_id: ModelId,
        matching_fields: &[MatchingField],
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
    
    async fn get_channels(
        &self,
        model_id: ModelId,
    ) -> Result<Vec<Channel>, Box<dyn Error + Send + Sync>>;
    
    async fn get_scoring_events(
        &self,
        transaction_id: ModelId,
    ) -> Result<Vec<ScoringEvent>, Box<dyn Error + Send + Sync>>;
    
    async fn get_triggered_rules(
        &self,
        scoring_event_id: ModelId,
    ) -> Result<Vec<TriggeredRule>, Box<dyn Error + Send + Sync>>;
    
    async fn save_label(
        &self,
        label: &Label,
    ) -> Result<ModelId, Box<dyn Error + Send + Sync>>;
    
    async fn get_label(
        &self,
        label_id: ModelId,
    ) -> Result<Label, Box<dyn Error + Send + Sync>>;
    
    async fn update_transaction_label(
        &self,
        transaction_id: ModelId,
        label_id: ModelId,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
    
    /// Label multiple transactions with the same label in a batch operation
    async fn label_transactions(
        &self,
        transaction_ids: &[ModelId],
        fraud_level: FraudLevel,
        fraud_category: String,
        labeled_by: String,
    ) -> Result<LabelingResult, Box<dyn Error + Send + Sync>> {
        // Create label object
        let label = Label {
            id: 0, // Will be filled by the database
            fraud_level,
            fraud_category,
            label_source: LabelSource::Manual,
            labeled_by,
            created_at: Utc::now(),
        };
        
        // Save the label and get its ID
        let label_id = self.save_label(&label).await?;
        
        let mut success_count = 0;
        let mut failed_transaction_ids = Vec::new();
        
        // Apply the label to each transaction ID in the batch
        for &transaction_id in transaction_ids {
            match self.update_transaction_label(transaction_id, label_id).await {
                Ok(_) => {
                    success_count += 1;
                    tracing::info!("Successfully labeled transaction {}: label_id={}", transaction_id, label_id);
                },
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        transaction_id = %transaction_id,
                        label_id = %label_id,
                        "Failed to update transaction with label"
                    );
                    failed_transaction_ids.push(transaction_id);
                }
            }
        }
        
        Ok(LabelingResult {
            label_id,
            success_count,
            failed_transaction_ids,
        })
    }
} 