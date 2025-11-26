use crate::{
    model::*,
    queue::{ProdQueue, QueueService, RecalcQueue},
    scorers::Scorer,
    storage::{CommonStorage, ProdCommonStorage},
};
use common::config::{CommonConfig, ProcessorConfig};
use metrics::{counter, histogram};
use std::time::Instant;
use std::{error::Error, marker::PhantomData, sync::Arc, time::Duration};
use tokio::time::sleep;
#[cfg(not(test))]
use tracing::{debug, error, info, trace};
#[cfg(test)]
use {println as debug, println as info, println as trace, println as error};

pub struct Processor<P: Processible + ProcessibleSerde, S: Scorer> {
    config: ProcessorConfig,
    scorer: S,
    storage: Arc<dyn CommonStorage>,
    proc_queue: Arc<dyn QueueService>,
    recalc_queue: Arc<dyn QueueService>,
    _phantom: PhantomData<P>,
}

impl<P: Processible + ProcessibleSerde, S: Scorer> Processor<P, S> {
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
        processing_config: ProcessorConfig,
        scorer: S,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        info!("Initializing new Processor");

        let matcher_configs = if let Some(configs) = &processing_config.matcher_configs {
            configs.clone()
        } else {
            std::collections::HashMap::new()
        };
        let common_storage = if !matcher_configs.is_empty() {
            ProdCommonStorage::<P>::with_configs(&common_config.database_url, matcher_configs)
                .await?
        } else {
            ProdCommonStorage::<P>::new(&common_config.database_url).await?
        };

        let proc_queue = Arc::new(ProdQueue::new(&common_config.database_url).await?);
        let recalc_queue = Arc::new(RecalcQueue::new(&common_config.database_url).await?);

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

    pub async fn process(
        &self,
        transaction_id: ModelId,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!(
            "Processing: Starting, transaction id: {:?}",
            &transaction_id
        );

        let total_start = Instant::now();
        let transaction = self.storage.get_transaction(transaction_id).await?;

        let processible: P = P::from_json(transaction.payload)
            .expect("Processing: Failed to deserialize transaction during processing");

        debug!(
            "Processing: Extracting matching fields for transaction {:?}",
            &transaction_id
        );
        let matching_extract_start = Instant::now();
        let matching_fields = processible.extract_matching_fields();
        {
            let h = histogram!("frida_processing_stage_seconds", "stage" => "matching_extract");
            h.record(matching_extract_start.elapsed().as_secs_f64());
        }
        self.storage
            .save_matching_fields(transaction_id, &matching_fields)
            .await
            .expect("Processing: Failed to save matching fields during processing");
        {
            let h = histogram!("frida_processing_stage_seconds", "stage" => "matching_save");
            h.record(
                Instant::now()
                    .duration_since(matching_extract_start)
                    .as_secs_f64(),
            );
        }

        // Fetch connected transactions and direct connections
        let fetch_connected_start = Instant::now();
        let connected_transactions = self.fetch_connected_transactions(transaction_id).await?;
        {
            let h = histogram!("frida_processing_stage_seconds", "stage" => "fetch_connected");
            h.record(fetch_connected_start.elapsed().as_secs_f64());
        }
        let fetch_direct_start = Instant::now();
        let direct_connections = self.fetch_direct_connections(transaction_id).await?;
        {
            let h = histogram!("frida_processing_stage_seconds", "stage" => "fetch_direct");
            h.record(fetch_direct_start.elapsed().as_secs_f64());
        }
        // Enqueue connected transactions for recalculation
        let mut added_to_recalc = 0usize;
        for ct in &connected_transactions {
            if ct.transaction_id != transaction_id {
                if let Err(e) = self.recalc_queue.enqueue(ct.transaction_id).await {
                    error!(
                        "Failed to enqueue {} for recalculation: {}",
                        ct.transaction_id, e
                    );
                } else {
                    added_to_recalc += 1;
                }
            }
        }
        info!(
            "Transactions added to the recalculation queue: {}",
            added_to_recalc
        );

        debug!(
            "Extracting features for processible {:?}, transaction {:?}",
            &transaction_id, &transaction_id
        );
        let simple_extract_start = Instant::now();
        let simple_features = processible.extract_simple_features();
        {
            let h =
                histogram!("frida_processing_stage_seconds", "stage" => "features_simple_extract");
            h.record(simple_extract_start.elapsed().as_secs_f64());
        }
        let graph_extract_start = Instant::now();
        let graph_features =
            processible.extract_graph_features(&connected_transactions, &direct_connections);
        {
            let h =
                histogram!("frida_processing_stage_seconds", "stage" => "features_graph_extract");
            h.record(graph_extract_start.elapsed().as_secs_f64());
        }
        debug!(
            "Processing: Simple features: {}",
            serde_json::to_string(&simple_features)
                .unwrap_or_else(|_| "<serialize_error>".to_string())
        );
        debug!(
            "Processing: Graph features: {}",
            serde_json::to_string(&graph_features)
                .unwrap_or_else(|_| "<serialize_error>".to_string())
        );

        debug!(
            "Processing: Saving features for transaction {:?}",
            &transaction_id
        );
        let features_save_start = Instant::now();
        self.save_features(transaction_id, &Some(&simple_features), &graph_features)
            .await?;
        {
            let h = histogram!("frida_processing_stage_seconds", "stage" => "features_save");
            h.record(features_save_start.elapsed().as_secs_f64());
        }

        let features = [simple_features, graph_features].concat();
        let score_start = Instant::now();
        self.score_and_save_results(transaction_id, features)
            .await?;
        {
            let h = histogram!("frida_processing_stage_seconds", "stage" => "score_and_save");
            h.record(score_start.elapsed().as_secs_f64());
        }

        debug!("Processing: Mark processed {:?}", &transaction_id);
        let mark_processed_start = Instant::now();
        self.storage
            .mark_transaction_processed(transaction_id)
            .await?;

        // Only mark as processed if successful
        self.proc_queue.mark_processed(transaction_id).await?;
        {
            let h = histogram!("frida_processing_stage_seconds", "stage" => "mark_processed");
            h.record(mark_processed_start.elapsed().as_secs_f64());
        }
        info!("Processing: Finished transaction id: {:?}", transaction_id);
        {
            let h = histogram!("frida_processing_stage_seconds", "stage" => "process_total");
            h.record(total_start.elapsed().as_secs_f64());
        }
        {
            let c = counter!("frida_process_total", "kind" => "process", "status" => "ok");
            c.increment(1);
        }

        Ok(())
    }

    pub async fn recalculate(
        &self,
        transaction_id: ModelId,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!(
            "Recalculation: Starting transaction id: {:?}",
            &transaction_id
        );

        let total_start = Instant::now();
        let transaction = self.storage.get_transaction(transaction_id).await?;

        let processible: P = P::from_json(transaction.payload)
            .expect("Failed to deserialize transaction during recalculation");

        // Fetch connected transactions and direct connections
        let fetch_connected_start = Instant::now();
        let connected_transactions = self.fetch_connected_transactions(transaction_id).await?;
        {
            let h = histogram!("frida_recalc_stage_seconds", "stage" => "fetch_connected");
            h.record(fetch_connected_start.elapsed().as_secs_f64());
        }
        let fetch_direct_start = Instant::now();
        let direct_connections = self.fetch_direct_connections(transaction_id).await?;
        {
            let h = histogram!("frida_recalc_stage_seconds", "stage" => "fetch_direct");
            h.record(fetch_direct_start.elapsed().as_secs_f64());
        }

        debug!(
            "Extracting features for transaction {:?} in recalculation",
            transaction_id
        );
        let graph_extract_start = Instant::now();
        let features =
            processible.extract_graph_features(&connected_transactions, &direct_connections);
        {
            let h = histogram!("frida_recalc_stage_seconds", "stage" => "recalc_graph_extract");
            h.record(graph_extract_start.elapsed().as_secs_f64());
        }
        debug!(
            "Recalculation: Graph features: {}",
            serde_json::to_string(&features).unwrap_or_else(|_| "<serialize_error>".to_string())
        );

        debug!("Saving features for transaction {:?}", &transaction_id);
        // Only update graph features during recalculation; preserve simple features
        let features_save_start = Instant::now();
        self.save_features(transaction_id, &None, &features).await?;
        {
            let h = histogram!("frida_recalc_stage_seconds", "stage" => "features_save");
            h.record(features_save_start.elapsed().as_secs_f64());
        }
        // Fetch already persisted simple features and combine with freshly computed graph features
        let (maybe_simple, _) = self.storage.get_features(transaction_id).await?;
        let simple_features: Vec<Feature> = maybe_simple.unwrap_or_default();
        let all_features = [simple_features, features].concat();
        let score_start = Instant::now();
        self.score_and_save_results(transaction_id, all_features)
            .await?;
        {
            let h = histogram!("frida_recalc_stage_seconds", "stage" => "score_and_save");
            h.record(score_start.elapsed().as_secs_f64());
        }

        // Only mark as processed if successful
        self.recalc_queue.mark_processed(transaction_id).await?;
        info!(
            "Recalculation: Finished transaction id: {:?}",
            transaction_id
        );
        {
            let h = histogram!("frida_recalc_stage_seconds", "stage" => "recalc_total");
            h.record(total_start.elapsed().as_secs_f64());
        }
        {
            let c = counter!("frida_process_total", "kind" => "recalc", "status" => "ok");
            c.increment(1);
        }

        Ok(())
    }

    async fn fetch_connected_transactions(
        &self,
        transaction_id: i64,
    ) -> Result<Vec<ConnectedTransaction>, Box<dyn Error + Send + Sync>> {
        debug!("Fetching connected transactions for {:?}", &transaction_id);
        // Default options: max_depth=3, limit_count=100, no date filters, min_confidence=50
        let connected_transactions = self
            .storage
            .find_connected_transactions(
                transaction_id,
                Some(3),     // max_depth
                Some(10000), // limit_count
                None,        // filter_config
                Some(50),    // min_confidence
            )
            .await?;
        info!(
            "Found {} connected transactions",
            connected_transactions.len()
        );
        Ok(connected_transactions)
    }

    async fn fetch_direct_connections(
        &self,
        transaction_id: i64,
    ) -> Result<Vec<DirectConnection>, Box<dyn Error + Send + Sync>> {
        debug!("Fetching direct connections for {:?}", &transaction_id);
        let direct_connections = self.storage.get_direct_connections(transaction_id).await?;

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
            error!("Failed to save features for {}: {}", transaction_id, e);
            return Err(e);
        }

        debug!("Saved features for transaction {:?}", &transaction_id);
        Ok(())
    }

    async fn score_and_save_results(
        &self,
        transaction_id: i64,
        features: Vec<Feature>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        debug!("Scoring transaction {}", transaction_id);

        let activation = self
            .storage
            .get_activation_by_channel_id(self.scorer.channel_id())
            .await
            .map_err(|e| {
                error!(
                    "Failed to fetch activation for channel {}: {}",
                    self.scorer.channel_id(),
                    e
                );
                e
            })?;
        if let Err(e) = self
            .scorer
            .score_and_save_result(transaction_id, activation.id, features)
            .await
        {
            error!(
                "Failed to score and save result for transaction {}: {}",
                transaction_id, e
            );
            return Err(e);
        }

        Ok(())
    }
}
