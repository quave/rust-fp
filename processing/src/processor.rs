use crate::{
    model::*,
    queue::QueueService,
    scorers::Scorer,
    storage::{CommonStorage, ProcessibleStorage},
};
use common::config::ProcessorConfig;
use tokio::time::sleep;
#[cfg(not(test))]
use tracing::{debug, info, trace, warn};
#[cfg(test)]
use {println as debug, println as info, println as trace, println as warn};
use std::{error::Error, sync::Arc, time::Duration};

pub struct Processor<P: Processible, S: Scorer> {
    config: ProcessorConfig,
    scorer: S,
    common_storage: Arc<dyn CommonStorage>,
    processible_storage: Arc<dyn ProcessibleStorage<P>>,
    proc_queue: Arc<dyn QueueService>,
    recalc_queue: Arc<dyn QueueService>,
}

impl<P, S> Processor<P, S>
where
    P: Processible,
    S: Scorer,
{
    pub fn new(
        config: ProcessorConfig,
        scorer: S,
        common_storage: Arc<dyn CommonStorage>,
        processible_storage: Arc<dyn ProcessibleStorage<P>>,
        proc_queue: Arc<dyn QueueService>,
        recalc_queue: Arc<dyn QueueService>,
    ) -> Self {
        info!("Initializing new Processor");
        Self {
            config,
            scorer,
            common_storage,
            processible_storage,
            proc_queue,
            recalc_queue,
        }
    }

    pub async fn start_processing_worker(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!("Starting processing worker");
        
        loop {
            if let Some(transaction_id) = self.proc_queue.fetch_next().await? {
                self.process(transaction_id).await?;
            } else if let Some(transaction_id) = self.recalc_queue.fetch_next().await? {
                self.recalculate(transaction_id).await?;
            } else {
                trace!("No transactions in queues");
                sleep(Duration::from_millis(self.config.sleep_ms)).await;
            }            
        }
    }

    pub async fn process(&self, processible_id: ModelId) -> Result<Option<P>, Box<dyn Error + Send + Sync>> {
        info!("Processing model with ID: {:?}", &processible_id);

        let processible = self
            .processible_storage
            .get_processible(processible_id)
            .await?;

        let transaction_id = self.common_storage.save_transaction().await?;
        self.processible_storage.set_transaction_id(processible_id, transaction_id).await?;

        self.extract_and_save_matching_fields(&processible, transaction_id).await?;
        
        // Fetch connected transactions and direct connections
        let connected_transactions = self.fetch_connected_transactions(transaction_id).await?;
        let direct_connections = self.fetch_direct_connections(transaction_id).await?;
        
        debug!("Extracting features for processible {:?}, transaction {:?}", &processible_id, &transaction_id);
        let simple_features = processible.extract_simple_features();
        let graph_features = processible.extract_graph_features(
            &connected_transactions,
            &direct_connections
        );

        self.save_features(
            transaction_id, 
            Some(&simple_features), 
            &graph_features
        ).await?;

        let features = [simple_features, graph_features].concat();
        
        self.score_and_save_results( processible_id, features).await?;

        self.common_storage.mark_transaction_processed(transaction_id).await?;

        // Only mark as processed if successful
        self.proc_queue.mark_processed(processible_id).await?;

        Ok(Some(processible))
    }

    pub async fn recalculate(&self, processible_id: ModelId) -> Result<Option<P>, Box<dyn Error + Send + Sync>> {
        trace!("Recalculating processible with ID: {:?}", &processible_id);

        let processible = self
            .processible_storage
            .get_processible(processible_id)
            .await?;

        // Fetch connected transactions and direct connections
        let connected_transactions = self.fetch_connected_transactions(processible_id).await?;
        let direct_connections = self.fetch_direct_connections(processible_id).await?;
        
        debug!("Extracting features for transaction {:?} in recalculation", processible_id);
        let features = processible.extract_graph_features(
            &connected_transactions,
            &direct_connections
        );

        debug!("Saving features for transaction {:?}", &processible_id);
        
        self.save_features(
            processible_id, 
            None, 
            &features
        ).await?;
        
        self.score_and_save_results(processible_id, features).await?;

        // Only mark as processed if successful
        self.recalc_queue.mark_processed(processible_id).await?;

        Ok(Some(processible))
    
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
                    transaction_id, 
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

    async fn save_features(
        &self, 
        transaction_id: ModelId,
        simple_features: Option<&[Feature]>,
        graph_features: &[Feature],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        debug!("Saving features for transaction {:?}", &transaction_id);
        
        if let Err(e) = self
            .common_storage
            .save_features(transaction_id, simple_features, graph_features)
            .await
        {
            warn!("Failed to save features for {}: {}", transaction_id, e);
            return Err(e);
        }
        
        debug!("Saved features for transaction {:?}", &transaction_id);
        Ok(())
    }

    async fn score_and_save_results(
        &self, 
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
            .save_scores(transaction_id, default_channel_id, total_score, &triggered_rules)
            .await
        {
            warn!("Failed to save scores for {}: {}", transaction_id, e);
            return Err(e);
        }

        info!("Scored and saved results for transaction {:?}", transaction_id);
        Ok(())
    }
}
