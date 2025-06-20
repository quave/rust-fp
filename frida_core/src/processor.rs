use crate::{
    model::Processible,
    queue::QueueService,
    scorer::Scorer,
    storage::{CommonStorage, ProcessibleStorage},
};
use log::{debug, info, trace, warn};
use std::{error::Error, sync::Arc};

pub struct Processor<P: Processible, S: Scorer> {
    scorer: S,
    common_storage: Arc<dyn CommonStorage>,
    processible_storage: Arc<dyn ProcessibleStorage<P>>,
    queue: Arc<dyn QueueService<P>>,
}

impl<P, S> Processor<P, S>
where
    P: Processible,
    S: Scorer,
{
    pub fn new(
        scorer: S,
        common_storage: Arc<dyn CommonStorage>,
        processible_storage: Arc<dyn ProcessibleStorage<P>>,
        queue: Arc<dyn QueueService<P>>,
    ) -> Self {
        info!("Initializing new Processor");
        Self {
            scorer,
            common_storage,
            processible_storage,
            queue,
        }
    }

    pub async fn process(&self) -> Result<Option<P>, Box<dyn Error + Send + Sync>> {
        trace!("Starting processing cycle");

        // Try to get a transaction from the queue
        if let Some(transaction_id) = self.queue.dequeue().await? {
            info!("Processing transaction with ID: {:?}", &transaction_id);

            // Get transaction details
            let transaction = match self
                .processible_storage
                .get_transaction(transaction_id)
                .await
            {
                Ok(t) => t,
                Err(e) => {
                    warn!("Failed to get transaction {}: {}", transaction_id, e);
                    return Err(e);
                }
            };

            // Extract and save features
            debug!("Extracting features for transaction {:?}", &transaction_id);
            let features = transaction.extract_features();
            if let Err(e) = self
                .processible_storage
                .save_features(transaction.id(), &features)
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
                .common_storage
                .save_scores(transaction.id(), &result)
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
            trace!("No transactions in queue");
            Ok(None)
        }
    }
}
