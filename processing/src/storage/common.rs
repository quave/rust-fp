use crate::model::{Channel, ScoringEvent, TriggeredRule};
use crate::model::{
    Feature,
    expression_rule::Model as ExpressionRule,
    processible::{ColumnValueTrait, Filter},
    *,
};
use async_trait::async_trait;
use std::error::Error;

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

    async fn get_latest_transaction_by_payload(
        &self,
        payload_number: &str,
    ) -> Result<Option<Transaction>, Box<dyn Error + Send + Sync>>;

    async fn list_transaction_versions(
        &self,
        payload_number: &str,
    ) -> Result<Vec<Transaction>, Box<dyn Error + Send + Sync>>;

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
        simple_features: &'a Option<&'a [Feature]>,
        graph_features: &'a [Feature],
    ) -> Result<(), Box<dyn Error + Send + Sync>>;

    async fn get_activation_by_channel_id(
        &self,
        channel_id: ModelId,
    ) -> Result<channel_model_activation::Model, Box<dyn Error + Send + Sync>>;

    async fn get_channel_by_name(
        &self,
        name: &str,
    ) -> Result<Option<Channel>, Box<dyn Error + Send + Sync>>;

    async fn get_expression_rules(
        &self,
        channel_id: ModelId,
    ) -> Result<Vec<ExpressionRule>, Box<dyn Error + Send + Sync>>;

    async fn save_scores(
        &self,
        transaction_id: ModelId,
        activation_id: ModelId,
        total_score: i32,
        triggered_rules: &[ModelId],
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
        filter_config: Option<serde_json::Value>,
        min_confidence: Option<i32>,
    ) -> Result<Vec<ConnectedTransaction>, Box<dyn Error + Send + Sync>>;

    async fn get_direct_connections(
        &self,
        transaction_id: ModelId,
    ) -> Result<Vec<DirectConnection>, Box<dyn Error + Send + Sync>>;

    async fn save_matching_fields_with_timespace(
        &self,
        transaction_id: ModelId,
        matching_fields: &[MatchingField],
        datetime_alpha: Option<chrono::DateTime<chrono::Utc>>,
        datetime_beta: Option<chrono::DateTime<chrono::Utc>>,
        long_alpha: Option<f64>,
        lat_alpha: Option<f64>,
        long_beta: Option<f64>,
        lat_beta: Option<f64>,
        long_gamma: Option<f64>,
        lat_gamma: Option<f64>,
        long_delta: Option<f64>,
        lat_delta: Option<f64>,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;

    async fn save_matching_fields(
        &self,
        transaction_id: ModelId,
        matching_fields: &[MatchingField],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.save_matching_fields_with_timespace(
            transaction_id,
            matching_fields,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
    }

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
        fraud_level: &FraudLevel,
        fraud_category: &String,
        label_source: &LabelSource,
        labeled_by: &String,
    ) -> Result<Vec<ModelId>, Box<dyn Error + Send + Sync>>;
}
