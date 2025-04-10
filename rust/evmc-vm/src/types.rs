/// EVMC address
pub type Address = evmc_sys::evmc_address;

/// EVMC big-endian 256-bit integer
pub type Uint256 = evmc_sys::evmc_uint256be;

/// EVMC call kind.
pub type MessageKind = evmc_sys::evmc_call_kind;

/// EVMC message (call) flags.
pub type MessageFlags = evmc_sys::evmc_flags;

/// EVMC status code.
pub type StatusCode = evmc_sys::evmc_status_code;

/// EVMC step status code.
pub type StepStatusCode = evmc_sys::evmc_step_status_code;

/// EVMC access status.
pub type AccessStatus = evmc_sys::evmc_access_status;

/// EVMC storage status.
pub type StorageStatus = evmc_sys::evmc_storage_status;

/// EVMC VM revision.
pub type Revision = evmc_sys::evmc_revision;
