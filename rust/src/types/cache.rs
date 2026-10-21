#[cfg(feature = "hash-cache")]
use std::num::NonZeroUsize;
use std::{
    hash::{BuildHasher, Hash},
    sync::Mutex,
};

use lru::{DefaultHasher, LruCache};

/// A thread safe key-value cache with a fixed capacity and LRU eviction policy.
#[cfg(feature = "hash-cache")]
pub struct Cache<K, V, H = DefaultHasher>(
    // Mutex<LruCache<...>> is faster than quick_cache::Cache<...>
    Mutex<LruCache<K, V, H>>,
);

#[cfg(feature = "hash-cache")]
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
    /// provided `FnOnce` to insert it first. The referenced key is only cloned if the value is not
    /// present in the cache.
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

/// A value whose weight counts against the capacity of a [`WeightedCache`].
#[cfg(feature = "code-analysis-cache")]
pub trait Weight {
    /// Returns the weight of the value.
    fn weight(&self) -> usize;
}

/// A thread safe key-value cache that evicts the least recently used entries once the total
/// [`Weight`] of its values would exceed its capacity.
#[cfg(feature = "code-analysis-cache")]
pub struct WeightedCache<K, V, H = DefaultHasher> {
    capacity: usize,
    // The entries and their total weight.
    lru: Mutex<(LruCache<K, V, H>, usize)>,
}

#[cfg(feature = "code-analysis-cache")]
impl<K, V, H> WeightedCache<K, V, H>
where
    K: Hash + Eq,
    V: Weight + Clone,
    H: BuildHasher + Default,
{
    /// Creates a new [`WeightedCache`] whose values weigh at most `capacity` in total.
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            lru: Mutex::new((LruCache::unbounded_with_hasher(H::default()), 0)),
        }
    }

    /// Returns the value of the key in the cache if it is present in the cache or calls the
    /// provided `FnOnce` to insert it first. A value heavier than the capacity is not inserted.
    pub fn get_or_insert(&self, key: K, f: impl FnOnce() -> V) -> V {
        let mut lru = self.lru.lock().unwrap();
        let (lru, total_weight) = &mut *lru;
        if let Some(value) = lru.get(&key) {
            return value.clone();
        }

        let value = f();
        let weight = value.weight();
        if weight <= self.capacity {
            while *total_weight + weight > self.capacity {
                let (_, evicted) = lru.pop_lru().unwrap();
                *total_weight -= evicted.weight();
            }
            *total_weight += weight;
            lru.push(key, value.clone());
        }
        value
    }

    #[cfg(test)]
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    #[cfg(feature = "hash-cache")]
    #[test]
    fn new_creates_cache_with_specified_capacity() {
        assert_eq!(Cache::<u8, u8>::new(4).capacity(), 4);
    }

    #[cfg(feature = "hash-cache")]
    #[test]
    #[should_panic(expected = "called `Option::unwrap()` on a `None` value")]
    fn new_panics_on_zero_capacity() {
        Cache::<u8, u8>::new(0);
    }

    #[cfg(feature = "hash-cache")]
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

    #[cfg(feature = "code-analysis-cache")]
    impl Weight for u8 {
        fn weight(&self) -> usize {
            (*self).into()
        }
    }

    #[cfg(feature = "code-analysis-cache")]
    #[test]
    fn weighted_cache_get_or_insert_returns_cached_values_and_computes_and_inserts_only_on_cache_miss()
     {
        let cache = WeightedCache::<u8, u8>::new(50);
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

    #[cfg(feature = "code-analysis-cache")]
    #[test]
    fn weighted_cache_get_or_insert_evicts_least_recently_used_values_until_the_new_value_fits() {
        let cache = WeightedCache::<u8, u8>::new(6);
        for key in [1, 2, 3, 1, 4] {
            cache.get_or_insert(key, || key);
        }

        let (lru, total_weight) = &*cache.lru.lock().unwrap();
        assert_eq!(lru.iter().map(|(key, _)| *key).collect::<Vec<_>>(), [4, 1]);
        assert_eq!(*total_weight, 5);
    }

    #[cfg(feature = "code-analysis-cache")]
    #[test]
    fn weighted_cache_get_or_insert_does_not_insert_values_heavier_than_the_capacity() {
        let cache = WeightedCache::<u8, u8>::new(5);
        cache.get_or_insert(1, || 1);

        assert_eq!(cache.get_or_insert(6, || 6), 6);

        let (lru, total_weight) = &*cache.lru.lock().unwrap();
        assert_eq!(lru.iter().map(|(key, _)| *key).collect::<Vec<_>>(), [1]);
        assert_eq!(*total_weight, 1);
    }
}
