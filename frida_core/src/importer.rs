use std::{error::Error, sync::Arc};

use crate::{
    model::{Importable, ImportableSerde, ModelId, Processible},
    queue::QueueService,
    storage::ImportableStorage,
};
use log::{debug, info};

pub struct Importer<I: Importable, P: Processible> {
    importable_storage: Arc<dyn ImportableStorage<I>>,
    queue: Arc<dyn QueueService<P>>,
}

impl<I: Importable, P: Processible> Importer<I, P> {
    pub fn new(
        importable_storage: Arc<dyn ImportableStorage<I>>,
        queue: Arc<dyn QueueService<P>>,
    ) -> Self {
        info!("Initializing new Importer");
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

    pub async fn import(&self, transaction: I) -> Result<ModelId, Box<dyn Error + Send + Sync>> {
        debug!("Starting import process for new transaction");

        let id = self
            .importable_storage
            .save_transaction(&transaction)
            .await?;
        self.queue.enqueue(id).await?;
        info!("Successfully queued transaction {:?} for processing", id);

        Ok(id)
    }
}
