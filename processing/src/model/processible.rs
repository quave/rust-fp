use async_trait::async_trait;
use crate::model::{ModelId, Feature, ConnectedTransaction, DirectConnection, MatchingField};

#[async_trait]
pub trait Processible: Send + Sync {
    fn id(&self) -> ModelId;
    fn extract_simple_features(&self) -> Vec<Feature>;
    fn extract_graph_features(
        &self,
        connected_transactions: &[ConnectedTransaction],
        direct_connections: &[DirectConnection],
    ) -> Vec<Feature>;
    fn extract_matching_fields(&self) -> Vec<MatchingField>;
}


