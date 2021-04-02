use std::slice;

use tokio::sync::oneshot;

/// Set of possible requests that can be sent to the [`LoaderWorker`]
///
/// The three categories of commands are Load, Prime, and Clear; each of which has a single and
/// many variant for convenience.
#[derive(Debug)]
pub enum LoaderOp<K, V> {
    /// Fetch data from the resource wrapped by this data loader (or the cache).
    Load(LoadRequest<K, V>),
    /// Add values to the cache that were fetched from elsewhere.
    Prime(K, V),
    PrimeMany(Vec<(K, V)>),
    /// Remove values from the cache so that they will be reloaded when they are next requested.
    Clear(K),
    ClearMany(Vec<K>),
}

#[derive(Debug)]
pub enum LoadRequest<K, V> {
    One(K, oneshot::Sender<Option<V>>),
    Many(Vec<K>, oneshot::Sender<Vec<Option<V>>>),
}

impl<K, V> LoadRequest<K, V>
where
    V: Send + Clone + std::fmt::Debug,
{
    pub fn keys(&self) -> &[K] {
        match self {
            LoadRequest::One(ref key, _) => slice::from_ref(key),
            LoadRequest::Many(ref keys, _) => keys,
        }
    }

    pub fn send_response<'a, I>(self, values: I)
    where
        I: IntoIterator<Item = Option<&'a V>>,
        V: Send + 'a,
    {
        match self {
            LoadRequest::One(_, response_tx) => {
                let response = values.into_iter().next().flatten().cloned();
                if let Err(e) = response_tx.send(response) {
                    tracing::error!(?e, "receiver dropped");
                }
            }
            LoadRequest::Many(_, response_tx) => {
                let response = values.into_iter().map(|opt| opt.cloned()).collect::<Vec<_>>();
                if let Err(e) = response_tx.send(response) {
                    tracing::error!(?e, "receiver dropped");
                }
            }
        }
    }
}
