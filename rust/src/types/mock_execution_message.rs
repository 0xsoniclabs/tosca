use std::ptr;

use evmc_vm::{Address, ExecutionMessage, MessageKind, Uint256, ffi::evmc_message};

use crate::types::u256;

/// A test variant of [`ExecutionMessage`] with `'static` slices and a `'static` reference as code
/// hash, so that [`Self::to_evmc_message`] can point to them.
#[derive(Debug, Clone)]
pub struct MockExecutionMessage {
    pub kind: MessageKind,
    pub flags: u32,
    pub depth: i32,
    pub gas: i64,
    pub recipient: Address,
    pub sender: Address,
    pub input: &'static [u8],
    pub value: Uint256,
    pub create2_salt: Uint256,
    pub code_address: Address,
    pub code: &'static [u8],
    pub code_hash: Option<&'static Uint256>,
}

impl MockExecutionMessage {
    /// The gas of the [`Default`] message.
    pub const DEFAULT_INIT_GAS: u64 = i64::MAX.cast_unsigned();

    /// Converts the message into its FFI representation. Empty slices and a missing code hash
    /// become null pointers.
    pub fn to_evmc_message(&self) -> evmc_message {
        evmc_message {
            kind: self.kind,
            flags: self.flags,
            depth: self.depth,
            gas: self.gas,
            recipient: self.recipient,
            sender: self.sender,
            input_data: if self.input.is_empty() {
                ptr::null()
            } else {
                self.input.as_ptr()
            },
            input_size: self.input.len(),
            value: self.value,
            create2_salt: self.create2_salt,
            code_address: self.code_address,
            code: if self.code.is_empty() {
                ptr::null()
            } else {
                self.code.as_ptr()
            },
            code_size: self.code.len(),
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
            gas: Self::DEFAULT_INIT_GAS.cast_signed(),
            recipient: u256::ZERO.into(),
            sender: u256::ZERO.into(),
            input: &[],
            value: u256::ZERO.into(),
            create2_salt: u256::ZERO.into(),
            code_address: u256::ZERO.into(),
            code: &[],
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

#[cfg(test)]
mod tests {
    use super::*;

    static CODE_HASH: Uint256 = Uint256 { bytes: [3; 32] };

    fn message(
        input: &'static [u8],
        code: &'static [u8],
        code_hash: Option<&'static Uint256>,
    ) -> MockExecutionMessage {
        MockExecutionMessage {
            kind: MessageKind::EVMC_CREATE,
            flags: 1,
            depth: 2,
            gas: 3,
            recipient: u256::from(4u8).into(),
            sender: u256::from(5u8).into(),
            input,
            value: u256::from(6u8).into(),
            create2_salt: u256::from(7u8).into(),
            code_address: u256::from(8u8).into(),
            code,
            code_hash,
        }
    }

    #[test]
    fn to_evmc_message_copies_all_fields() {
        let message = message(&[1], &[2], Some(&CODE_HASH));
        let evmc_message = message.to_evmc_message();
        assert_eq!(evmc_message.kind, message.kind);
        assert_eq!(evmc_message.flags, message.flags);
        assert_eq!(evmc_message.depth, message.depth);
        assert_eq!(evmc_message.gas, message.gas);
        assert_eq!(evmc_message.recipient, message.recipient);
        assert_eq!(evmc_message.sender, message.sender);
        assert_eq!(evmc_message.input_data, message.input.as_ptr());
        assert_eq!(evmc_message.input_size, message.input.len());
        assert_eq!(evmc_message.value, message.value);
        assert_eq!(evmc_message.create2_salt, message.create2_salt);
        assert_eq!(evmc_message.code_address, message.code_address);
        assert_eq!(evmc_message.code, message.code.as_ptr());
        assert_eq!(evmc_message.code_size, message.code.len());
        assert_eq!(evmc_message.code_hash, &raw const CODE_HASH);
    }

    #[test]
    fn to_evmc_message_converts_empty_slices_and_missing_code_hash_to_null() {
        let evmc_message = message(&[], &[], None).to_evmc_message();
        assert!(evmc_message.input_data.is_null());
        assert_eq!(evmc_message.input_size, 0);
        assert!(evmc_message.code.is_null());
        assert_eq!(evmc_message.code_size, 0);
        assert!(evmc_message.code_hash.is_null());
    }

    #[test]
    fn default_is_a_call_with_default_init_gas_and_zeroed_fields() {
        let message = MockExecutionMessage::default();
        assert_eq!(message.kind, MessageKind::EVMC_CALL);
        assert_eq!(message.flags, 0);
        assert_eq!(message.depth, 0);
        assert_eq!(
            message.gas,
            MockExecutionMessage::DEFAULT_INIT_GAS.cast_signed()
        );
        assert_eq!(message.recipient, Address::default());
        assert_eq!(message.sender, Address::default());
        assert!(message.input.is_empty());
        assert_eq!(message.value, Uint256::default());
        assert_eq!(message.create2_salt, Uint256::default());
        assert_eq!(message.code_address, Address::default());
        assert!(message.code.is_empty());
        assert_eq!(message.code_hash, None);
    }

    #[test]
    fn execution_message_from_mock_execution_message_copies_all_fields() {
        let mock = message(&[1], &[2], Some(&CODE_HASH));
        let message = ExecutionMessage::from(mock.clone());
        assert_eq!(message.kind, mock.kind);
        assert_eq!(message.flags, mock.flags);
        assert_eq!(message.depth, mock.depth);
        assert_eq!(message.gas, mock.gas);
        assert_eq!(message.recipient, mock.recipient);
        assert_eq!(message.sender, mock.sender);
        assert_eq!(message.input, mock.input);
        assert_eq!(message.value, mock.value);
        assert_eq!(message.create2_salt, mock.create2_salt);
        assert_eq!(message.code_address, mock.code_address);
        assert_eq!(message.code, mock.code);
        assert_eq!(message.code_hash, Some(CODE_HASH));
    }
}
