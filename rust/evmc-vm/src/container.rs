// EVMC: Ethereum Client-VM Connector API.
// Copyright 2019 The EVMC Authors.
// Licensed under the Apache License, Version 2.0.

use std::ops::{Deref, DerefMut};

use crate::EvmcVm;

/// Container struct for EVMC instances and user-defined data.
#[repr(C)]
pub struct EvmcContainer<T>
where
    T: EvmcVm + Sized,
{
    #[allow(dead_code)]
    instance: ::evmc_sys::evmc_vm,
    vm: T,
}

impl<T> EvmcContainer<T>
where
    T: EvmcVm + Sized,
{
    /// Basic constructor.
    pub fn new(_instance: ::evmc_sys::evmc_vm) -> Box<Self> {
        Box::new(Self {
            instance: _instance,
            vm: T::init(),
        })
    }

    /// Take ownership of the given pointer and return a box.
    ///
    /// # Safety
    /// `instance` must be a valid non-aliased pointer to an [`EvmcContainer`] struct allocated by
    /// the global allocator.
    pub unsafe fn from_ffi_pointer(instance: *mut ::evmc_sys::evmc_vm) -> Box<Self> {
        assert!(!instance.is_null(), "from_ffi_pointer received NULL");
        // Safety:
        // instance is a valid pointer to an [`EvmcContainer`] struct allocated by the global
        // allocator.
        unsafe { Box::from_raw(instance as *mut EvmcContainer<T>) }
    }

    /// Convert boxed self into an FFI pointer, surrendering ownership of the heap data.
    pub fn into_ffi_pointer(boxed: Box<Self>) -> *mut ::evmc_sys::evmc_vm {
        Box::into_raw(boxed) as *mut ::evmc_sys::evmc_vm
    }
}

impl<T> Deref for EvmcContainer<T>
where
    T: EvmcVm,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.vm
    }
}

impl<T> DerefMut for EvmcContainer<T>
where
    T: EvmcVm,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.vm
    }
}

/// Container struct for steppable EVMC instances and user-defined data.
#[repr(C)]
pub struct SteppableEvmcContainer<T>
where
    T: EvmcVm + Sized,
{
    #[allow(dead_code)]
    instance: ::evmc_sys::evmc_vm_steppable,
    vm: T,
}

impl<T> SteppableEvmcContainer<T>
where
    T: EvmcVm + Sized,
{
    /// Basic constructor.
    pub fn new(_instance: ::evmc_sys::evmc_vm_steppable) -> Box<Self> {
        Box::new(Self {
            instance: _instance,
            vm: T::init(),
        })
    }

    /// Take ownership of the given pointer and free the associated memory.
    ///
    /// # Safety
    /// `instance` must be a valid non-aliased pointer to an [`SteppableEvmcContainer`] struct
    /// allocated by the global allocator.
    pub unsafe fn from_ffi_pointer(instance: *mut ::evmc_sys::evmc_vm_steppable) -> Box<Self> {
        assert!(!instance.is_null(), "from_ffi_pointer received NULL");
        // Safety:
        // instance is a valid pointer to an [`EvmcContainer`] struct allocated by the global
        // allocator.
        unsafe { Box::from_raw(instance as *mut SteppableEvmcContainer<T>) }
    }

    /// Convert boxed self into an FFI pointer, surrendering ownership of the heap data.
    pub fn into_ffi_pointer(boxed: Box<Self>) -> *mut ::evmc_sys::evmc_vm_steppable {
        Box::into_raw(boxed) as *mut ::evmc_sys::evmc_vm_steppable
    }
}

impl<T> Drop for SteppableEvmcContainer<T>
where
    T: EvmcVm + Sized,
{
    fn drop(&mut self) {
        unsafe {
            EvmcContainer::<T>::from_ffi_pointer(self.instance.vm);
        }
    }
}

impl<T> Deref for SteppableEvmcContainer<T>
where
    T: EvmcVm,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.vm
    }
}

impl<T> DerefMut for SteppableEvmcContainer<T>
where
    T: EvmcVm,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.vm
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ExecutionContext, ExecutionMessage, ExecutionResult, types::*};

    struct TestVm {}

    impl EvmcVm for TestVm {
        fn init() -> Self {
            TestVm {}
        }
        fn execute(
            &self,
            _revision: evmc_sys::evmc_revision,
            _code: &[u8],
            _message: &ExecutionMessage,
            _context: Option<&mut ExecutionContext>,
        ) -> ExecutionResult {
            ExecutionResult {
                status_code: StatusCode::EVMC_FAILURE,
                gas_left: 0,
                gas_refund: 0,
                output: Box::default(),
                create_address: None,
            }
        }
    }

    unsafe extern "C" fn get_dummy_tx_context(
        _context: *mut evmc_sys::evmc_host_context,
    ) -> evmc_sys::evmc_tx_context {
        evmc_sys::evmc_tx_context {
            tx_gas_price: Uint256::default(),
            tx_origin: Address::default(),
            block_coinbase: Address::default(),
            block_number: 0,
            block_timestamp: 0,
            block_gas_limit: 0,
            block_prev_randao: Uint256::default(),
            chain_id: Uint256::default(),
            block_base_fee: Uint256::default(),
            blob_base_fee: Uint256::default(),
            blob_hashes: std::ptr::null(),
            blob_hashes_count: 0,
            initcodes: std::ptr::null(),
            initcodes_count: 0,
        }
    }

    #[test]
    fn container_new() {
        let instance = ::evmc_sys::evmc_vm {
            abi_version: ::evmc_sys::EVMC_ABI_VERSION as i32,
            name: std::ptr::null(),
            version: std::ptr::null(),
            destroy: None,
            execute: None,
            get_capabilities: None,
            set_option: None,
        };

        let code = [0u8; 0];

        let message = ::evmc_sys::evmc_message {
            kind: ::evmc_sys::evmc_call_kind::EVMC_CALL,
            flags: 0,
            depth: 0,
            gas: 0,
            recipient: ::evmc_sys::evmc_address::default(),
            sender: ::evmc_sys::evmc_address::default(),
            input_data: std::ptr::null(),
            input_size: 0,
            value: ::evmc_sys::evmc_uint256be::default(),
            create2_salt: ::evmc_sys::evmc_bytes32::default(),
            code_address: ::evmc_sys::evmc_address::default(),
            code: std::ptr::null(),
            code_size: 0,
            code_hash: std::ptr::null(),
        };
        let message: ExecutionMessage = (&message).into();

        let host = ::evmc_sys::evmc_host_interface {
            account_exists: None,
            get_storage: None,
            set_storage: None,
            get_balance: None,
            get_code_size: None,
            get_code_hash: None,
            copy_code: None,
            selfdestruct: None,
            call: None,
            get_tx_context: Some(get_dummy_tx_context),
            get_block_hash: None,
            emit_log: None,
            access_account: None,
            access_storage: None,
            get_transient_storage: None,
            set_transient_storage: None,
        };
        let host_context = std::ptr::null_mut();

        let mut context = ExecutionContext::new(&host, host_context);
        let container = EvmcContainer::<TestVm>::new(instance);
        assert_eq!(
            container
                .execute(
                    evmc_sys::evmc_revision::EVMC_PETERSBURG,
                    &code,
                    &message,
                    Some(&mut context)
                )
                .status_code,
            ::evmc_sys::evmc_status_code::EVMC_FAILURE
        );

        let ptr = EvmcContainer::into_ffi_pointer(container);

        let mut context = ExecutionContext::new(&host, host_context);
        let container = unsafe { EvmcContainer::<TestVm>::from_ffi_pointer(ptr) };
        assert_eq!(
            container
                .execute(
                    evmc_sys::evmc_revision::EVMC_PETERSBURG,
                    &code,
                    &message,
                    Some(&mut context)
                )
                .status_code,
            ::evmc_sys::evmc_status_code::EVMC_FAILURE
        );
    }
}
