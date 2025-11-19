use async_trait::async_trait;
use processing::model::{ConnectedTransaction, DirectConnection, Feature, FeatureValue, MatchingField, Processible};

use crate::model::{EcomOrder};

#[async_trait]
impl Processible for EcomOrder {
    fn validate(&self) -> Result<(), String> {
        Ok(())
    }

    fn payload_number(&self) -> String {
        self.order_number.clone()
    }

    fn schema_version(&self) -> (i32, i32) {
        (1, 0)
    }

    fn extract_simple_features(
        &self,
    ) -> Vec<Feature> {
        let mut features = Vec::new();

        // Count of items
        features.push(Feature {
            name: "item_count".to_string(),
            value: Box::new(FeatureValue::Int(self.items.len() as i64)),
        });

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
                chrono::DateTime::from_timestamp(self.created_at.timestamp(), 0)
                    .unwrap_or_else(|| chrono::Utc::now())
            )),
        });

        // Derived order time as "HH:MM:SS" string for rule comparisons
        let order_time_str = self.created_at.format("%H:%M:%S").to_string();
        features.push(Feature {
            name: "order_time".to_string(),
            value: Box::new(FeatureValue::String(order_time_str)),
        });

        // Placeholder for new-customer logic; set to false for now
        features.push(Feature {
            name: "is_new_customer".to_string(),
            value: Box::new(FeatureValue::Bool(false)),
        });

        // Best-effort country code; default to "US"
        features.push(Feature {
            name: "country_code".to_string(),
            value: Box::new(FeatureValue::String("US".to_string())),
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
