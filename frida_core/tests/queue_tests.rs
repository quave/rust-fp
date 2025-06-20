use frida_core::{
    in_memory_queue::InMemoryQueue, model::Feature, model::Processible, queue_service::QueueService,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestOrder {
    id: i64,
    order_number: String,
}

impl Processible for TestOrder {
    type Id = i64;

    fn get_id(&self) -> Self::Id {
        self.id
    }

    fn extract_features<'life0, 'async_trait>(
        &'life0 self,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = Vec<Feature>> + ::core::marker::Send + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        todo!()
    }
}

#[tokio::test]
async fn test_in_memory_queue() {
    let queue: InMemoryQueue<TestOrder> = InMemoryQueue::new();

    // Create a test order
    let order = TestOrder {
        id: 10,
        order_number: "TEST123".to_string(),
    };

    // Test enqueue
    queue.enqueue(&order.id).await.unwrap();

    // Test dequeue
    let dequeued_order_id = queue.dequeue().await.unwrap().unwrap();
    assert_eq!(dequeued_order_id, 10);

    // Test empty queue
    assert!(queue.dequeue().await.unwrap().is_none());
}
