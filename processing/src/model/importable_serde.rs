use async_trait::async_trait;
use serde::de::DeserializeOwned;

#[async_trait]
pub trait ImportableSerde: super::importable::Importable + DeserializeOwned {
    fn as_json(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;
    fn from_json(json: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>>;
}
