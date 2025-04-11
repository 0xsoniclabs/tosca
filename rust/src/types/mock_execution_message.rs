use std::ptr;

use evmc_vm::{Address, ExecutionMessage, MessageKind, Uint256, ffi::evmc_message};

use crate::types::u256;

/// The same as ExecutionMessage but with `pub` fields for easier testing.
#[derive(Debug)]
pub struct MockExecutionMessage {
    pub kind: MessageKind,
    pub flags: u32,
    pub depth: i32,
    pub gas: i64,
    pub recipient: Address,
    pub sender: Address,
    pub input: Option<&'static [u8]>,
    pub value: Uint256,
    pub create2_salt: Uint256,
    pub code_address: Address,
    pub code: Option<&'static [u8]>,
    pub code_hash: Option<&'static Uint256>,
}

impl MockExecutionMessage {
    pub const DEFAULT_INIT_GAS: u64 = i64::MAX as u64;

    pub fn to_evmc_message(&self) -> evmc_message {
        evmc_message {
            kind: self.kind,
            flags: self.flags,
            depth: self.depth,
            gas: self.gas,
            recipient: self.recipient,
            sender: self.sender,
            input_data: self.input.map(<[u8]>::as_ptr).unwrap_or(ptr::null()),
            input_size: self.input.map(<[u8]>::len).unwrap_or_default(),
            value: self.value,
            create2_salt: self.create2_salt,
            code_address: self.code_address,
            code: self.code.map(<[u8]>::as_ptr).unwrap_or(ptr::null()),
            code_size: self.code.map(<[u8]>::len).unwrap_or_default(),
            code_hash: self.code_hash.map(|h| h as *const _).unwrap_or(ptr::null()),
        }
    }
}

impl Default for MockExecutionMessage {
    fn default() -> Self {
        MockExecutionMessage {
            kind: MessageKind::EVMC_CALL,
            flags: 0,
            depth: 0,
            gas: Self::DEFAULT_INIT_GAS as i64,
            recipient: u256::ZERO.into(),
            sender: u256::ZERO.into(),
            input: None,
            value: u256::ZERO.into(),
            create2_salt: u256::ZERO.into(),
            code_address: u256::ZERO.into(),
            code: None,
            code_hash: None,
        }
    }
}

impl From<MockExecutionMessage> for ExecutionMessage<'_> {
    fn from(value: MockExecutionMessage) -> Self {
        Self {
            kind: value.kind,
            flags: value.flags,
            depth: value.depth,
            gas: value.gas,
            recipient: value.recipient,
            sender: value.sender,
            input: value.input,
            value: value.value,
            create2_salt: value.create2_salt,
            code_address: value.code_address,
            code: value.code,
            code_hash: value.code_hash.copied(),
        }
    }
}
