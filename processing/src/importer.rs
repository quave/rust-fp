use std::{error::Error, marker::PhantomData, sync::Arc};

use crate::{
    model::{ModelId, Processible, ProcessibleSerde},
    queue::QueueService,
    storage::CommonStorage,
};

#[derive(Clone)]
pub struct Importer<P: Processible + ProcessibleSerde> {
    storage: Arc<dyn CommonStorage>,
    queue: Arc<dyn QueueService>,
    _phantom: PhantomData<P>,
}

impl<P: Processible + ProcessibleSerde> Importer<P> {
    pub fn new(
        storage: Arc<dyn CommonStorage>,
        queue: Arc<dyn QueueService>,
    ) -> Self {
        tracing::info!("Initializing new Importer");
        Self {
            storage,
            queue,
            _phantom: PhantomData,
        }
    }

    pub async fn import(&self, processible: P) -> Result<ModelId, Box<dyn Error + Send + Sync>> {
        tracing::debug!("Starting import process for new transaction");

        let payload_number = processible.payload_number();
        let payload = processible.as_json()?;
        let schema_version = processible.schema_version();

        let id = self
            .storage
            .insert_transaction(payload_number, payload, schema_version)
            .await?;

        self.queue.enqueue(id).await?;
        tracing::info!("Successfully queued importable {:?} for processing", id);

        Ok(id)
    }
}
