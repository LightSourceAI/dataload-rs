use std::collections::HashMap;
use std::hash::{BuildHasher, Hash};

pub trait Cache {
    type K;
    type V;

    /// Returns all the values associated with the provided keys in order with their respective
    /// keys.
    fn get(&self, keys: &[Self::K]) -> Vec<Option<&Self::V>>;

    /// Returns key value pairs for the requested keys.
    fn get_key_vals<'cache, 'a>(
        &'cache self,
        keys: &'a [Self::K],
    ) -> Vec<(&'a Self::K, Option<&'cache Self::V>)>;

    fn insert(&mut self, key: Self::K, value: Self::V);
    fn insert_many<I: IntoIterator<Item = (Self::K, Self::V)>>(&mut self, key_vals: I);

    fn remove(&mut self, keys: &[Self::K]);
    fn flush(&mut self);
}

impl<K, V, S: BuildHasher> Cache for HashMap<K, V, S>
where
    K: Eq + Hash,
{
    type K = K;
    type V = V;

    fn get(&self, keys: &[Self::K]) -> Vec<Option<&Self::V>> {
        keys.iter().map(|k| self.get(k)).collect::<Vec<_>>()
    }

    fn get_key_vals<'cache, 'a>(
        &'cache self,
        keys: &'a [Self::K],
    ) -> Vec<(&'a Self::K, Option<&'cache Self::V>)> {
        keys.iter().map(|k| (k, self.get(k))).collect::<Vec<_>>()
    }

    fn insert(&mut self, key: Self::K, value: Self::V) {
        self.insert(key, value);
    }

    fn insert_many<I: IntoIterator<Item = (Self::K, Self::V)>>(&mut self, key_vals: I) {
        for (key, value) in key_vals.into_iter() {
            self.insert(key, value);
        }
    }

    fn remove(&mut self, keys: &[Self::K]) {
        for key in keys.iter() {
            self.remove(key);
        }
    }

    fn flush(&mut self) {
        self.clear();
    }
}
