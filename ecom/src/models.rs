use async_trait::async_trait;
use processing::model::*;
use processing::ui_model::ModelRegistry;
use serde::{Serialize, Deserialize};
use std::sync::OnceLock;
use crate::entities::{order, order_item, customer, billing_data};

// Define a static model registry
static MODEL_REGISTRY: OnceLock<ModelRegistry> = OnceLock::new();

/// Combined Order model with related data using SeaORM entities directly
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub order: order::Model,
    pub items: Vec<order_item::Model>,
    pub customer: customer::Model,
    pub billing: billing_data::Model,
}

impl WebTransaction for Order {
    fn id(&self) -> ModelId {
        self.order.id
    }
}

impl ModelRegistryProvider for Order {
    fn get_registry() -> &'static ModelRegistry {
        // TODO: Implement proper ModelRegistry creation once we understand the correct API
        // For now, create a minimal registry to satisfy the trait
        MODEL_REGISTRY.get_or_init(|| {
            ModelRegistry::default()
        })
    }
}

#[async_trait]
impl Processible for Order {
    fn extract_simple_features(
        &self,
    ) -> Vec<Feature> {
        let mut features = Vec::new();

        features.push(Feature {
            name: "amount".to_string(),
            value: Box::new(FeatureValue::Double(
                self.items.iter().map(|i| i.price as f64).sum(),
            )),
        });

        features.push(Feature {
            name: "amounts".to_string(),
            value: Box::new(FeatureValue::DoubleList(
                self.items.iter().map(|i| i.price as f64).collect(),
            )),
        });

        features.push(Feature {
            name: "categories".to_string(),
            value: Box::new(FeatureValue::StringList(
                self.items.iter().map(|i| i.category.clone()).collect(),
            )),
        });

        features.push(Feature {
            name: "created_at".to_string(),
            value: Box::new(FeatureValue::DateTime(
                chrono::DateTime::from_timestamp(self.order.created_at.and_utc().timestamp(), 0)
                    .unwrap_or_else(|| chrono::Utc::now())
            )),
        });

        features.push(Feature {
            name: "is_high_value".to_string(),
            value: Box::new(FeatureValue::Bool(
                self.items.iter().map(|i| i.price as f64).sum::<f64>() > 1000.0,
            )),
        });

        features
    }

    fn extract_graph_features(
        &self,
        connected_transactions: &[ConnectedTransaction],
        direct_connections: &[DirectConnection]
    ) -> Vec<Feature> {
        let mut features = Vec::new();
        // Add connection-related features
        features.push(Feature {
            name: "connected_transaction_count".to_string(),
            value: Box::new(FeatureValue::Int(connected_transactions.len() as i64)),
        });
        
        features.push(Feature {
            name: "direct_connection_count".to_string(),
            value: Box::new(FeatureValue::Int(direct_connections.len() as i64)),
        });

        features
    }

    fn tx_id(&self) -> ModelId {
        self.order.transaction_id
    }

    fn id(&self) -> ModelId {
        self.order.id
    }

    fn extract_matching_fields(&self) -> Vec<MatchingField> {
        let mut fields = Vec::new();

        // Add email matching field
        fields.push(MatchingField {
            matcher: "exact".to_string(),
            value: self.customer.email.clone(),
        });

        // Add customer name matching field
        fields.push(MatchingField {
            matcher: "exact".to_string(),
            value: self.customer.name.clone(),
        });

        // Add billing address matching field
        fields.push(MatchingField {
            matcher: "exact".to_string(),
            value: self.billing.billing_address.clone(),
        });

        // Add payment details matching field (for similar payment methods)
        fields.push(MatchingField {
            matcher: "exact".to_string(),
            value: self.billing.payment_details.clone(),
        });

        fields
    }
}