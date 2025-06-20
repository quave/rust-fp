use std::error::Error;

use async_trait::async_trait;

use crate::model::Processible;

// Queue service interface
#[async_trait]
pub trait QueueService<T: Processible>: Send + Sync {
    fn new() -> Self;
    async fn enqueue(&self, id: &T::Id) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn dequeue(&self) -> Result<Option<T::Id>, Box<dyn Error + Send + Sync>>;
}

pub trait QueueServiceFactory<T: Processible> {
    fn new() -> Self;
}
