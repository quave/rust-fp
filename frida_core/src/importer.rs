use crate::{
    model::{Importable, Processible},
    queue_service::QueueService,
    storage::Storage,
};
use log::{debug, info, warn};
use std::{error::Error, marker::PhantomData};

pub struct Importer<T: Processible, IT: Importable, ST: Storage<IT, T>, Q: QueueService<T>> {
    storage: ST,
    queue: Q,
    _phantom_t: PhantomData<T>,
    _phantom_it: PhantomData<IT>,
}

impl<T, IT, ST, Q> Importer<T, IT, ST, Q>
where
    T: Processible,
    IT: Importable,
    ST: Storage<IT, T>,
    Q: QueueService<T>,
{
    pub fn new(storage: ST, queue: Q) -> Self {
        info!("Initializing new Importer");
        Self {
            storage,
            queue,
            _phantom_t: PhantomData,
            _phantom_it: PhantomData,
        }
    }

    pub async fn import(&self, transaction: IT) -> Result<T::Id, Box<dyn Error + Send + Sync>> {
        debug!("Starting import process for new transaction");

        // Save the transaction
        let transaction_id = match self.storage.save_transaction(&transaction).await {
            Ok(id) => {
                info!("Successfully saved transaction with ID: {:?}", id);
                id
            }
            Err(e) => {
                warn!("Failed to save transaction: {}", e);
                return Err(e);
            }
        };

        // Queue for processing
        if let Err(e) = self.queue.enqueue(&transaction_id).await {
            warn!("Failed to queue transaction {:?}: {}", transaction_id, e);
            return Err(e);
        }
        info!(
            "Successfully queued transaction {:?} for processing",
            transaction_id
        );

        Ok(transaction_id)
    }
}
