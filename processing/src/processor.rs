use crate::{
    model::{Processible, ConnectedTransaction, DirectConnection, Feature, TriggeredRule},
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

            let processible = self
                .processible_storage
                .get_processible(transaction_id)
                .await?;

            self.extract_and_save_matching_fields(&processible, transaction_id).await?;
            
            // Fetch connected transactions and direct connections
            let connected_transactions = self.fetch_connected_transactions(transaction_id).await?;
            let direct_connections = self.fetch_direct_connections(transaction_id).await?;
            
            let features = self.extract_and_save_features(
                &processible, 
                transaction_id, 
                &connected_transactions, 
                &direct_connections
            ).await?;
            
            self.score_and_save_results(&processible, transaction_id, features).await?;

            // Only mark as processed if successful
            self.queue.mark_processed(transaction_id).await?;

            Ok(Some(processible))
        } else {
            trace!("No transactions in queue");
            Ok(None)
        }
    }

    async fn fetch_connected_transactions(
        &self,
        transaction_id: i64
    ) -> Result<Vec<ConnectedTransaction>, Box<dyn Error + Send + Sync>> {
        debug!("Fetching connected transactions for {:?}", &transaction_id);
        // Default options: max_depth=3, limit_count=100, no date filters, min_confidence=50
        let connected_transactions = self.common_storage
            .find_connected_transactions(
                transaction_id,
                Some(3),     // max_depth
                Some(100),   // limit_count
                None,        // min_created_at
                None,        // max_created_at
                Some(50)     // min_confidence
            )
            .await?;
        
        info!("Found {} connected transactions", connected_transactions.len());
        Ok(connected_transactions)
    }

    async fn fetch_direct_connections(
        &self,
        transaction_id: i64
    ) -> Result<Vec<DirectConnection>, Box<dyn Error + Send + Sync>> {
        debug!("Fetching direct connections for {:?}", &transaction_id);
        let direct_connections = self.common_storage
            .get_direct_connections(transaction_id)
            .await?;
        
        info!("Found {} direct connections", direct_connections.len());
        Ok(direct_connections)
    }

    async fn extract_and_save_matching_fields(
        &self, 
        processible: &P, 
        transaction_id: i64
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        debug!("Extracting matching fields for transaction {:?}", &transaction_id);
        let matching_fields = processible.extract_matching_fields();
        
        if !matching_fields.is_empty() {
            if let Err(e) = self
                .common_storage
                .save_matching_fields(
                    processible.tx_id(), 
                    &matching_fields
                )
                .await
            {
                warn!("Failed to save matching fields for {}: {}", transaction_id, e);
                return Err(e);
            }
            info!("Extracted and saved {} matching fields", matching_fields.len());
        }
        
        Ok(())
    }

    async fn extract_and_save_features(
        &self, 
        processible: &P, 
        transaction_id: i64,
        connected_transactions: &[ConnectedTransaction],
        direct_connections: &[DirectConnection]
    ) -> Result<Vec<Feature>, Box<dyn Error + Send + Sync>> {
        debug!("Extracting features for transaction {:?}", &transaction_id);
        let features = processible.extract_features(
            connected_transactions,
            direct_connections
        );
        
        if let Err(e) = self
            .common_storage
            .save_features(processible.tx_id(), &features)
            .await
        {
            warn!("Failed to save features for {}: {}", transaction_id, e);
            return Err(e);
        }
        
        info!("Extracted and saved {} features", features.len());
        Ok(features)
    }

    async fn score_and_save_results(
        &self, 
        processible: &P, 
        transaction_id: i64,
        features: Vec<Feature>
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        debug!("Scoring transaction {}", transaction_id);
        let result = self.scorer.score(features).await;
        
        // Calculate total score
        let total_score: i32 = result.iter().map(|r| r.score).sum();
        
        // Convert ScorerResult to TriggeredRule
        // For now, use a default channel_id of 1 (you may need to adjust this)
        let default_channel_id = 1;
        
        // Convert scoring results to triggered rules
        let triggered_rules: Vec<TriggeredRule> = result.iter().map(|r| {
            TriggeredRule {
                id: 0, // This will be generated by the database
                scoring_events_id: 0, // This will be set by the save_scores method
                rule_id: r.name.parse().unwrap_or(0), // Convert rule name to ModelId (i64)
            }
        }).collect();
        
        if let Err(e) = self
            .common_storage
            .save_scores(processible.tx_id(), default_channel_id, total_score, &triggered_rules)
            .await
        {
            warn!("Failed to save scores for {}: {}", transaction_id, e);
            return Err(e);
        }

        info!("Scored and saved results for transaction {:?}", transaction_id);
        Ok(())
    }
}
