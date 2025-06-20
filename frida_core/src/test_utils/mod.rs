use std::{collections::VecDeque, sync::Mutex};

use crate::{
    model::{ModelId, Processible},
    queue::QueueService,
};

// Mock queue implementation for tests
#[derive(Default)]
pub struct MockQueue {
    queue: Mutex<VecDeque<ModelId>>,
}

impl MockQueue {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
        }
    }
}

#[async_trait::async_trait]
impl<P: Processible> QueueService<P> for MockQueue {
    async fn enqueue(&self, id: ModelId) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.queue.lock().unwrap().push_back(id.clone());
        Ok(())
    }

    async fn dequeue(&self) -> Result<Option<ModelId>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self.queue.lock().unwrap().pop_front())
    }
}
