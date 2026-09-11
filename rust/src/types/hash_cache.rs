use sha3::{Digest, Keccak256, digest::FixedOutput};

#[cfg(feature = "hash-cache")]
use crate::types::Cache;
use crate::types::u256;

#[cfg(feature = "hash-cache")]
type HashCache32 = Cache<[u8; 32], u256>;
#[cfg(feature = "hash-cache")]
type HashCache64 = Cache<[u8; 64], u256>;

pub struct HashCache {
    #[cfg(feature = "hash-cache")]
    hash_cache_32: HashCache32,
    #[cfg(feature = "hash-cache")]
    hash_cache_64: HashCache64,
}

impl Default for HashCache {
    fn default() -> Self {
        Self::new(Self::DEFAULT_CACHE_SIZE)
    }
}

impl HashCache {
    const DEFAULT_CACHE_SIZE: usize = 1024; // value taken from evmzero

    #[allow(unused_variables)]
    pub fn new(size: usize) -> Self {
        Self {
            #[cfg(feature = "hash-cache")]
            hash_cache_32: HashCache32::new(size),
            #[cfg(feature = "hash-cache")]
            hash_cache_64: HashCache64::new(size),
        }
    }

    fn sha3(data: &[u8]) -> u256 {
        let mut hasher = Keccak256::new();
        hasher.update(data);
        let bytes = hasher.finalize_fixed().into();

        u256::from_be_bytes(bytes)
    }

    #[allow(clippy::unused_self)]
    pub fn hash(&self, data: &[u8]) -> u256 {
        std::cfg_select! {
            feature = "hash-cache" => {
                if let Some(data) = data.as_array::<32>() {
                    self.hash_cache_32
                        .get_or_insert_ref(data, || Self::sha3(data))
                } else if let Some(data) = data.as_array::<64>() {
                    self.hash_cache_64
                        .get_or_insert_ref(data, || Self::sha3(data))
                } else {
                    Self::sha3(data)
                }
            }
            _ => Self::sha3(data),
        }
    }

    #[cfg(test)]
    #[allow(clippy::unused_self)]
    pub fn capacity(&self) -> usize {
        std::cfg_select! {
            feature = "hash-cache" => {
                assert_eq!(self.hash_cache_32.capacity(), self.hash_cache_64.capacity());
                self.hash_cache_32.capacity()
            }
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[test]
    fn default_uses_the_default_capacity_or_zero_if_hash_cache_is_disabled() {
        assert_eq!(
            HashCache::default().capacity(),
            cfg_select! {
                feature = "hash-cache" => HashCache::DEFAULT_CACHE_SIZE,
                _ => 0,
            }
        );
    }

    #[test]
    fn new_uses_the_requested_capacity_or_zero_if_hash_cache_is_disabled() {
        assert_eq!(
            HashCache::new(8).capacity(),
            cfg_select! {
                feature = "hash-cache" => 8,
                _ => 0,
            }
        );
    }

    #[test]
    fn sha3_creates_u256_from_big_endian_bytes() {
        let data = [0xab; 32];

        let mut hasher = Keccak256::new();
        hasher.update(data);
        let bytes: [u8; 32] = hasher.finalize_fixed().into();

        assert_eq!(HashCache::sha3(&data).to_be_bytes(), bytes);
    }

    #[rstest]
    #[case::empty(
        0,
        "c5d2460186f7233c_927e7db2dcc703c0_e500b653ca82273b_7bfad8045d85a470"
    )]
    #[case::cached_32_bytes(
        32,
        "7d3a608bb850f47c_2d77d6be73b8f93c_94a80264b7bb3cc5_c7d2fb54d07ef6b9"
    )]
    #[case::uncached_33_bytes(
        33,
        "034ef2adf2deaa46_a3b4d9ca2a21a4ab_513d60477f6a454b_a9dde60a9b3e4803"
    )]
    #[case::cached_64_bytes(
        64,
        "1326a3abf3dcddf6_71522549daabcd80_6131ee546c4a0353_0fa3075b3ef5a399"
    )]
    #[case::uncached_65_bytes(
        65,
        "1090dbec48f7f57f_241cd63982ccab20_2c65844d3a54ee79_7b1a6de433635179"
    )]
    fn hash_returns_keccak256_whether_or_not_the_length_is_cached(
        #[case] len: usize,
        #[case] expected: &str,
    ) {
        let cache = HashCache::default();
        let data = vec![0xab; len];
        assert_eq!(format!("{:x}", cache.hash(&data)), expected);
        assert_eq!(format!("{:x}", cache.hash(&data)), expected); // second call hits the cache if applicable
    }
}
