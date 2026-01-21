use crate::{
    model::{ConnectedTransaction, DirectConnection, Feature, Processible, ProcessibleSerde}, 
    queue::{ProdQueue, QueueName, QueueService}, 
    storage::{CommonStorage, mongo_common::MongoCommonStorage},
    scorers::Scorer, 
};
use common::config::{CommonConfig, ProcessorConfig};
use metrics::{counter, histogram, Histogram, Counter};
use mongodb::bson::oid::ObjectId;
use std::time::Instant;
use std::{error::Error, marker::PhantomData, sync::Arc, time::Duration};
use tokio::time::sleep;
#[cfg(not(test))]
use tracing::{debug, error, info};
#[cfg(test)]
use {println as debug, println as info, println as error};

pub struct ProcessorMetrics {
    processed: Counter,
    processing_fetch_transaction_timing: Histogram,
    processing_extract_matchers_timing: Histogram,
    processing_save_matching_timing: Histogram,
    processing_save_features_timing: Histogram,
    processing_score_and_save_timing: Histogram,
    processing_fetch_connected_timing: Histogram,
    processing_fetch_direct_timing: Histogram,
    processing_extract_features_simple_timing: Histogram,
    processing_extract_features_graph_timing: Histogram,
    processing_total_timing: Histogram,

    recalculated: Counter,
    recalc_fetch_transaction_timing: Histogram,
    recalc_fetch_features_simple_timing: Histogram,
    recalc_extract_features_graph_timing: Histogram,
    recalc_save_features_timing: Histogram,
    recalc_score_and_save_timing: Histogram,
    recalc_fetch_connected_timing: Histogram,
    recalc_fetch_direct_timing: Histogram,
    recalc_total_timing: Histogram,
}

impl ProcessorMetrics {
    pub fn new() -> Self {
        let processing_timing_metric_name = "frida_processing_timing";
        let recalculation_timing_metric_name = "frida_recalculation_timing";
        Self {
            processed: counter!("frida_processed_count", "status" => "ok"),
            processing_fetch_transaction_timing: histogram!(processing_timing_metric_name, "stage" => "fetch_transaction"),
            processing_extract_matchers_timing: histogram!(processing_timing_metric_name, "stage" => "extract_matchers"),
            processing_save_matching_timing: histogram!(processing_timing_metric_name, "stage" => "save_matching"),
            processing_save_features_timing: histogram!(processing_timing_metric_name, "stage" => "save_features"),
            processing_score_and_save_timing: histogram!(processing_timing_metric_name, "stage" => "score_and_save"),
            processing_fetch_connected_timing: histogram!(processing_timing_metric_name, "stage" => "fetch_connected"),
            processing_fetch_direct_timing: histogram!(processing_timing_metric_name, "stage" => "fetch_direct"),
            processing_extract_features_simple_timing: histogram!(processing_timing_metric_name, "stage" => "extract_features_simple"),
            processing_extract_features_graph_timing: histogram!(processing_timing_metric_name, "stage" => "extract_features_graph"),
            processing_total_timing: histogram!(processing_timing_metric_name, "stage" => "process_total"),

            recalculated: counter!("frida_recalculated_count", "status" => "ok"),
            recalc_fetch_transaction_timing: histogram!(recalculation_timing_metric_name, "stage" => "fetch_transaction"),
            recalc_fetch_features_simple_timing: histogram!(recalculation_timing_metric_name, "stage" => "fetch_features_simple"),
            recalc_extract_features_graph_timing: histogram!(recalculation_timing_metric_name, "stage" => "extract_features_graph"),
            recalc_save_features_timing: histogram!(recalculation_timing_metric_name, "stage" => "save_features"),
            recalc_score_and_save_timing: histogram!(recalculation_timing_metric_name, "stage" => "score_and_save"),
            recalc_fetch_connected_timing: histogram!(recalculation_timing_metric_name, "stage" => "fetch_connected"),
            recalc_fetch_direct_timing: histogram!(recalculation_timing_metric_name, "stage" => "fetch_direct"),
            recalc_total_timing: histogram!(recalculation_timing_metric_name, "stage" => "recalc_total"),
        }
    }
}

pub struct Processor<P: Processible + ProcessibleSerde<Id = ObjectId>> {
    config: Arc<ProcessorConfig>,
    scorers: Vec<Arc<dyn Scorer>>,
    storage: Arc<dyn CommonStorage<P::Id>>,
    proc_queue: Arc<dyn QueueService<P::Id>>,
    recalc_queue: Arc<dyn QueueService<P::Id>>,
    metrics: Arc<ProcessorMetrics>,
    _phantom: PhantomData<P>,
}

impl<P: Processible + ProcessibleSerde<Id = ObjectId>> Processor<P> {
    pub fn new_raw(
        config: Arc<ProcessorConfig>,
        scorers: Vec<Arc<dyn Scorer>>,
        storage: Arc<dyn CommonStorage<P::Id>>,
        proc_queue: Arc<dyn QueueService<P::Id>>,
        recalc_queue: Arc<dyn QueueService<P::Id>>,
    ) -> Self {
        Self {
            config,
            scorers,
            storage,
            proc_queue,
            recalc_queue,
            metrics: Arc::new(ProcessorMetrics::new()),
            _phantom: PhantomData,
        }
    }

    pub async fn new(
        common_config: CommonConfig,
        processing_config: Arc<ProcessorConfig>,
        scorers: Vec<Arc<dyn Scorer>>,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        info!("Initializing new Processor");

        let matcher_configs = if let Some(configs) = &processing_config.matcher_configs {
            configs.clone()
        } else {
            std::collections::HashMap::new()
        };
        let common_storage: Arc<dyn CommonStorage<P::Id>> = if !matcher_configs.is_empty() {
            Arc::new(MongoCommonStorage::with_configs(&common_config.database_url, "frida", matcher_configs)
                .await?)
        } else {
            Arc::new(MongoCommonStorage::new(&common_config.database_url, "frida").await?)
        };

        let proc_queue: Arc<dyn QueueService<P::Id>> = Arc::new(ProdQueue::new(&common_config, QueueName::Processing).await?);
        let recalc_queue = Arc::new(ProdQueue::new(&common_config, QueueName::Recalculation).await?);

        Ok(Self {
            config: processing_config,
            scorers,
            storage: common_storage,
            proc_queue,
            recalc_queue,
            metrics: Arc::new(ProcessorMetrics::new()),
            _phantom: PhantomData,
        })
    }

    pub async fn start_processing_worker(self: Arc<Processor<P>>) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!("Starting processing worker");

        loop {
            if let Some((transaction_id, msg_id)) = self.proc_queue.fetch_next(1).await?.first() {
                self.process(*transaction_id).await?;
                self.proc_queue.mark_processed(*msg_id).await?;
            } else if let Some((transaction_id, msg_id)) = self.recalc_queue.fetch_next(1).await?.first() {
                self.recalculate(*transaction_id).await?;
                self.recalc_queue.mark_processed(*msg_id).await?;
            } else {
                sleep(Duration::from_millis(self.config.sleep_ms)).await;
            }
        }
    }

    pub async fn process(
        &self,
        transaction_id: P::Id,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!(
            "Processing: Starting, transaction id: {:?}",
            &transaction_id
        );

        let perf_timer = Instant::now();
        let transaction = self.storage.get_transaction(transaction_id).await?;

        let processible: P = P::from_json(transaction.payload)
            .expect("Processing: Failed to deserialize transaction during processing");

        debug!(
            "Processing: Extracting matching fields for transaction {:?}",
            &transaction_id
        );
        let perf_stage_1 = perf_timer.elapsed();
        self.metrics.processing_fetch_transaction_timing.record(perf_stage_1);

        let matching_fields = processible.extract_matching_fields();
        let perf_stage_2 = perf_timer.elapsed();
        self.metrics.processing_extract_matchers_timing.record(perf_stage_2 - perf_stage_1);

        self.storage
            .save_matching_fields(&transaction_id, &matching_fields)
            .await
            .expect("Processing: Failed to save matching fields during processing");
        let perf_stage_3 = perf_timer.elapsed();
        self.metrics.processing_save_matching_timing.record(perf_stage_3 - perf_stage_2);

        // Fetch connected transactions and direct connections
        let connected_transactions: Vec<ConnectedTransaction> = self.fetch_connected_transactions(&processible.payload_number()).await?;
        let perf_stage_4 = perf_timer.elapsed();
        self.metrics.processing_fetch_connected_timing.record(perf_stage_4 - perf_stage_3);

        let direct_connections: Vec<DirectConnection> = self.fetch_direct_connections(&processible.payload_number()).await?;
        let perf_stage_5 = perf_timer.elapsed();
        self.metrics.processing_fetch_direct_timing.record(perf_stage_5 - perf_stage_4);

        // Enqueue connected transactions for recalculation
        let ids = connected_transactions.iter().map(|ct| ct.payload_number.clone()).collect::<Vec<_>>();
        // TODO: get tx ids or operate on payload numbers
        let enqueued_ids = self.recalc_queue.is_enqueued(&ids).await?;
        let not_enqueued_ids: Vec<P::Id> = ids.into_iter().filter(|id| !enqueued_ids.contains(id)).collect();
        self.recalc_queue.enqueue(&not_enqueued_ids).await?;
        debug!("Transactions added to the recalculation queue: {}", not_enqueued_ids.len());

        let simple_features = processible.extract_simple_features();
        let perf_stage_6 = perf_timer.elapsed();
        self.metrics.processing_extract_features_simple_timing.record(perf_stage_6 - perf_stage_5);

        let graph_features =
            processible.extract_graph_features(&connected_transactions, &direct_connections);
        let perf_stage_7 = perf_timer.elapsed();
        self.metrics.processing_extract_features_graph_timing.record(perf_stage_7 - perf_stage_6);
        debug!(
            "Processing: Simple features: {}",
            serde_json::to_string(&simple_features).unwrap_or_else(|_| "<serialize_error>".to_string())
        );
        debug!(
            "Processing: Graph features: {}",
            serde_json::to_string(&graph_features).unwrap_or_else(|_| "<serialize_error>".to_string())
        );

        debug!(
            "Processing: Saving features for transaction {:?}",
            &transaction_id
        );
        self.save_features(transaction_id, &Some(&simple_features), &graph_features)
            .await?;
        let perf_stage_8 = perf_timer.elapsed();
        self.metrics.processing_save_features_timing.record(perf_stage_8 - perf_stage_7);

        for scorer in self.scorers.iter() {
            self.score_and_save_results(transaction_id, scorer.clone(), &simple_features, &graph_features).await?;
        }

        let perf_stage_9 = perf_timer.elapsed();
        self.metrics.processing_score_and_save_timing.record(perf_stage_9 - perf_stage_8);

        debug!("Processing: Mark processed {:?}", &transaction_id);
        self.storage
            .mark_transaction_processed(transaction_id)
            .await?;

        info!("Processing: Finished transaction id: {:?}", transaction_id);
        self.metrics.processing_total_timing.record(perf_timer.elapsed());
        self.metrics.processed.increment(1);

        Ok(())
    }

    pub async fn recalculate(
        &self,
        transaction_id: P::Id,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!("Recalculation: Starting transaction id: {:?}", &transaction_id);

        let perf_timer = Instant::now();
        let transaction = self.storage.get_transaction(transaction_id).await?;

        
        let processible: P = P::from_json(transaction.payload)
            .expect("Failed to deserialize transaction during recalculation");

        let payload_number = processible.payload_number();
        let perf_stage_1 = perf_timer.elapsed();
        self.metrics.recalc_fetch_transaction_timing.record(perf_stage_1);

        let connected_transactions: Vec<ConnectedTransaction> = self.fetch_connected_transactions(&payload_number).await?;
        let perf_stage_2 = perf_timer.elapsed();
        self.metrics.recalc_fetch_connected_timing.record(perf_stage_2 - perf_stage_1);

        let direct_connections: Vec<DirectConnection> = self.fetch_direct_connections(&payload_number).await?;
        let perf_stage_3 = perf_timer.elapsed();
        self.metrics.recalc_fetch_direct_timing.record(perf_stage_3 - perf_stage_2);

        debug!("Extracting features for transaction {:?} in recalculation", transaction_id);
        let features =
            processible.extract_graph_features(&connected_transactions, &direct_connections);
        let perf_stage_4 = perf_timer.elapsed();
        self.metrics.recalc_extract_features_graph_timing.record(perf_stage_4 - perf_stage_3);

        debug!(
            "Recalculation: Graph features: {}",
            serde_json::to_string(&features).unwrap_or_else(|_| "<serialize_error>".to_string())
        );

        debug!("Saving features for transaction {:?}", &transaction_id);
        // Only update graph features during recalculation; preserve simple features
        self.save_features(transaction_id, &None, &features).await?;
        let perf_stage_5 = perf_timer.elapsed();
        self.metrics.recalc_save_features_timing.record(perf_stage_5 - perf_stage_4);

        // Fetch already persisted simple features and combine with freshly computed graph features
        let transaction = self.storage.get_transaction(transaction_id).await?;
        let features_set = transaction.features_set.expect("Failed to get features set during recalculation");
        let perf_stage_6 = perf_timer.elapsed();
        self.metrics.recalc_fetch_features_simple_timing.record(perf_stage_6 - perf_stage_5);

        for scorer in self.scorers.iter() {
            self.score_and_save_results(transaction_id, scorer.clone(), &features_set.simple_features, &features_set.graph_features).await?;
        }
        let perf_stage_7 = perf_timer.elapsed();
        self.metrics.recalc_score_and_save_timing.record(perf_stage_7 - perf_stage_6);

        // Only mark as processed if successful
        info!("Recalculation: Finished transaction id: {:?}", transaction_id);
        self.metrics.recalc_total_timing.record(perf_timer.elapsed());
        self.metrics.recalculated.increment(1);

        Ok(())
    }

    async fn fetch_connected_transactions(
        &self,
        payload_number: &str,
    ) -> Result<Vec<ConnectedTransaction>, Box<dyn Error + Send + Sync>> {
        debug!("Fetching connected transactions for payload_number {:?}", payload_number);
        // Default options: max_depth=3, limit_count=100, no date filters, min_confidence=50
        let connected_transactions = self
            .storage
            .find_connected_transactions(
                payload_number,
                Some(10),     // max_depth
                Some(200), // limit_count
                None,        // filter_config
                Some(50),    // min_confidence
            )
            .await?;
        info!(
            "Found {} connected transactions for transaction payload_number {:?}",
            connected_transactions.len(),
            payload_number
        );
        Ok(connected_transactions)
    }

    async fn fetch_direct_connections(
        &self,
        payload_number: &str,
    ) -> Result<Vec<DirectConnection>, Box<dyn Error + Send + Sync>> {
        debug!("Fetching direct connections for payload_number {:?}", &payload_number);
        let direct_connections: Vec<DirectConnection> = self.storage.get_direct_connections(payload_number).await?;

        info!("Found {} direct connections for payload_number {:?}", direct_connections.len(), payload_number);
        Ok(direct_connections)
    }

    async fn save_features(
        &self,
        transaction_id: P::Id,
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
        transaction_id: P::Id,
        scorer: Arc<dyn Scorer>,
        simple_features: &[Feature],
        graph_features: &[Feature],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        debug!("Scoring transaction {}", transaction_id);

        let scoring_result = scorer
            .score(simple_features, graph_features)
            .await?;

        self.storage.save_scores(transaction_id, scorer.channel(), scoring_result).await?;

        Ok(())
    }
}
