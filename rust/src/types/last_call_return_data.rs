use std::ops::Deref;

use evmc_vm::ExecutionResult;

/// The return data of the last call, either owned by its result or borrowed from the caller.
pub enum LastCallReturnData<'a> {
    Slice(&'a [u8]),
    CallResult(ExecutionResult),
}

impl Deref for LastCallReturnData<'_> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Slice(slice) => slice,
            Self::CallResult(result) => result.output(),
        }
    }
}
