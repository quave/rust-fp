use async_trait::async_trait;

#[async_trait]
pub trait Importable: Send + Sync {
    fn validate(&self) -> Result<(), String>;
}


