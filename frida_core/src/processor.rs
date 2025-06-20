use crate::{
    model::{Importable, Processible},
    queue_service::QueueService,
    scorer::Scorer,
    storage::Storage,
};
use log::{debug, info, warn};
use std::{error::Error, marker::PhantomData};

pub struct Processor<
    T: Processible,
    IT: Importable,
    S: Scorer,
    ST: Storage<IT, T>,
    Q: QueueService<T>,
> {
    scorer: S,
    storage: ST,
    queue: Q,
    _phantom_t: PhantomData<T>,
    _phantom_it: PhantomData<IT>,
}

impl<T, IT, S, ST, Q> Processor<T, IT, S, ST, Q>
where
    T: Processible,
    IT: Importable,
    S: Scorer,
    ST: Storage<IT, T>,
    Q: QueueService<T>,
{
    pub fn new(scorer: S, storage: ST, queue: Q) -> Self {
        info!("Initializing new Processor");
        Self {
            scorer,
            storage,
            queue,
            _phantom_t: PhantomData,
            _phantom_it: PhantomData,
        }
    }

    pub async fn process(&self) -> Result<Option<T>, Box<dyn Error + Send + Sync>> {
        debug!("Starting processing cycle");

        // Try to get a transaction from the queue
        if let Some(transaction_id) = self.queue.dequeue().await? {
            info!("Processing transaction with ID: {:?}", &transaction_id);

            // Get transaction details
            let transaction = match self.storage.get_transaction(&transaction_id).await {
                Ok(t) => t,
                Err(e) => {
                    warn!("Failed to get transaction {}: {}", transaction_id, e);
                    return Err(e);
                }
            };

            // Extract and save features
            debug!("Extracting features for transaction {:?}", &transaction_id);
            let features = transaction.extract_features().await;
            if let Err(e) = self
                .storage
                .save_features(&transaction.get_id(), &features)
                .await
            {
                warn!("Failed to save features for {}: {}", transaction_id, e);
                return Err(e);
            }
            info!("Extracted and saved {} features", features.len());

            // Score the transaction and save scores
            debug!("Scoring transaction {:?}", transaction_id);
            let result = self.scorer.score(features).await;
            if let Err(e) = self
                .storage
                .save_scores(&transaction.get_id(), &result)
                .await
            {
                warn!("Failed to save scores for {}: {}", transaction_id, e);
                return Err(e);
            }
            info!(
                "Scored and saved results for transaction {:?}",
                transaction_id
            );

            Ok(Some(transaction))
        } else {
            debug!("No transactions in queue");
            Ok(None)
        }
    }
}
