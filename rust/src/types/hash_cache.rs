#[cfg(feature = "hash-cache")]
use std::env;

use sha3::{Digest, Keccak256};

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

impl HashCache {
    #[cfg(feature = "hash-cache")]
    const ENV_VAR: &str = "EVMRS_HASH_CACHE_SIZE";

    #[cfg(feature = "hash-cache")]
    const DEFAULT_CACHE_SIZE: usize = 1024; // value taken from evmzero

    pub fn new_from_env_size() -> Self {
        #[cfg(feature = "hash-cache")]
        let size = env::var(Self::ENV_VAR)
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(Self::DEFAULT_CACHE_SIZE);
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
        let mut bytes = [0; 32];
        hasher.finalize_into((&mut bytes).into());

        u256::from_be_bytes(bytes)
    }

    #[allow(clippy::unused_self)]
    pub fn hash(&self, data: &[u8]) -> u256 {
        #[cfg(feature = "hash-cache")]
        if data.len() == 32 {
            // SAFETY:
            // data has length 32 so it is safe to cast it to &[u8; 32].
            let data = unsafe { &*(data.as_ptr() as *const [u8; 32]) };
            self.hash_cache_32
                .get_or_insert_ref(data, || Self::sha3(data))
        } else if data.len() == 64 {
            // SAFETY:
            // data has length 64 so it is safe to cast it to &[u8; 64].
            let data = unsafe { &*(data.as_ptr() as *const [u8; 64]) };
            self.hash_cache_64
                .get_or_insert_ref(data, || Self::sha3(data))
        } else {
            Self::sha3(data)
        }
        #[cfg(not(feature = "hash-cache"))]
        Self::sha3(data)
    }
}
