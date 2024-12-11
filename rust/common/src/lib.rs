#[cfg(not(feature = "custom-evmc"))]
pub extern crate evmc_vm_tosca as evmc_vm;
#[cfg(feature = "custom-evmc")]
pub extern crate evmc_vm_tosca_refactor as evmc_vm;

mod amount;
mod execution_context;
mod mock_execution_message;
pub use amount::u256;
pub mod opcode;
pub use execution_context::*;
pub use mock_execution_message::*;
