use std::{error::Error, sync::Arc};

use crate::{
    model::{Importable, ImportableSerde, ModelId},
    queue::QueueService,
    storage::ImportableStorage,
};

#[derive(Clone)]
pub struct Importer<I: Importable> {
    importable_storage: Arc<dyn ImportableStorage<I>>,
    queue: Arc<dyn QueueService>,
}

impl<I: Importable> Importer<I> {
    pub fn new(
        importable_storage: Arc<dyn ImportableStorage<I>>,
        queue: Arc<dyn QueueService>,
    ) -> Self {
        tracing::info!("Initializing new Importer");
        Self {
            importable_storage,
            queue,
        }
    }

    pub fn extract_model<IS: ImportableSerde + 'static>(
        json: &str,
    ) -> Result<Box<dyn Importable>, Box<dyn Error + Send + Sync>> {
        let model: IS = serde_json::from_str(json)?;
        Ok(Box::new(model))
    }

    pub async fn import(&self, importable: I) -> Result<ModelId, Box<dyn Error + Send + Sync>> {
        tracing::debug!("Starting import process for new transaction");

        let id = self
            .importable_storage
            .save(&importable)
            .await?;
        self.queue.enqueue(id).await?;
        tracing::info!("Successfully queued importable {:?} for processing", id);

        Ok(id)
    }
}
