use sha3::{Digest, Keccak256};

#[cfg(feature = "hash-cache")]
use crate::types::Cache;
use crate::types::u256;

#[cfg(feature = "hash-cache")]
const CACHE_SIZE: usize = 1024; // value taken from evmzero

#[cfg(feature = "hash-cache")]
pub type HashCache32 = Cache<CACHE_SIZE, [u8; 32], u256>;
#[cfg(feature = "hash-cache")]
pub type HashCache64 = Cache<CACHE_SIZE, [u8; 64], u256>;

#[cfg(feature = "hash-cache")]
static HASH_CACHE_32: HashCache32 = HashCache32::new();
#[cfg(feature = "hash-cache")]
static HASH_CACHE_64: HashCache64 = HashCache64::new();

fn sha3(data: &[u8]) -> u256 {
    let mut hasher = Keccak256::new();
    hasher.update(data);
    let mut bytes = [0; 32];
    hasher.finalize_into((&mut bytes).into());

    u256::from_be_bytes(bytes)
}

pub fn hash(data: &[u8]) -> u256 {
    #[cfg(feature = "hash-cache")]
    if data.len() == 32 {
        let mut arr = [0; 32];
        arr.copy_from_slice(data);
        HASH_CACHE_32.get_or_insert(arr, || sha3(&arr))
    } else if data.len() == 64 {
        let mut arr = [0; 64];
        arr.copy_from_slice(data);
        HASH_CACHE_64.get_or_insert(arr, || sha3(&arr))
    } else {
        sha3(data)
    }
    #[cfg(not(feature = "hash-cache"))]
    sha3(data)
}
