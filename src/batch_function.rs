use async_trait::async_trait;

/// A `BatchFunction` defines the method through which some `Loader` may fetch
/// batched data from some resource. The `BatchFunction` receives a slice of keys
/// that have been requested during the `Loader`'s most recent execution frame, and some user
/// defined context struct.
///
/// Unlike the reference facebook dataloader implementation, the BatchFunction is not required to
/// return a result for all keys that were provided. Instead, it can return any set of loaded key
/// value pairs, in any order it chooses. Requesters of keys whose values are not returned by the
/// `BatchFunction` will receive a `None`. Error handling and reporting is expected to be done
/// within the BatchFunction (i.e. through some error sink in the context).
///
/// Multiple `BatchFunctions` (and therefore loaders) can share the same context (likely through an
/// `Arc`).
#[async_trait]
pub trait BatchFunction<K, V> {
    type Context;
    async fn load(keys: &[K], context: &Self::Context) -> Vec<(K, V)>;
}
