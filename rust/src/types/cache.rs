use std::{
    hash::{BuildHasher, Hash},
    num::NonZeroUsize,
    sync::{LazyLock, Mutex},
};

use lru::{DefaultHasher, LruCache};

pub struct Cache<const S: usize, K, V, H = DefaultHasher>(
    // Mutex<LruCache<...>> is faster that quick_cache::Cache<...>
    LazyLock<Mutex<LruCache<K, V, H>>>,
)
where
    K: Hash + Eq;

impl<const S: usize, K, V, H> Cache<S, K, V, H>
where
    K: Hash + Eq,
    H: BuildHasher + Default,
{
    pub const fn new() -> Self {
        Self(LazyLock::new(|| {
            Mutex::new(LruCache::with_hasher(
                NonZeroUsize::new(S).unwrap(),
                H::default(),
            ))
        }))
    }

    pub fn get_or_insert(&self, key: K, f: impl FnOnce() -> V) -> V
    where
        V: Clone,
    {
        self.0.lock().unwrap().get_or_insert(key, f).clone()
    }
}
