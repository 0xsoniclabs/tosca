#![allow(unused_crate_dependencies)]
mod evmc;
mod ffi;
mod interpreter;
mod types;
mod utils;

#[cfg(all(
    feature = "needs-cache",
    not(feature = "code-analysis-cache"),
    not(feature = "hash-cache"),
))]
compile_error!(
    "Feature `needs-cache` is only a helper feature and not supposed to be enabled on its own.
    Either disable it or enable one or all of `code-analysis-cache` or `hash-cache`."
);

pub use evmc_vm;
use llvm_profile_wrappers::{
    llvm_profile_enabled, llvm_profile_reset_counters, llvm_profile_set_filename,
    llvm_profile_write_file,
};
#[cfg(feature = "mock")]
pub use types::MockExecutionContextTrait;
pub use types::{ExecutionContextTrait, MockExecutionMessage, Opcode, u256};

/// Dump coverage data when compiled with `RUSTFLAGS="-C instrument-coverage"`.
/// Otherwise this is a no-op.
#[unsafe(no_mangle)]
pub extern "C" fn evmrs_dump_coverage(filename: Option<&std::ffi::c_char>) {
    if llvm_profile_enabled() != 0 {
        llvm_profile_set_filename(filename);
        llvm_profile_write_file();
        llvm_profile_reset_counters();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn evmrs_is_coverage_enabled() -> u8 {
    llvm_profile_enabled()
}

#[cfg(feature = "mimalloc")]
use mimalloc::MiMalloc;

#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;
