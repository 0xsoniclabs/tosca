#[cfg(feature = "alloc-reuse")]
use std::sync::Mutex;
use std::{mem::ManuallyDrop, ptr};

use evmc_vm::{Address, ExecutionResult, StatusCode, ffi::evmc_result};

/// Capacity of the pooled buffers. Outputs up to this size are served from the pool, larger ones
/// get an exactly sized allocation.
const POOL_CAPACITY: usize = 128;

/// A buffer with a capacity of exactly [`POOL_CAPACITY`]. [`Vec::with_capacity`] guarantees only
/// at least that much, but the release callback has to reconstruct the buffer from the constant.
fn new_buffer() -> Vec<u8> {
    let buf: Box<[u8]> = Box::new([0; POOL_CAPACITY]);
    buf.into_vec()
}

#[cfg(feature = "alloc-reuse")]
static REUSABLE_OUTPUT: Mutex<Vec<Vec<u8>>> = Mutex::new(Vec::new());

/// The output of an execution. Its buffer outlives the interpreter because the host owns it until
/// it calls the release callback, which either frees it or returns it to the pool. Which of the
/// two applies is encoded in the callback itself, so the buffer needs no header to describe it.
#[derive(Debug, Default)]
pub enum Output {
    #[default]
    Empty,
    /// Holds a buffer with a capacity of exactly [`POOL_CAPACITY`].
    Pooled(Vec<u8>),
    Owned(Box<[u8]>),
}

impl Output {
    pub fn new(data: &[u8]) -> Self {
        if data.is_empty() {
            return Self::Empty;
        }
        if data.len() > POOL_CAPACITY {
            std::hint::cold_path();
            return Self::Owned(Box::from(data));
        }
        let mut buf = std::cfg_select! {
            feature = "alloc-reuse" => {
                if let Some(buf) = REUSABLE_OUTPUT.lock().unwrap().pop() {
                    #[cfg(feature = "unsafe-hints")]
                    // SAFETY:
                    // Pooled buffers come from `new_buffer` and never grow beyond its capacity.
                    unsafe {
                        std::hint::assert_unchecked(buf.capacity() >= POOL_CAPACITY);
                    }
                    buf
                } else {
                    std::hint::cold_path();
                    new_buffer()
                }
            }
            _ => new_buffer(),
        };
        buf.clear();
        buf.extend_from_slice(data);
        Self::Pooled(buf)
    }

    pub fn as_slice(&self) -> &[u8] {
        match self {
            Self::Empty => &[],
            Self::Pooled(buf) => buf,
            Self::Owned(buf) => buf,
        }
    }

    /// Hands the buffer over to the result, which passes it on to the host. The host owes the call
    /// to the release callback.
    pub fn into_execution_result(
        self,
        status_code: StatusCode,
        gas_left: i64,
        gas_refund: i64,
    ) -> ExecutionResult {
        let mut this = ManuallyDrop::new(self);
        let (output_data, output_size, release) = match &mut *this {
            Self::Empty => (ptr::null(), 0, None),
            Self::Pooled(buf) => (buf.as_ptr(), buf.len(), Some(release_pooled as _)),
            Self::Owned(buf) => (buf.as_ptr(), buf.len(), Some(release_owned as _)),
        };
        // SAFETY:
        // `output_data` is null or points to `output_size` many bytes of the leaked buffer, which
        // stays valid until the release callback is called. The callback matches how the buffer
        // was allocated and is safe to call exactly once with a pointer to a copy of the result.
        unsafe {
            ExecutionResult::from_raw(evmc_result {
                status_code,
                gas_left,
                gas_refund,
                output_data,
                output_size,
                release,
                create_address: Address::default(),
                padding: [0; 4],
            })
        }
    }
}

impl Drop for Output {
    fn drop(&mut self) {
        #[cfg(feature = "alloc-reuse")]
        if let Self::Pooled(buf) = self {
            REUSABLE_OUTPUT.lock().unwrap().push(std::mem::take(buf));
        }
    }
}

/// Callback to pass across FFI, returning the output buffer to the pool.
extern "C" fn release_pooled(result: *const evmc_result) {
    // SAFETY:
    // The caller passes a pointer to a result created from an [`Output::Pooled`], so `output_data`
    // is the pointer of a buffer created by `new_buffer`, whose capacity is exactly
    // [`POOL_CAPACITY`] and never grows because only shorter outputs are pooled.
    let output_data = unsafe { (*result).output_data };
    // SAFETY:
    // See above.
    let buf = unsafe { Vec::from_raw_parts(output_data.cast_mut(), 0, POOL_CAPACITY) };
    std::cfg_select! {
        feature = "alloc-reuse" => REUSABLE_OUTPUT.lock().unwrap().push(buf),
        _ => drop(buf),
    };
}

/// Callback to pass across FFI, de-allocating the output buffer.
extern "C" fn release_owned(result: *const evmc_result) {
    // SAFETY:
    // The caller passes a pointer to a result created from an [`Output::Owned`], so `output_data`
    // and `output_size` are the pointer and length of a boxed slice.
    let result = unsafe { *result };
    // SAFETY:
    // See above.
    drop(unsafe {
        Box::from_raw(ptr::slice_from_raw_parts_mut(
            result.output_data.cast_mut(),
            result.output_size,
        ))
    });
}
