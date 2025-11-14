use crate::model::{Feature, processible::{ColumnValueTrait, Filter}, *};
use async_trait::async_trait;
use std::error::Error;
use crate::model::{Channel, ScoringEvent, TriggeredRule};

#[async_trait]
pub trait CommonStorage: Send + Sync {
    async fn insert_transaction(
        &self,
        payload_number: String,
        payload: serde_json::Value,
        schema_version: (i32, i32),
    ) -> Result<ModelId, Box<dyn Error + Send + Sync>>;

    async fn get_transaction(
        &self,
        transaction_id: ModelId,
    ) -> Result<Transaction, Box<dyn Error + Send + Sync>>;

    async fn filter_transactions(
        &self,
        filters: &[Filter<Box<dyn ColumnValueTrait>>],
    ) -> Result<Vec<Transaction>, Box<dyn Error + Send + Sync>>;

    async fn mark_transaction_processed(
        &self,
        transaction_id: ModelId,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;

    async fn save_features<'a>(
        &self,
        transaction_id: ModelId,
        simple_features: &'a Option<&'a[Feature]>,
        graph_features: &'a [Feature],
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
        
    async fn label_transactions(
        &self,
        transaction_ids: &[ModelId],
        fraud_level: FraudLevel,
        fraud_category: String,
        label_source: LabelSource,
        labeled_by: String,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
} 