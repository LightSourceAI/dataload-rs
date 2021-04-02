#[derive(Debug, Default)]
pub struct WorkerStats {
    /// Human readable name used to identify this worker stats when it is reported.
    tag: &'static str,
    /// Number of `LoaderOp::Load` that were received by the worker.
    load_requests: u32,
    /// The total number of keys that were requested for loading (not necessarily unique).
    items_requested: u32,
    /// The number of keys that were immediately found in the loader cache.
    cache_hits: u32,
    /// Number of times that this worker executed the `LoaderWorker::execute_load` function.
    loads: u32,
    /// The average number of keys (not-unique) that were fetched during load operations.
    average_batch_size: f32,
    /// The max number of keys (not-unique) that were fetched during a single load.
    max_batch_size: u32,
    /// The min number of keys (not-unique) that were fetched during a single load.
    min_batch_size: u32,

    /// The max number of unique keys fetched during a single load.
    max_batch_unique: u32,
    /// The min number of unique keys fetched during a single load.
    min_batch_unique: u32,
    /// The total number of unique items that were actually loaded.
    items_loaded: u32,
}

impl WorkerStats {
    pub fn new(tag: &'static str) -> Self {
        Self { tag, min_batch_size: u32::max_value(), ..Default::default() }
    }

    pub fn record_load_request(&mut self, items_requested: u32) {
        self.load_requests += 1;
        self.items_requested += items_requested;
    }

    pub fn record_cache_hits(&mut self, hits: u32) {
        self.cache_hits += hits;
    }

    pub fn record_load_exec(&mut self, batch_size: u32) {
        let new_total_load = self.loads + 1;
        self.average_batch_size = (((self.average_batch_size as f64 * self.loads as f64)
            + batch_size as f64)
            / new_total_load as f64) as f32;
        self.loads = new_total_load;
        if batch_size > self.max_batch_size {
            self.max_batch_size = batch_size;
        }
        if batch_size < self.min_batch_size {
            self.min_batch_size = batch_size;
        }
    }

    pub fn record_load_exec_completed(&mut self, unique_batch_size: u32, loaded_item_count: u32) {
        self.items_loaded += loaded_item_count;

        if unique_batch_size > self.max_batch_unique {
            self.max_batch_size = unique_batch_size;
        }
        if unique_batch_size < self.min_batch_unique {
            self.min_batch_size = unique_batch_size;
        }
    }
}

impl Drop for WorkerStats {
    fn drop(&mut self) {
        tracing::debug!(worker_stats = ?self);
    }
}
