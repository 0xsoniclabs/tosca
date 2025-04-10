mod evmc_vm;
mod steppable_evmc_vm;
use std::slice;

pub use evmc_vm::EVMC_CAPABILITY;

/// This type is indented to be used to enforce the correct lifetime for references created from
/// pointers obtained via FFI. It is assumed that the lifetime of the pointer is bound by the
/// lifetime of the token.
struct LifetimeToken;

/// # Safety
/// ptr must be non-null and valid for reads for the lifetime of the borrow of token.
#[allow(clippy::needless_lifetimes)] // use explicit lifetimes for easier understanding
unsafe fn ref_from_ptr_scoped<'s, T>(ptr: *const T, _token: &'s LifetimeToken) -> &'s T {
    // SAFETY:
    // ptr is non-null and valid for reads for the lifetime of the borrow of token
    unsafe { &*ptr }
}

/// # Safety
/// ptr must be non-null and valid for reads and writes for the lifetime of the borrow of token.
#[allow(clippy::needless_lifetimes)] // use explicit lifetimes for easier understanding
#[allow(clippy::mut_from_ref)] // false positive
unsafe fn ref_mut_from_ptr_scoped<'s, T>(ptr: *mut T, _token: &'s LifetimeToken) -> &'s mut T {
    // SAFETY:
    // ptr is non-null and valid for reads and writes for the lifetime of the borrow of token
    unsafe { &mut *ptr }
}

/// # Safety
/// ptr must be non-null and valid for reads for `len * mem::size_of::<T>()` many bytes for the
/// lifetime of the borrow of token.
#[allow(clippy::needless_lifetimes)] // use explicit lifetimes for easier understanding
unsafe fn slice_from_raw_parts_scoped<'s, T>(
    ptr: *const T,
    len: usize,
    _token: &'s LifetimeToken,
) -> &'s [T] {
    // SAFETY:
    // ptr is non-null and valid for reads for the lifetime of the borrow of token
    unsafe { slice::from_raw_parts(ptr, len) }
}
