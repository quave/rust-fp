use async_trait::async_trait;
use serde::Serialize;
use crate::model::ModelId;

#[async_trait]
pub trait WebTransaction: Send + Sync + Serialize {
    fn id(&self) -> ModelId;
}


