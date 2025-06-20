use crate::queue_service::QueueService;
use async_trait::async_trait;
use std::collections::VecDeque;
use std::error::Error;
use std::marker::PhantomData;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::model::Processible;

// In-memory implementation
pub struct InMemoryQueue<P: Processible> {
    queue: Arc<Mutex<VecDeque<String>>>,
    _phantom: PhantomData<P>,
}

#[async_trait]
impl<P: Processible> QueueService<P> for InMemoryQueue<P> {
    fn new() -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::new())),
            _phantom: PhantomData,
        }
    }

    async fn enqueue(&self, transaction: &P::Id) -> Result<(), Box<dyn Error + Send + Sync>> {
        let serialized = serde_json::to_string(transaction)?;
        let mut queue = self.queue.lock().await;
        queue.push_back(serialized);
        Ok(())
    }

    async fn dequeue(&self) -> Result<Option<P::Id>, Box<dyn Error + Send + Sync>> {
        let mut queue = self.queue.lock().await;
        if let Some(serialized) = queue.pop_front() {
            let order: P::Id = serde_json::from_str(&serialized)?;
            Ok(Some(order))
        } else {
            Ok(None)
        }
    }
}
