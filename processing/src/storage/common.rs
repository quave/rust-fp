use crate::model::{
    Feature, mongo_model::{ScoringChannel, ScoringEvent, Transaction}, processible::{ColumnValueTrait, Filter}, *
};
use async_trait::async_trait;
use tracing::debug;
use std::error::Error;
use jsonschema::validate;
use serde_json::{Value, json};


#[async_trait]
pub trait CommonStorage<ID: Send + Sync + PartialEq>: Send + Sync {
    fn validate_features(
        &self,
        features: &[Feature],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let features_json = serde_json::to_value(features)?;
        debug!(
            "Raw features JSON string: {}",
            serde_json::to_string(&features_json)?
        );
        let validation_result = validate(&self.get_features_schema(), &features_json);
        if let Err(errors) = validation_result {
            debug!("Validation error details: {:?}", errors);
            return Err(format!("Feature validation failed: {:?}", errors).into());
        }
        Ok(())
    }

    async fn insert_imported_transaction(
        &self,
        payload_number: String,
        payload: serde_json::Value,
        schema_version: (i32, i32),
    ) -> Result<ID, Box<dyn Error + Send + Sync>>;

    async fn get_transaction(
        &self,
        transaction_id: ID,
    ) -> Result<Transaction, Box<dyn Error + Send + Sync>>;

    async fn filter_transactions(
        &self,
        filters: &[Filter<Box<dyn ColumnValueTrait>>],
    ) -> Result<Vec<Transaction>, Box<dyn Error + Send + Sync>>;

    async fn mark_transaction_processed(
        &self,
        transaction_id: ID,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;

    async fn save_features<'a>(
        &self,
        transaction_id: ID,
        simple_features: &'a Option<&'a [Feature]>,
        graph_features: &'a [Feature],
    ) -> Result<(), Box<dyn Error + Send + Sync>>;

    async fn get_active_model_activations(
        &self,
    ) -> Result<Vec<ScoringChannel>, Box<dyn Error + Send + Sync>>;

    async fn save_scores(
        &self,
        transaction_id: ID,
        channel: ScoringChannel,
        scoring_result: Box<dyn ScoringResult>,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;

    async fn find_connected_transactions(
        &self,
        payload_number: &str,
        max_depth: Option<i32>,
        limit_count: Option<i32>,
        filter_config: Option<serde_json::Value>,
        min_confidence: Option<i32>,
    ) -> Result<Vec<ConnectedTransaction>, Box<dyn Error + Send + Sync>>;

    async fn get_direct_connections(
        &self,
        payload_number: &str,
    ) -> Result<Vec<DirectConnection>, Box<dyn Error + Send + Sync>>;

    async fn save_matching_fields_with_timespace(
        &self,
        transaction_id: &ID,
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
        transaction_id: &ID,
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

    async fn get_scoring_events(
        &self,
        transaction_id: ID,
    ) -> Result<Vec<ScoringEvent>, Box<dyn Error + Send + Sync>>;

    async fn label_transactions(
        &self,
        payload_numbers: &[String],
        fraud_level: &FraudLevel,
        fraud_category: &String,
        label_source: &LabelSource,
        labeled_by: &String,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;

    fn default_matcher_config(&self) -> MatcherConfig {
        (80, 50)
    }
    
    fn get_features_schema(&self) -> Value {
        // Create a schema that matches all test cases
        json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                    },
                    "type": {
                        "type": "string",
                        "enum": [
                            "integer",
                            "double",
                            "string",
                            "boolean",
                            "datetime",
                            "integer_array",
                            "double_array",
                            "string_array",
                            "boolean_array"
                        ]
                    },
                    "value": {
                        "type": ["number", "string", "boolean", "array"]
                    }
                },
                "required": ["name", "type", "value"],
                "dependencies": {
                    "type": {
                        "oneOf": [
                            {
                                "properties": {
                                    "type": { "const": "integer" },
                                    "value": { "type": "number" }
                                }
                            },
                            {
                                "properties": {
                                    "type": { "const": "double" },
                                    "value": { "type": "number" }
                                }
                            },
                            {
                                "properties": {
                                    "type": { "const": "string" },
                                    "value": { "type": "string" }
                                }
                            },
                            {
                                "properties": {
                                    "type": { "const": "boolean" },
                                    "value": { "type": "boolean" }
                                }
                            },
                            {
                                "properties": {
                                    "type": { "const": "datetime" },
                                    "value": { "type": "string", "format": "date-time" }
                                }
                            },
                            {
                                "properties": {
                                    "type": { "const": "integer_array" },
                                    "value": { "type": "array", "items": { "type": "number" } }
                                }
                            },
                            {
                                "properties": {
                                    "type": { "const": "double_array" },
                                    "value": { "type": "array", "items": { "type": "number" } }
                                }
                            },
                            {
                                "properties": {
                                    "type": { "const": "string_array" },
                                    "value": { "type": "array", "items": { "type": "string" } }
                                }
                            },
                            {
                                "properties": {
                                    "type": { "const": "boolean_array" },
                                    "value": { "type": "array", "items": { "type": "boolean" } }
                                }
                            }
                        ]
                    }
                }
            }
        })
    }

}
