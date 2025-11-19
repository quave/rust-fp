use std::{error::Error, marker::PhantomData, sync::Arc};

use crate::{
    model::{ModelId, Processible, ProcessibleSerde},
    queue::QueueService,
    storage::CommonStorage,
};
use metrics::{counter, histogram};
use std::time::Instant;

#[derive(Clone)]
pub struct Importer<P: Processible + ProcessibleSerde> {
    storage: Arc<dyn CommonStorage>,
    queue: Arc<dyn QueueService>,
    _phantom: PhantomData<P>,
}

impl<P: Processible + ProcessibleSerde> Importer<P> {
    pub fn new(
        storage: Arc<dyn CommonStorage>,
        queue: Arc<dyn QueueService>,
    ) -> Self {
        tracing::info!("Initializing new Importer");
        Self {
            storage,
            queue,
            _phantom: PhantomData,
        }
    }

    pub async fn import(&self, processible: P) -> Result<ModelId, Box<dyn Error + Send + Sync>> {
        tracing::debug!("Starting import process for new transaction");

        let total_start = Instant::now();
        let payload_number = processible.payload_number();
        let payload = processible.as_json()?;
        let schema_version = processible.schema_version();

        let insert_start = Instant::now();
        let id = self
            .storage
            .insert_transaction(payload_number, payload, schema_version)
            .await?;
        {
            let h = histogram!("frida_import_duration_seconds", "stage" => "insert_transaction");
            h.record(insert_start.elapsed().as_secs_f64());
        }

        let enqueue_start = Instant::now();
        self.queue.enqueue(id).await?;
        {
            let h = histogram!("frida_import_duration_seconds", "stage" => "enqueue");
            h.record(enqueue_start.elapsed().as_secs_f64());
        }
        tracing::info!("Successfully queued importable {:?} for processing", id);
        {
            let h = histogram!("frida_import_duration_seconds", "stage" => "total");
            h.record(total_start.elapsed().as_secs_f64());
        }
        {
            let c = counter!("frida_import_total", "status" => "ok");
            c.increment(1);
        }

        Ok(id)
    }
}
