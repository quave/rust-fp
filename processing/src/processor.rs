use crate::{
    model::Processible,
    queue::QueueService,
    scorers::Scorer,
    storage::{CommonStorage, ProcessibleStorage},
};
#[cfg(not(test))]
use tracing::{debug, info, trace, warn};
#[cfg(test)]
use {println as debug, println as info, println as trace, println as warn};
use std::{error::Error, sync::Arc};

pub struct Processor<P: Processible, S: Scorer> {
    scorer: S,
    common_storage: Arc<dyn CommonStorage>,
    processible_storage: Arc<dyn ProcessibleStorage<P>>,
    queue: Arc<dyn QueueService>,
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
        queue: Arc<dyn QueueService>,
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

        if let Some(transaction_id) = self.queue.fetch_next().await? {
            info!("Processing transaction with ID: {:?}", &transaction_id);

            let order = self
                .processible_storage
                .get_processible(transaction_id)
                .await?;

            // Extract and save features
            debug!("Extracting features for transaction {:?}", &transaction_id);
            let features = order.extract_features();
            println!("process 3 save_features");
            if let Err(e) = self
                .common_storage
                .save_features(order.tx_id(), &features)
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
                .save_scores(order.tx_id(), &result)
                .await
            {
                warn!("Failed to save scores for {}: {}", transaction_id, e);
                return Err(e);
            }

            info!(
                "Scored and saved results for transaction {:?}",
                transaction_id
            );

            // Only mark as processed if successful
            self.queue.mark_processed(transaction_id).await?;

            Ok(Some(order))
        } else {
            trace!("No transactions in queue");
            Ok(None)
        }
    }
}
