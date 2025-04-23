use std::{
    hash::{BuildHasher, Hash},
    num::NonZeroUsize,
    sync::Mutex,
};

use lru::{DefaultHasher, LruCache};

pub struct Cache<K, V, H = DefaultHasher>(
    // Mutex<LruCache<...>> is faster that quick_cache::Cache<...>
    Mutex<LruCache<K, V, H>>,
)
where
    K: Hash + Eq;

impl<K, V, H> Cache<K, V, H>
where
    K: Hash + Eq,
    H: BuildHasher + Default,
{
    pub fn new(size: usize) -> Self {
        Self(Mutex::new(LruCache::with_hasher(
            NonZeroUsize::new(size).unwrap(),
            H::default(),
        )))
    }

    #[cfg(feature = "code-analysis-cache")]
    pub fn get_or_insert(&self, key: K, f: impl FnOnce() -> V) -> V
    where
        V: Clone,
    {
        self.0.lock().unwrap().get_or_insert(key, f).clone()
    }

    #[cfg(feature = "hash-cache")]
    pub fn get_or_insert_ref<Q>(&self, key: &Q, f: impl FnOnce() -> V) -> V
    where
        K: std::borrow::Borrow<Q>,
        Q: ToOwned<Owned = K> + Hash + Eq,
        V: Clone,
    {
        self.0.lock().unwrap().get_or_insert_ref(key, f).clone()
    }

    #[cfg(test)]
    pub fn capacity(&self) -> usize {
        self.0.lock().unwrap().cap().into()
    }
}
