use async_trait::async_trait;
use mongodb::bson::oid::ObjectId;
use processing::model::{
    ConnectedTransaction, DirectConnection, Feature, FeatureValue, MatchingField, Processible,
};

use crate::model::EcomF2Order;

#[async_trait]
impl Processible for EcomF2Order {
    type Id = ObjectId;

    fn validate(&self) -> Result<(), String> {
        Ok(())
    }

    fn payload_number(&self) -> String {
        self.id.to_string()
    }

    fn schema_version(&self) -> (i32, i32) {
        EcomF2Order::schema_version()
    }

    fn extract_simple_features(&self) -> Vec<Feature> {
        let mut features = Vec::new();

        // Count of items
        features.push(Feature {
            name: "item_count".to_string(),
            value: Box::new(FeatureValue::Int(self.item_iter().count() as i64)),
        });

        let total_amount = self.total_amount();
        features.push(Feature {
            name: "amount".to_string(),
            value: Box::new(FeatureValue::Double(total_amount)),
        });

        features.push(Feature {
            name: "amounts".to_string(),
            value: Box::new(FeatureValue::DoubleList(self.item_prices())),
        });

        features.push(Feature {
            name: "categories".to_string(),
            value: Box::new(FeatureValue::StringList(self.item_categories())),
        });

        features.push(Feature {
            name: "created_at".to_string(),
            value: Box::new(FeatureValue::DateTime(self.created())),
        });

        // Derived order time as "HH:MM:SS" string for rule comparisons
        let order_time_str = self.created().format("%H:%M:%S").to_string();
        features.push(Feature {
            name: "order_time".to_string(),
            value: Box::new(FeatureValue::String(order_time_str)),
        });

        let is_new_customer = self
            .customer_account
            .as_ref()
            .and_then(|account| account.created_date)
            .map(|created_at| self.created().signed_duration_since(created_at).num_days() < 30)
            .unwrap_or(true);
        features.push(Feature {
            name: "is_new_customer".to_string(),
            value: Box::new(FeatureValue::Bool(is_new_customer)),
        });

        features.push(Feature {
            name: "is_high_value".to_string(),
            value: Box::new(FeatureValue::Bool(total_amount > 1000.0)),
        });

        features
    }

    fn extract_graph_features(
        &self,
        connected_transactions: &[ConnectedTransaction<Self::Id>],
        direct_connections: &[DirectConnection<Self::Id>],
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
        if let Some(email) = self.customer_email() {
            fields.push(MatchingField::new_simple(
                "customer.email".to_string(),
                email,
            ));
        }

        // Add customer name matching field
        if let Some(name) = self.customer_full_name() {
            fields.push(MatchingField::new_simple("customer.name".to_string(), name));
        }

        if let Some(token) = self.device_ident_token() {
            fields.push(MatchingField::new_simple(
                "deviceIdent.token".to_string(),
                token,
            ));
        }

        fields
    }
}
