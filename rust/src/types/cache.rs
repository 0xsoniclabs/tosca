use std::{
    hash::{BuildHasher, Hash},
    num::NonZeroUsize,
    sync::Mutex,
};

use lru::{DefaultHasher, LruCache};

/// A thread safe key-value cache with a fixed capacity and LRU eviction policy.
pub struct Cache<K, V, H = DefaultHasher>(
    // Mutex<LruCache<...>> is faster than quick_cache::Cache<...>
    Mutex<LruCache<K, V, H>>,
);

impl<K, V, H> Cache<K, V, H>
where
    K: Hash + Eq,
    H: BuildHasher + Default,
{
    /// Creates a new cache with the given capacity. The capacity must be greater than zero.
    pub fn new(size: usize) -> Self {
        Self(Mutex::new(LruCache::with_hasher(
            NonZeroUsize::new(size).unwrap(),
            H::default(),
        )))
    }

    /// Returns the value of the key in the cache if it is present in the cache or calls the
    /// provided `FnOnce` to insert it first.
    #[cfg(feature = "code-analysis-cache")]
    pub fn get_or_insert(&self, key: K, f: impl FnOnce() -> V) -> V
    where
        V: Clone,
    {
        self.0.lock().unwrap().get_or_insert(key, f).clone()
    }

    /// Returns the value of the key in the cache if it is present in the cache or calls the
    /// provided `FnOnce` to insert it first. The referenced key is only cloned if the value is not
    /// present in the cache.
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

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    #[test]
    fn new_creates_cache_with_specified_capacity() {
        assert_eq!(Cache::<u8, u8>::new(4).capacity(), 4);
    }

    #[test]
    #[should_panic(expected = "called `Option::unwrap()` on a `None` value")]
    fn new_panics_on_zero_capacity() {
        Cache::<u8, u8>::new(0);
    }

    #[test]
    fn capacity_is_respected_on_insertions() {
        let cache = Cache::<u8, ()>::new(2);

        cache.0.lock().unwrap().get_or_insert(1, || ());
        assert_eq!(cache.0.lock().unwrap().len(), 1);
        cache.0.lock().unwrap().get_or_insert(2, || ());
        assert_eq!(cache.0.lock().unwrap().len(), 2);
        cache.0.lock().unwrap().get_or_insert(3, || ());
        assert_eq!(cache.0.lock().unwrap().len(), 2);
    }

    #[cfg(feature = "code-analysis-cache")]
    #[test]
    fn get_or_insert_returns_cached_values_and_computes_and_inserts_only_on_cache_miss() {
        let cache = Cache::<u8, u8>::new(2);
        let calls = Cell::new(0);
        let get = |key| {
            cache.get_or_insert(key, || {
                calls.set(calls.get() + 1);
                key * 10
            })
        };

        assert_eq!(get(1), 10);
        assert_eq!(calls.get(), 1);
        // different key
        assert_eq!(get(2), 20);
        assert_eq!(calls.get(), 2);
        // same key again
        assert_eq!(get(1), 10);
        assert_eq!(calls.get(), 2);
        // different key
        assert_eq!(get(3), 30); // evicts 2
        assert_eq!(calls.get(), 3);
        // evicted key is recomputed
        assert_eq!(get(2), 20);
        assert_eq!(calls.get(), 4);
    }

    #[cfg(feature = "hash-cache")]
    #[test]
    fn get_or_insert_ref_returns_cached_values_and_computes_and_inserts_only_on_cache_miss() {
        let cache = Cache::<u8, u8>::new(2);
        let calls = Cell::new(0);
        let get = |key: u8| {
            cache.get_or_insert_ref(&key, || {
                calls.set(calls.get() + 1);
                key * 10
            })
        };

        assert_eq!(get(1), 10);
        assert_eq!(calls.get(), 1);
        // different key
        assert_eq!(get(2), 20);
        assert_eq!(calls.get(), 2);
        // same key again
        assert_eq!(get(1), 10);
        assert_eq!(calls.get(), 2);
        // different key
        assert_eq!(get(3), 30); // evicts 2
        assert_eq!(calls.get(), 3);
        // evicted key is recomputed
        assert_eq!(get(2), 20);
        assert_eq!(calls.get(), 4);
    }
}
