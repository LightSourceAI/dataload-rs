use std::ops::Drop;
use std::{collections::HashMap, fmt::Debug};

use tokio::sync::{mpsc, oneshot};

use crate::{
    batch_function::BatchFunction,
    loader_op::{LoadRequest, LoaderOp},
    loader_worker::LoaderWorker,
};

/// Batch loads values from some expensive resource, primarily intended for mitigating GraphQL's
/// N+1 problem.
///
/// Users can call [`Loader::load`] and [`Loader::load_many`] to fetch values from the underlying resource or
/// cache. The cache can be cleared with calls to [`Loader::clear`] and [`Loader::clear_many`], and values can be
/// added to the cache out-of-band through the use of [`Loader::prime`] and [`Loader::prime_many`].
///
/// The `Loader` struct acts as an intermediary between the async domain in which `load` calls are
/// invoked and the pseudo-single-threaded domain of the `LoaderWorker`. Callers can invoke the
/// `Loader` from multiple parallel tasks, and the loader will enqueue the requested operations on
/// the request queue for processing by its `LoaderWorker`. The worker processes the requests
/// sequentially and provides results via response oneshot channels back to the Loader.
pub struct Loader<K, V>
where
    K: 'static + Eq + Debug + Copy + Send,
    V: 'static + Send + Debug + Clone,
{
    request_tx: mpsc::UnboundedSender<LoaderOp<K, V>>,
    load_task_handle: tokio::task::JoinHandle<()>,
}

impl<K, V> Drop for Loader<K, V>
where
    K: 'static + Eq + Debug + Copy + Send,
    V: 'static + Send + Debug + Clone,
{
    fn drop(&mut self) {
        self.load_task_handle.abort();
    }
}

impl<K, V> Loader<K, V>
where
    K: 'static + Eq + Debug + Ord + Copy + std::hash::Hash + Send + Sync,
    V: 'static + Send + Debug + Clone,
{
    /// Creates a new Loader for the provided BatchFunction and Context type.
    ///
    /// Note: the batch function is passed in as a marker for type inference.
    pub fn new<F, ContextT>(_: F, context: ContextT) -> Self
    where
        ContextT: Send + Sync + 'static,
        F: 'static + BatchFunction<K, V, Context = ContextT> + Send,
    {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            request_tx: tx,
            load_task_handle: tokio::task::spawn(
                LoaderWorker::<K, V, F, HashMap<K, V>, ContextT>::new(HashMap::new(), rx, context)
                    .start(),
            ),
        }
    }
}

impl<K, V> Loader<K, V>
where
    K: 'static + Eq + Debug + Ord + Copy + Send + Sync,
    V: 'static + Send + Debug + Clone,
{
    /// Loads a value from the underlying resource.
    ///
    /// Returns None if the value could not be loaded by the BatchFunction.
    ///
    /// If the value is already in the loader cache, it is returned as soon as it is processed.
    /// Otherwise, the requested key is enqueued for batch loading in the next loader execution
    /// frame.
    pub async fn load(&self, key: K) -> Option<V> {
        let (response_tx, response_rx) = oneshot::channel();
        self.request_tx.send(LoaderOp::Load(LoadRequest::One(key, response_tx))).unwrap();
        response_rx.await.unwrap()
    }

    /// Loads many values at once.
    ///
    /// Returns None for values that could not be loaded by the BatchFunction.
    ///
    /// If all the values are already present in the laoder cache, they are returned as soon as the
    /// request is processed by the worker. Otherwise, the keys is enqueue for batch loading in the
    /// next loader execution frame.
    pub async fn load_many(&self, keys: Vec<K>) -> Vec<Option<V>> {
        let (response_tx, response_rx) = oneshot::channel();
        self.request_tx.send(LoaderOp::Load(LoadRequest::Many(keys, response_tx))).unwrap();
        response_rx.await.unwrap()
    }

    /// Adds a value to the cache.
    pub async fn prime(&self, key: K, value: V) {
        self.request_tx.send(LoaderOp::Prime(key, value)).unwrap();
    }

    /// Adds many values to the cache at once.
    pub async fn prime_many(&self, key_vals: Vec<(K, V)>) {
        self.request_tx.send(LoaderOp::PrimeMany(key_vals)).unwrap();
    }

    /// Removes a value from the cache.
    ///
    /// This key will be reloaded when it is next requested.
    pub async fn clear(&self, key: K) {
        self.request_tx.send(LoaderOp::Clear(key)).unwrap();
    }

    /// Removes multiple values from the cache at once.
    ///
    /// These keys will be reloaded when requested.
    pub async fn clear_many(&self, keys: Vec<K>) {
        self.request_tx.send(LoaderOp::ClearMany(keys)).unwrap();
    }
}
