// EVMC: Ethereum Client-VM Connector API.
// Copyright 2019 The EVMC Authors.
// Licensed under the Apache License, Version 2.0.

//! Rust bindings for EVMC (Ethereum Client-VM Connector API).
//!
//! This crate documents how to use certain data types.

#![allow(clippy::too_many_arguments)]

mod container;
mod types;

use std::{ptr, slice};

pub use container::{EvmcContainer, SteppableEvmcContainer};
pub use evmc_sys as ffi;
pub use types::*;

/// Trait EVMC VMs have to implement.
pub trait EvmcVm {
    /// This is called once at initialisation time.
    fn init() -> Self;

    /// This is called for each supplied option.
    fn set_option(&mut self, _: &str, _: &str) -> Result<(), SetOptionError> {
        Ok(())
    }

    /// This is called for every incoming message.
    fn execute<'a>(
        &self,
        revision: Revision,
        code: &'a [u8],
        message: &'a ExecutionMessage,
        context: Option<&'a mut ExecutionContext<'a>>,
    ) -> ExecutionResult;
}

pub trait SteppableEvmcVm {
    fn step_n<'a>(
        &self,
        revision: Revision,
        code: &'a [u8],
        message: &'a ExecutionMessage,
        context: Option<&'a mut ExecutionContext<'a>>,
        status: StepStatusCode,
        pc: u64,
        gas_refunds: i64,
        stack: &'a mut [Uint256],
        memory: &'a mut [u8],
        last_call_result_data: &'a mut [u8],
        steps: i32,
    ) -> StepResult;
}

/// Error codes for set_option.
#[derive(Debug)]
pub enum SetOptionError {
    InvalidKey,
    InvalidValue,
}

/// EVMC result structure.
#[derive(Debug)]
pub struct ExecutionResult {
    pub status_code: StatusCode,
    pub gas_left: i64,
    pub gas_refund: i64,
    pub output: Option<Vec<u8>>,
    pub create_address: Option<Address>,
}

#[derive(Debug)]
pub struct StepResult {
    pub step_status_code: StepStatusCode,
    pub status_code: StatusCode,
    pub revision: Revision,
    pub pc: u64,
    pub gas_left: i64,
    pub gas_refund: i64,
    pub output: Option<Vec<u8>>,
    pub stack: Vec<Uint256>,
    pub memory: Vec<u8>,
    pub last_call_return_data: Option<Vec<u8>>,
}

/// EVMC execution message structure.
#[derive(Debug)]
pub struct ExecutionMessage {
    pub kind: MessageKind,
    pub flags: u32,
    pub depth: i32,
    pub gas: i64,
    pub recipient: Address,
    pub sender: Address,
    pub input: Option<Vec<u8>>,
    pub value: Uint256,
    pub create2_salt: Uint256,
    pub code_address: Address,
    pub code: Option<Vec<u8>>,
    pub code_hash: Option<Uint256>,
}

/// EVMC transaction context structure.
pub type ExecutionTxContext = ffi::evmc_tx_context;

/// EVMC context structure. Exposes the EVMC host functions, message data, and transaction context
/// to the executing VM.
pub struct ExecutionContext<'a> {
    host: &'a ffi::evmc_host_interface,
    context: *mut ffi::evmc_host_context,
    tx_context: Option<ExecutionTxContext>,
}

impl<'a> ExecutionContext<'a> {
    pub fn new(host: &'a ffi::evmc_host_interface, context: *mut ffi::evmc_host_context) -> Self {
        ExecutionContext {
            host,
            context,
            tx_context: None,
        }
    }

    /// Retrieve the transaction context.
    pub fn get_tx_context(&mut self) -> &ExecutionTxContext {
        let get_tx_context = self.host.get_tx_context.unwrap();
        let context = self.context;
        self.tx_context
            .get_or_insert_with(|| unsafe { get_tx_context(context) })
    }

    /// Check if an account exists.
    pub fn account_exists(&self, address: &Address) -> bool {
        unsafe { self.host.account_exists.unwrap()(self.context, address) }
    }

    /// Read from a storage key.
    pub fn get_storage(&self, address: &Address, key: &Uint256) -> Uint256 {
        unsafe { self.host.get_storage.unwrap()(self.context, address, key) }
    }

    /// Set value of a storage key.
    pub fn set_storage(
        &mut self,
        address: &Address,
        key: &Uint256,
        value: &Uint256,
    ) -> StorageStatus {
        unsafe { self.host.set_storage.unwrap()(self.context, address, key, value) }
    }

    /// Get balance of an account.
    pub fn get_balance(&self, address: &Address) -> Uint256 {
        unsafe { self.host.get_balance.unwrap()(self.context, address) }
    }

    /// Get code size of an account.
    pub fn get_code_size(&self, address: &Address) -> usize {
        unsafe { self.host.get_code_size.unwrap()(self.context, address) }
    }

    /// Get code hash of an account.
    pub fn get_code_hash(&self, address: &Address) -> Uint256 {
        unsafe { self.host.get_code_hash.unwrap()(self.context, address) }
    }

    /// Copy code of an account.
    pub fn copy_code(&self, address: &Address, code_offset: usize, buffer: &mut [u8]) -> usize {
        unsafe {
            self.host.copy_code.unwrap()(
                self.context,
                address,
                code_offset,
                // FIXME: ensure that alignment of the array elements is OK
                buffer.as_mut_ptr(),
                buffer.len(),
            )
        }
    }

    /// Self-destruct the current account.
    pub fn selfdestruct(&mut self, address: &Address, beneficiary: &Address) -> bool {
        unsafe { self.host.selfdestruct.unwrap()(self.context, address, beneficiary) }
    }

    /// Call to another account.
    pub fn call(&mut self, message: &ExecutionMessage) -> ExecutionResult {
        // There is no need to make any kind of copies here, because the caller
        // won't go out of scope and ensures these pointers remain valid.
        let input_size = message.input.as_ref().map(Vec::len).unwrap_or_default();
        let input_data = message
            .input
            .as_ref()
            .map(Vec::as_ptr)
            .unwrap_or(ptr::null());
        let code_size = message.code.as_ref().map(Vec::len).unwrap_or_default();
        let code_data = message
            .code
            .as_ref()
            .map(Vec::as_ptr)
            .unwrap_or(ptr::null());
        // Cannot use a nice from trait here because that complicates memory management,
        // evmc_message doesn't have a release() method we could abstract it with.
        let message = ffi::evmc_message {
            kind: message.kind,
            flags: message.flags,
            depth: message.depth,
            gas: message.gas,
            recipient: message.recipient,
            sender: message.sender,
            input_data,
            input_size,
            value: message.value,
            create2_salt: message.create2_salt,
            code_address: message.code_address,
            code: code_data,
            code_size,
            code_hash: ptr::null(),
        };
        unsafe { self.host.call.unwrap()(self.context, &message).into() }
    }

    /// Get block hash of an account.
    pub fn get_block_hash(&self, num: i64) -> Uint256 {
        unsafe { self.host.get_block_hash.unwrap()(self.context, num) }
    }

    /// Emit a log.
    pub fn emit_log(&mut self, address: &Address, data: &[u8], topics: &[Uint256]) {
        unsafe {
            self.host.emit_log.unwrap()(
                self.context,
                address,
                // FIXME: ensure that alignment of the array elements is OK
                data.as_ptr(),
                data.len(),
                topics.as_ptr(),
                topics.len(),
            )
        }
    }

    /// Access an account.
    pub fn access_account(&mut self, address: &Address) -> AccessStatus {
        unsafe { self.host.access_account.unwrap()(self.context, address) }
    }

    /// Access a storage key.
    pub fn access_storage(&mut self, address: &Address, key: &Uint256) -> AccessStatus {
        unsafe { self.host.access_storage.unwrap()(self.context, address, key) }
    }

    /// Read from a transient storage key.
    pub fn get_transient_storage(&self, address: &Address, key: &Uint256) -> Uint256 {
        unsafe { self.host.get_transient_storage.unwrap()(self.context, address, key) }
    }

    /// Set value of a transient storage key.
    pub fn set_transient_storage(&mut self, address: &Address, key: &Uint256, value: &Uint256) {
        unsafe { self.host.set_transient_storage.unwrap()(self.context, address, key, value) }
    }
}

impl From<ffi::evmc_result> for ExecutionResult {
    fn from(result: ffi::evmc_result) -> Self {
        let ret = Self {
            status_code: result.status_code,
            gas_left: result.gas_left,
            gas_refund: result.gas_refund,
            output: if result.output_data.is_null() {
                assert_eq!(result.output_size, 0);
                None
            } else if result.output_size == 0 {
                None
            } else {
                Some(from_buf_raw::<u8>(result.output_data, result.output_size))
            },
            // Consider it is always valid.
            create_address: Some(result.create_address),
        };

        // Release allocated ffi struct.
        if result.release.is_some() {
            unsafe {
                result.release.unwrap()(&result);
            }
        }

        ret
    }
}

fn allocate_output_data<T>(output: Option<&Vec<T>>) -> (*const T, usize) {
    if let Some(buf) = output {
        if !buf.is_empty() {
            let buf_len = buf.len();

            // Manually allocate heap memory for the new home of the output buffer.
            let memlayout =
                std::alloc::Layout::from_size_align(buf_len * size_of::<T>(), align_of::<T>())
                    .expect("Bad layout");
            let new_buf = unsafe { std::alloc::alloc(memlayout) as *mut T };
            unsafe {
                // Copy the data into the allocated buffer.
                std::ptr::copy(buf.as_ptr(), new_buf, buf_len);
            }

            return (new_buf as *const T, buf_len);
        }
    }
    (std::ptr::null(), 0)
}

unsafe fn deallocate_output_data<T>(ptr: *const T, size: usize) {
    if !ptr.is_null() {
        let buf_layout =
            std::alloc::Layout::from_size_align(size * size_of::<T>(), align_of::<T>())
                .expect("Bad layout");
        std::alloc::dealloc(ptr as *mut u8, buf_layout);
    }
}

impl From<ExecutionResult> for ffi::evmc_result {
    fn from(value: ExecutionResult) -> Self {
        let (buffer, len) = allocate_output_data(value.output.as_ref());
        Self {
            status_code: value.status_code,
            gas_left: value.gas_left,
            gas_refund: value.gas_refund,
            output_data: buffer,
            output_size: len,
            release: Some(release_stack_result),
            create_address: value.create_address.unwrap_or_default(),
            padding: [0u8; 4],
        }
    }
}

impl From<StepResult> for ffi::evmc_step_result {
    fn from(value: StepResult) -> Self {
        let (output_data, output_size) = allocate_output_data(value.output.as_ref());
        let (stack, stack_size) = allocate_output_data(Some(&value.stack));
        let (memory, memory_size) = allocate_output_data(Some(&value.memory));
        let (last_call_return_data, last_call_return_data_size) =
            allocate_output_data(value.last_call_return_data.as_ref());

        Self {
            step_status_code: value.step_status_code,
            status_code: value.status_code,
            revision: value.revision,
            pc: value.pc,
            gas_left: value.gas_left,
            gas_refund: value.gas_refund,
            output_data,
            output_size,
            stack,
            stack_size,
            memory,
            memory_size,
            last_call_return_data,
            last_call_return_data_size,
            release: Some(release_stack_step_result),
        }
    }
}

impl From<ffi::evmc_step_result> for StepResult {
    fn from(value: ffi::evmc_step_result) -> Self {
        let ret = Self {
            step_status_code: value.step_status_code,
            status_code: value.status_code,
            revision: value.revision,
            pc: value.pc,
            gas_left: value.gas_left,
            gas_refund: value.gas_refund,
            output: if value.output_data.is_null() || value.output_size == 0 {
                None
            } else {
                Some(Vec::from(unsafe {
                    slice::from_raw_parts(value.output_data as *mut u8, value.output_size)
                }))
            },
            stack: if value.stack.is_null() || value.stack_size == 0 {
                Vec::new()
            } else {
                unsafe {
                    Vec::from(slice::from_raw_parts(
                        value.stack as *mut _,
                        value.stack_size,
                    ))
                }
            },
            memory: if value.memory.is_null() || value.memory_size == 0 {
                Vec::new()
            } else {
                unsafe {
                    Vec::from(slice::from_raw_parts(
                        value.memory as *mut _,
                        value.memory_size,
                    ))
                }
            },
            last_call_return_data: if value.last_call_return_data.is_null()
                || value.last_call_return_data_size == 0
            {
                None
            } else {
                Some(unsafe {
                    Vec::from(slice::from_raw_parts(
                        value.last_call_return_data as *mut _,
                        value.last_call_return_data_size,
                    ))
                })
            },
        };

        // If release function is provided, use it to release resources.
        if let Some(release) = value.release {
            unsafe {
                release(&value);
            }
        }

        ret
    }
}

/// Callback to pass across FFI, de-allocating the optional output_data.
extern "C" fn release_stack_result(result: *const ffi::evmc_result) {
    unsafe {
        let tmp = *result;
        deallocate_output_data(tmp.output_data, tmp.output_size);
    }
}

/// Callback to pass across FFI, de-allocating all allocated fields.
extern "C" fn release_stack_step_result(result: *const ffi::evmc_step_result) {
    unsafe {
        let tmp = *result;
        deallocate_output_data(tmp.output_data, tmp.output_size);
        deallocate_output_data(tmp.stack, tmp.stack_size);
        deallocate_output_data(tmp.memory, tmp.memory_size);
        deallocate_output_data(tmp.last_call_return_data, tmp.last_call_return_data_size);
    }
}

impl From<&ffi::evmc_message> for ExecutionMessage {
    fn from(message: &ffi::evmc_message) -> Self {
        ExecutionMessage {
            kind: message.kind,
            flags: message.flags,
            depth: message.depth,
            gas: message.gas,
            recipient: message.recipient,
            sender: message.sender,
            input: if message.input_data.is_null() || message.input_size == 0 {
                None
            } else {
                Some(from_buf_raw::<u8>(message.input_data, message.input_size))
            },
            value: message.value,
            create2_salt: message.create2_salt,
            code_address: message.code_address,
            code: if message.code.is_null() || message.code_size == 0 {
                None
            } else {
                Some(from_buf_raw::<u8>(message.code, message.code_size))
            },
            code_hash: if message.code_hash.is_null() {
                None
            } else {
                Some(unsafe { *message.code_hash })
            },
        }
    }
}

fn from_buf_raw<T>(ptr: *const T, size: usize) -> Vec<T> {
    // Pre-allocate a vector.
    let mut buf = Vec::with_capacity(size);
    unsafe {
        // Copy from the C buffer to the vec's buffer.
        std::ptr::copy(ptr, buf.as_mut_ptr(), size);
        // Set the len of the vec manually.
        buf.set_len(size);
    }
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test-specific helper to dispose of execution results in unit tests
    extern "C" fn test_result_dispose(result: *const ffi::evmc_result) {
        unsafe {
            if !result.is_null() {
                let owned = *result;
                Vec::from_raw_parts(
                    owned.output_data as *mut u8,
                    owned.output_size,
                    owned.output_size,
                );
            }
        }
    }

    #[test]
    fn result_from_ffi() {
        let f = ffi::evmc_result {
            status_code: StatusCode::EVMC_SUCCESS,
            gas_left: 1337,
            gas_refund: 21,
            output_data: Box::into_raw(Box::new([0xde, 0xad, 0xbe, 0xef])) as *const u8,
            output_size: 4,
            release: Some(test_result_dispose),
            create_address: Address { bytes: [0u8; 20] },
            padding: [0u8; 4],
        };

        let r: ExecutionResult = f.into();

        assert_eq!(r.status_code, StatusCode::EVMC_SUCCESS);
        assert_eq!(r.gas_left, 1337);
        assert_eq!(r.gas_refund, 21);
        assert!(r.output.is_some());
        assert_eq!(r.output.unwrap().len(), 4);
        assert!(r.create_address.is_some());
    }

    #[test]
    fn result_into_stack_ffi() {
        let r = ExecutionResult {
            status_code: StatusCode::EVMC_FAILURE,
            gas_left: 420,
            gas_refund: 21,
            output: Some(vec![0xc0, 0xff, 0xee, 0x71, 0x75]),
            create_address: None,
        };

        let f: ffi::evmc_result = r.into();
        unsafe {
            assert_eq!(f.status_code, StatusCode::EVMC_FAILURE);
            assert_eq!(f.gas_left, 420);
            assert_eq!(f.gas_refund, 21);
            assert!(!f.output_data.is_null());
            assert_eq!(f.output_size, 5);
            assert_eq!(
                std::slice::from_raw_parts(f.output_data, 5) as &[u8],
                &[0xc0, 0xff, 0xee, 0x71, 0x75]
            );
            assert_eq!(f.create_address.bytes, [0u8; 20]);
            if f.release.is_some() {
                f.release.unwrap()(&f);
            }
        }
    }

    #[test]
    fn result_into_stack_ffi_empty_data() {
        let r = ExecutionResult {
            status_code: StatusCode::EVMC_FAILURE,
            gas_left: 420,
            gas_refund: 21,
            output: None,
            create_address: None,
        };

        let f: ffi::evmc_result = r.into();
        unsafe {
            assert_eq!(f.status_code, StatusCode::EVMC_FAILURE);
            assert_eq!(f.gas_left, 420);
            assert_eq!(f.gas_refund, 21);
            assert!(f.output_data.is_null());
            assert_eq!(f.output_size, 0);
            assert_eq!(f.create_address.bytes, [0u8; 20]);
            if f.release.is_some() {
                f.release.unwrap()(&f);
            }
        }
    }

    #[test]
    fn message_from_ffi() {
        let recipient = Address { bytes: [32u8; 20] };
        let sender = Address { bytes: [128u8; 20] };
        let value = Uint256 { bytes: [0u8; 32] };
        let create2_salt = Uint256 { bytes: [255u8; 32] };
        let code_address = Address { bytes: [64u8; 20] };

        let msg = ffi::evmc_message {
            kind: MessageKind::EVMC_CALL,
            flags: 44,
            depth: 66,
            gas: 4466,
            recipient,
            sender,
            input_data: std::ptr::null(),
            input_size: 0,
            value,
            create2_salt,
            code_address,
            code: std::ptr::null(),
            code_size: 0,
            code_hash: std::ptr::null(),
        };

        let ret: ExecutionMessage = (&msg).into();

        assert_eq!(ret.kind, msg.kind);
        assert_eq!(ret.flags, msg.flags);
        assert_eq!(ret.depth, msg.depth);
        assert_eq!(ret.gas, msg.gas);
        assert_eq!(ret.recipient, msg.recipient);
        assert_eq!(ret.sender, msg.sender);
        assert!(ret.input.is_none());
        assert_eq!(ret.value, msg.value);
        assert_eq!(ret.create2_salt, msg.create2_salt);
        assert_eq!(ret.code_address, msg.code_address);
        assert!(ret.code.is_none());
    }

    #[test]
    fn message_from_ffi_with_input() {
        let input = vec![0xc0, 0xff, 0xee];
        let recipient = Address { bytes: [32u8; 20] };
        let sender = Address { bytes: [128u8; 20] };
        let value = Uint256 { bytes: [0u8; 32] };
        let create2_salt = Uint256 { bytes: [255u8; 32] };
        let code_address = Address { bytes: [64u8; 20] };

        let msg = ffi::evmc_message {
            kind: MessageKind::EVMC_CALL,
            flags: 44,
            depth: 66,
            gas: 4466,
            recipient,
            sender,
            input_data: input.as_ptr(),
            input_size: input.len(),
            value,
            create2_salt,
            code_address,
            code: std::ptr::null(),
            code_size: 0,
            code_hash: std::ptr::null(),
        };

        let ret: ExecutionMessage = (&msg).into();

        assert_eq!(ret.kind, msg.kind);
        assert_eq!(ret.flags, msg.flags);
        assert_eq!(ret.depth, msg.depth);
        assert_eq!(ret.gas, msg.gas);
        assert_eq!(ret.recipient, msg.recipient);
        assert_eq!(ret.sender, msg.sender);
        assert!(ret.input.is_some());
        assert_eq!(ret.input.unwrap(), input);
        assert_eq!(ret.value, msg.value);
        assert_eq!(ret.create2_salt, msg.create2_salt);
        assert_eq!(ret.code_address, msg.code_address);
        assert!(ret.code.is_none());
    }

    #[test]
    fn message_from_ffi_with_code() {
        let recipient = Address { bytes: [32u8; 20] };
        let sender = Address { bytes: [128u8; 20] };
        let value = Uint256 { bytes: [0u8; 32] };
        let create2_salt = Uint256 { bytes: [255u8; 32] };
        let code_address = Address { bytes: [64u8; 20] };
        let code = vec![0x5f, 0x5f, 0xfd];

        let msg = ffi::evmc_message {
            kind: MessageKind::EVMC_CALL,
            flags: 44,
            depth: 66,
            gas: 4466,
            recipient,
            sender,
            input_data: std::ptr::null(),
            input_size: 0,
            value,
            create2_salt,
            code_address,
            code: code.as_ptr(),
            code_size: code.len(),
            code_hash: std::ptr::null(),
        };

        let ret: ExecutionMessage = (&msg).into();

        assert_eq!(ret.kind, msg.kind);
        assert_eq!(ret.flags, msg.flags);
        assert_eq!(ret.depth, msg.depth);
        assert_eq!(ret.gas, msg.gas);
        assert_eq!(ret.recipient, msg.recipient);
        assert_eq!(ret.sender, msg.sender);
        assert!(ret.input.is_none());
        assert_eq!(ret.value, msg.value);
        assert_eq!(ret.create2_salt, msg.create2_salt);
        assert_eq!(ret.code_address, msg.code_address);
        assert!(ret.code.is_some());
        assert_eq!(ret.code.unwrap(), code);
    }

    unsafe extern "C" fn get_dummy_tx_context(
        _context: *mut ffi::evmc_host_context,
    ) -> ffi::evmc_tx_context {
        ffi::evmc_tx_context {
            tx_gas_price: Uint256 { bytes: [0u8; 32] },
            tx_origin: Address { bytes: [0u8; 20] },
            block_coinbase: Address { bytes: [0u8; 20] },
            block_number: 42,
            block_timestamp: 235117,
            block_gas_limit: 105023,
            block_prev_randao: Uint256 { bytes: [0xaa; 32] },
            chain_id: Uint256::default(),
            block_base_fee: Uint256::default(),
            blob_base_fee: Uint256::default(),
            blob_hashes: std::ptr::null(),
            blob_hashes_count: 0,
            initcodes: std::ptr::null(),
            initcodes_count: 0,
        }
    }

    unsafe extern "C" fn get_dummy_code_size(
        _context: *mut ffi::evmc_host_context,
        _addr: *const Address,
    ) -> usize {
        105023_usize
    }

    unsafe extern "C" fn execute_call(
        _context: *mut ffi::evmc_host_context,
        _msg: *const ffi::evmc_message,
    ) -> ffi::evmc_result {
        // Some dumb validation for testing.
        let msg = unsafe { *_msg };
        let success = if msg.input_size != 0 && msg.input_data.is_null() {
            false
        } else {
            msg.input_size != 0 || msg.input_data.is_null()
        };

        ffi::evmc_result {
            status_code: if success {
                StatusCode::EVMC_SUCCESS
            } else {
                StatusCode::EVMC_INTERNAL_ERROR
            },
            gas_left: 2,
            gas_refund: 0,
            // NOTE: we are passing the input pointer here, but for testing the lifetime is ok
            output_data: msg.input_data,
            output_size: msg.input_size,
            release: None,
            create_address: Address::default(),
            padding: [0u8; 4],
        }
    }

    // Update these when needed for tests
    fn get_dummy_host_interface() -> ffi::evmc_host_interface {
        ffi::evmc_host_interface {
            account_exists: None,
            get_storage: None,
            set_storage: None,
            get_balance: None,
            get_code_size: Some(get_dummy_code_size),
            get_code_hash: None,
            copy_code: None,
            selfdestruct: None,
            call: Some(execute_call),
            get_tx_context: Some(get_dummy_tx_context),
            get_block_hash: None,
            emit_log: None,
            access_account: None,
            access_storage: None,
            get_transient_storage: None,
            set_transient_storage: None,
        }
    }

    #[test]
    fn execution_context() {
        let host_context = std::ptr::null_mut();
        let host_interface = get_dummy_host_interface();
        let mut exe_context = ExecutionContext::new(&host_interface, host_context);
        let a = exe_context.get_tx_context();

        let b = unsafe { get_dummy_tx_context(host_context) };

        assert_eq!(a.block_gas_limit, b.block_gas_limit);
        assert_eq!(a.block_timestamp, b.block_timestamp);
        assert_eq!(a.block_number, b.block_number);
    }

    #[test]
    fn get_code_size() {
        // This address is useless. Just a dummy parameter for the interface function.
        let test_addr = Address { bytes: [0u8; 20] };
        let host = get_dummy_host_interface();
        let host_context = std::ptr::null_mut();

        let exe_context = ExecutionContext::new(&host, host_context);

        let a: usize = 105023;
        let b = exe_context.get_code_size(&test_addr);

        assert_eq!(a, b);
    }

    #[test]
    fn test_call_empty_data() {
        // This address is useless. Just a dummy parameter for the interface function.
        let test_addr = Address::default();
        let host = get_dummy_host_interface();
        let host_context = std::ptr::null_mut();
        let mut exe_context = ExecutionContext::new(&host, host_context);

        let message = ExecutionMessage {
            kind: MessageKind::EVMC_CALL,
            flags: 0,
            depth: 0,
            gas: 6566,
            recipient: test_addr,
            sender: test_addr,
            input: None,
            value: Uint256::default(),
            create2_salt: Uint256::default(),
            code_address: test_addr,
            code: None,
            code_hash: None,
        };

        let b = exe_context.call(&message);

        assert_eq!(b.status_code, StatusCode::EVMC_SUCCESS);
        assert_eq!(b.gas_left, 2);
        assert!(b.output.is_none());
        assert!(b.create_address.is_some());
        assert_eq!(b.create_address.unwrap(), Address::default());
    }

    #[test]
    fn test_call_with_data() {
        // This address is useless. Just a dummy parameter for the interface function.
        let test_addr = Address::default();
        let host = get_dummy_host_interface();
        let host_context = std::ptr::null_mut();
        let mut exe_context = ExecutionContext::new(&host, host_context);

        let data = vec![0xc0, 0xff, 0xfe];

        let message = ExecutionMessage {
            kind: MessageKind::EVMC_CALL,
            flags: 0,
            depth: 0,
            gas: 6566,
            recipient: test_addr,
            sender: test_addr,
            input: Some(data.clone()),
            value: Uint256::default(),
            create2_salt: Uint256::default(),
            code_address: test_addr,
            code: None,
            code_hash: None,
        };

        let b = exe_context.call(&message);

        assert_eq!(b.status_code, StatusCode::EVMC_SUCCESS);
        assert_eq!(b.gas_left, 2);
        assert!(b.output.is_some());
        assert_eq!(b.output.unwrap(), data);
        assert!(b.create_address.is_some());
        assert_eq!(b.create_address.unwrap(), Address::default());
    }
}
