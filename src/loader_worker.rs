use std::fmt::Debug;
use std::marker::PhantomData;
use std::slice;

use futures::future::FutureExt;
use tokio::sync::mpsc;
use tracing::{span, Level};

use crate::{
    batch_function::BatchFunction,
    cache::Cache,
    loader_op::{LoadRequest, LoaderOp},
};

/// A `LoaderWorker` is the "single-thread" worker task that actually does the loading work.
///
/// Once started, it runs in a loop until the parent Loader aborts it's `JoinHandle` or drops the
/// request queue tx channel.
///
/// The worker can be in one of three states during its lifetime:
///
/// 1. Waiting for requests
/// 2. Flushing the request queue and staging keys for loading.
/// 3. Executing its load batch function.
///
/// One cycle through this loop may be called an "execution frame".
///
/// In state (1), the worker awaits any messages on the request queue channel, idling until work arrives.
///
/// In state (2), the worker will synchronously pull request from the queue until it receives a NoneType indicating that
/// there are no more pending requests. Prime and Clear requests are resolved immediately by
/// synchronously issuing requests to the cache. For Load requests, the worker checks if the
/// request can be resolved immediately from the cache. If so, it immediately sends the value on
/// the load request's response channel, otherwise it determines which keys are missing from the
/// cache and stages them for loading.
///
/// In state (3), the loader invokes its `BatchFunction` with the set of keys that it collected in
/// (2). The values returned by the `BatchFunction` are inserted into the cache and then
/// outstanding Load requests are resolved from the cache. If the `BatchFunction` did not return a
/// that was requested (perhaps because of an error), the corresponding Load request is returned a
/// NoneType on its response channel.
pub struct LoaderWorker<K, V, F, CacheT, ContextT>
where
    K: 'static + Eq + Debug + Ord + Copy + Send + Sync,
    V: 'static + Send + Debug + Clone,
    F: 'static + BatchFunction<K, V, Context = ContextT> + Send,
    CacheT: Cache,
    ContextT: Send + Sync + 'static,
{
    cache: CacheT,
    request_rx: mpsc::UnboundedReceiver<LoaderOp<K, V>>,
    keys_to_load: Vec<K>,
    pending_request: Vec<LoadRequest<K, V>>,
    context: ContextT,
    phantom_batch_function: PhantomData<F>,
    debug_name: &'static str,
}

impl<K, V, F, CacheT, ContextT> LoaderWorker<K, V, F, CacheT, ContextT>
where
    K: 'static + Eq + Debug + Copy + Ord + Send + Sync,
    V: 'static + Send + Debug + Clone,
    F: 'static + BatchFunction<K, V, Context = ContextT> + Send,
    CacheT: Cache<K = K, V = V>,
    ContextT: Send + Sync + 'static,
{
    pub fn new(
        cache: CacheT,
        request_rx: mpsc::UnboundedReceiver<LoaderOp<K, V>>,
        context: ContextT,
    ) -> Self {
        Self {
            cache,
            request_rx,
            keys_to_load: Vec::new(),
            pending_request: Vec::new(),
            context,
            phantom_batch_function: PhantomData,
            debug_name: std::any::type_name::<(K, V)>(),
        }
    }

    pub async fn start(mut self) {
        let span = span!(Level::TRACE, "LoaderWorker", kv = self.debug_name,);
        let _enter = span.enter();

        loop {
            // Async await until we receive the first op.
            match self.request_rx.recv().await {
                None => {
                    tracing::info!("Tx channel closed. Terminating LoaderWorker.");
                    return;
                }
                Some(op) => self.mux_op(op),
            }
            // Flush remainder of the op queue before executing load.
            while let Some(Some(op)) = self.request_rx.recv().now_or_never() {
                self.mux_op(op);
            }
            if !self.pending_request.is_empty() {
                self.execute_load().await;
            }
        }
    }

    #[tracing::instrument(skip(self))]
    fn mux_op(&mut self, op: LoaderOp<K, V>) {
        match op {
            LoaderOp::Load(request) => {
                let cached = self.cache.get_key_vals(request.keys());
                let keys_to_load = cached
                    .iter()
                    .filter_map(|(k, v)| if v.is_none() { Some(**k) } else { None })
                    .collect::<Vec<_>>();
                tracing::debug!(requested_keys = ?request.keys(), ?keys_to_load);
                if keys_to_load.is_empty() {
                    let values = cached.into_iter().map(|(_k, v)| v).collect::<Vec<_>>();
                    request.send_response(values);
                } else {
                    self.keys_to_load.extend(&keys_to_load);
                    self.pending_request.push(request);
                }
            }
            LoaderOp::Prime(key, value) => self.cache.insert(key, value),
            LoaderOp::PrimeMany(key_vals) => self.cache.insert_many(key_vals),
            LoaderOp::Clear(key) => self.cache.remove(slice::from_ref(&key)),
            LoaderOp::ClearMany(keys) => self.cache.remove(&keys),
        }
    }

    #[tracing::instrument(skip(self))]
    async fn execute_load(&mut self) {
        self.keys_to_load.sort();
        self.keys_to_load.dedup();
        let loaded_keyvals = F::load(&self.keys_to_load, &self.context).await;
        tracing::debug!(?loaded_keyvals);
        self.cache.insert_many(loaded_keyvals);

        for request in self.pending_request.drain(..) {
            let values = self.cache.get(request.keys());
            request.send_response(values);
        }
    }
}
