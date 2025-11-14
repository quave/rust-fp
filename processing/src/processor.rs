use crate::{
    model::*,
    queue::{ProdQueue, QueueService},
    scorers::Scorer,
    storage::{CommonStorage, ProdCommonStorage},
};
use common::config::{CommonConfig, ProcessorConfig};
use tokio::time::sleep;
#[cfg(not(test))]
use tracing::{debug, info, trace, warn};
#[cfg(test)]
use {println as debug, println as info, println as trace, println as warn};
use std::{error::Error, marker::PhantomData, sync::Arc, time::Duration};

pub struct Processor<P: Processible + ProcessibleSerde, S: Scorer> {
    config: ProcessorConfig,
    scorer: S,
    storage: Arc<dyn CommonStorage>,
    proc_queue: Arc<dyn QueueService>,
    recalc_queue: Arc<dyn QueueService>,
    _phantom: PhantomData<P>,
}

impl<P: Processible + ProcessibleSerde + 'static, S: Scorer> Processor<P, S>
{
    pub fn new_raw(
        config: ProcessorConfig,
        scorer: S,
        storage: Arc<dyn CommonStorage>,
        proc_queue: Arc<dyn QueueService>,
        recalc_queue: Arc<dyn QueueService>,
        ) -> Self {
        Self {
            config,
            scorer,
            storage,
            proc_queue,
            recalc_queue,
            _phantom: PhantomData,
        }
    }

    pub async fn new(
        common_config: CommonConfig,
        processing_config:    ProcessorConfig,
        scorer: S,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        info!("Initializing new Processor");

        let matcher_configs = if let Some(configs) = &processing_config.matcher_configs {
            configs.clone()
        } else {
            std::collections::HashMap::new()
        };
        let common_storage =
            if !matcher_configs.is_empty() {
                ProdCommonStorage::<P>::with_configs(&common_config.database_url, matcher_configs).await?
            } else {
                ProdCommonStorage::<P>::new(&common_config.database_url).await?
            };
        

        let proc_queue = Arc::new(ProdQueue::new(&common_config.database_url).await?);
        let recalc_queue = Arc::new(ProdQueue::new(&common_config.database_url).await?);
    
        Ok(Self {
            config: processing_config,
            scorer,
            storage: Arc::new(common_storage),
            proc_queue,
            recalc_queue,
            _phantom: PhantomData,
        })
    }

    pub async fn start_processing_worker(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!("Starting processing worker");
        
        loop {
            if let Some(transaction_id) = self.proc_queue.fetch_next(1).await?.first() {
                self.process(*transaction_id).await?;
            } else if let Some(transaction_id) = self.recalc_queue.fetch_next(1).await?.first() {
                self.recalculate(*transaction_id).await?;
            } else {
                trace!("No transactions in queues");
                sleep(Duration::from_millis(self.config.sleep_ms)).await;
            }            
        }
    }

    pub async fn process(&self, transaction_id: ModelId) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!("Processing: Starting, transaction id: {:?}", &transaction_id);

        let transaction = self
            .storage
            .get_transaction(transaction_id)
            .await?;

        let processible: P = P::from_json(transaction.payload)
        .expect("Processing: Failed to deserialize transaction during processing");

        debug!("Processing: Extracting matching fields for transaction {:?}", &transaction_id);
        let matching_fields = processible.extract_matching_fields();
        self
            .storage
            .save_matching_fields(
                transaction_id, 
                &matching_fields
            )
            .await
            .expect("Processing: Failed to save matching fields during processing");

        // Fetch connected transactions and direct connections
        let connected_transactions = self.fetch_connected_transactions(transaction_id).await?;
        let direct_connections = self.fetch_direct_connections(transaction_id).await?;
        
        debug!("Extracting features for processible {:?}, transaction {:?}", &transaction_id, &transaction_id);
        let simple_features = processible.extract_simple_features();
        let graph_features = processible.extract_graph_features(
            &connected_transactions,
            &direct_connections
        );

        debug!("Processing: Extracting matching fields for transaction {:?}", &transaction_id);
        self.save_features(
            transaction_id, 
            &Some(&simple_features), 
            &graph_features
        ).await?;

        let features = [simple_features, graph_features].concat();
        
        self.score_and_save_results( transaction_id, features).await?;

        debug!("Processing: Extracting matching fields for transaction {:?}", &transaction_id);
        self.storage.mark_transaction_processed(transaction_id).await?;

        // Only mark as processed if successful
        self.proc_queue.mark_processed(transaction_id).await?;

        Ok(())
    }

    pub async fn recalculate(&self, transaction_id: ModelId) -> Result<(), Box<dyn Error + Send + Sync>> {
        debug!("Recalculating transaction with id: {:?}", &transaction_id);

        let transaction = self
            .storage
            .get_transaction(transaction_id)
            .await?;

        let processible: P = P::from_json(transaction.payload)
            .expect("Failed to deserialize transaction during recalculation");

        // Fetch connected transactions and direct connections
        let connected_transactions = self.fetch_connected_transactions(transaction_id).await?;
        let direct_connections = self.fetch_direct_connections(transaction_id).await?;
        
        debug!("Extracting features for transaction {:?} in recalculation", transaction_id);
        let features = processible.extract_graph_features(
            &connected_transactions,
            &direct_connections
        );

        debug!("Saving features for transaction {:?}", &transaction_id);
        
        self.save_features(
            transaction_id, 
            &None, 
            &features
        ).await?;
        
        self.score_and_save_results(transaction_id, features).await?;

        // Only mark as processed if successful
        self.recalc_queue.mark_processed(transaction_id).await?;

        Ok(())
    }

    async fn fetch_connected_transactions(
        &self,
        transaction_id: i64
    ) -> Result<Vec<ConnectedTransaction>, Box<dyn Error + Send + Sync>> {
        debug!("Fetching connected transactions for {:?}", &transaction_id);
        // Default options: max_depth=3, limit_count=100, no date filters, min_confidence=50
        let connected_transactions = self.storage
            .find_connected_transactions(
                transaction_id,
                Some(3),     // max_depth
                Some(10000),   // limit_count
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
        let direct_connections = self.storage
            .get_direct_connections(transaction_id)
            .await?;
        
        info!("Found {} direct connections", direct_connections.len());
        Ok(direct_connections)
    }

    async fn save_features(
        &self, 
        transaction_id: ModelId,
        simple_features: &Option<&[Feature]>,
        graph_features: &[Feature],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        debug!("Saving features for transaction {:?}", &transaction_id);
        
        if let Err(e) = self
            .storage
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
            .storage
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
