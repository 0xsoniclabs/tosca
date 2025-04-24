use std::{
    ffi::{CStr, c_char},
    panic,
};

use evmc_vm::{
    EvmcContainer, EvmcVm, ExecutionContext, ExecutionMessage, ExecutionResult, SetOptionError,
    ffi::{
        EVMC_ABI_VERSION, evmc_capabilities, evmc_capabilities_flagset, evmc_host_context,
        evmc_host_interface, evmc_message, evmc_result, evmc_revision, evmc_set_option_result,
        evmc_status_code, evmc_vm as evmc_vm_t,
    },
};

use crate::{
    evmrs::EvmRs,
    ffi::{
        LifetimeToken, ref_from_ptr_scoped, ref_mut_from_ptr_scoped, slice_from_raw_parts_scoped,
    },
};

static EVM_RS_NAME: &CStr = c"evmrs";
static EVM_RS_VERSION: &CStr = c"0.1.0";

/// Evmrs is currently only capable of executing EVM1 bytecode and not EWASM or precompiled
/// contracts.
pub const EVMC_CAPABILITY: evmc_capabilities = evmc_capabilities::EVMC_CAPABILITY_EVM1;

extern "C" fn __evmc_get_capabilities(_instance: *mut evmc_vm_t) -> evmc_capabilities_flagset {
    EVMC_CAPABILITY as evmc_capabilities_flagset
}

extern "C" fn __evmc_set_option(
    instance: *mut evmc_vm_t,
    key: *const c_char,
    value: *const c_char,
) -> evmc_set_option_result {
    let token = LifetimeToken;

    assert!(!instance.is_null());

    if key.is_null() {
        return evmc_set_option_result::EVMC_SET_OPTION_INVALID_NAME;
    }

    // SAFETY:
    // `key` is not null. The caller must make sure that it points to a C string.
    let key = unsafe { CStr::from_ptr(key) };
    let Ok(key) = key.to_str() else {
        return evmc_set_option_result::EVMC_SET_OPTION_INVALID_NAME;
    };

    let value = if !value.is_null() {
        // SAFETY:
        // `value` is not null. The caller must make sure that it points to a C string.
        unsafe { CStr::from_ptr(value) }
    } else {
        c""
    };

    let Ok(value) = value.to_str() else {
        return evmc_set_option_result::EVMC_SET_OPTION_INVALID_VALUE;
    };

    // SAFETY:
    // `instance` is not null. The caller must make sure that `instance` points to a valid
    // `EvmcContainer::<EvmRs>` (which is the case it it was created with evmc_create_evmrs) and the
    // pointer is unique.
    let container =
        unsafe { ref_mut_from_ptr_scoped(instance as *mut EvmcContainer<EvmRs>, &token) };

    match container.set_option(key, value) {
        Ok(()) => evmc_set_option_result::EVMC_SET_OPTION_SUCCESS,
        Err(SetOptionError::InvalidKey) => evmc_set_option_result::EVMC_SET_OPTION_INVALID_NAME,
        Err(SetOptionError::InvalidValue) => evmc_set_option_result::EVMC_SET_OPTION_INVALID_VALUE,
    }
}

#[unsafe(no_mangle)]
pub(super) extern "C" fn evmc_create_evmrs() -> *mut evmc_vm_t {
    let new_instance = evmc_vm_t {
        abi_version: EVMC_ABI_VERSION as i32,
        destroy: Some(__evmc_destroy),
        execute: Some(__evmc_execute),
        get_capabilities: Some(__evmc_get_capabilities),
        set_option: Some(__evmc_set_option),
        name: EVM_RS_NAME.as_ptr(),
        version: EVM_RS_VERSION.as_ptr(),
    };

    let container = EvmcContainer::<EvmRs>::new(new_instance);

    // Release ownership to EVMC.
    EvmcContainer::into_ffi_pointer(container)
}

extern "C" fn __evmc_destroy(instance: *mut evmc_vm_t) {
    if !instance.is_null() {
        // Acquire ownership from EVMC. This will deallocate it at the end of the scope.
        // SAFETY:
        // `instance` is not null. The caller must make sure that it points to a valid
        // `EvmcContainer::<EvmRs>`.
        unsafe {
            EvmcContainer::<EvmRs>::from_ffi_pointer(instance);
        }
    }
}

extern "C" fn __evmc_execute(
    instance: *mut evmc_vm_t,
    host: *const evmc_host_interface,
    context: *mut evmc_host_context,
    revision: evmc_revision,
    message: *const evmc_message,
    code: *const u8,
    code_size: usize,
) -> evmc_result {
    let token = LifetimeToken;

    if instance.is_null()
        || (host.is_null() && EVMC_CAPABILITY != evmc_capabilities::EVMC_CAPABILITY_PRECOMPILES)
        || message.is_null()
        || (code.is_null() && code_size != 0)
    {
        // These are irrecoverable errors that violate the EVMC spec.
        std::process::abort();
    }

    // SAFETY:
    // `message` is not null. The caller must make sure it points to a valid `ExecutionMessage`.
    let execution_message = ExecutionMessage::from(unsafe { ref_from_ptr_scoped(message, &token) });

    let code_ref = if code.is_null() {
        &[]
    } else {
        // SAFETY:
        // `code` is not null and `code_size > 0`. The caller must make sure that the size is
        // valid.
        unsafe { slice_from_raw_parts_scoped(code, code_size, &token) }
    };

    // SAFETY:
    // `instance` is not null. The caller must make sure that `instance` points to a valid
    // `EvmcContainer::<EvmRs>` (which is the case it it was created with evmc_create_evmrs) and the
    // pointer is unique.
    let container =
        unsafe { ref_mut_from_ptr_scoped(instance as *mut EvmcContainer<EvmRs>, &token) };

    panic::catch_unwind(|| {
        let mut execution_context = if host.is_null() {
            None
        } else {
            // SAFETY:
            // `host` is not null. The caller must make sure that it points to a valid
            // `evmc_host_interface`.
            let host = unsafe { ref_from_ptr_scoped(host, &token) };
            Some(ExecutionContext::new(host, context))
        };

        container.execute(
            revision,
            code_ref,
            &execution_message,
            execution_context.as_mut(),
        )
    })
    .unwrap_or(ExecutionResult {
        status_code: evmc_status_code::EVMC_INTERNAL_ERROR,
        gas_left: 0,
        gas_refund: 0,
        output: Box::default(),
        create_address: None,
    })
    .into()
}

#[cfg(test)]
mod tests {
    use evmc_vm::ffi::{evmc_capabilities_flagset, evmc_set_option_result};

    use crate::ffi::{
        EVMC_CAPABILITY,
        evmc_vm::{__evmc_destroy, __evmc_get_capabilities, __evmc_set_option, evmc_create_evmrs},
    };

    #[test]
    fn create_set_option_destroy() {
        let vm = evmc_create_evmrs();
        assert_eq!(
            __evmc_get_capabilities(vm),
            EVMC_CAPABILITY as evmc_capabilities_flagset
        );
        assert_eq!(
            __evmc_set_option(vm, std::ptr::null(), std::ptr::null()),
            evmc_set_option_result::EVMC_SET_OPTION_INVALID_NAME,
        );
        assert_eq!(
            __evmc_set_option(vm, c"\xF0\x90\x80".as_ptr(), std::ptr::null()),
            evmc_set_option_result::EVMC_SET_OPTION_INVALID_NAME,
        );
        assert_eq!(
            __evmc_set_option(vm, c"key".as_ptr(), std::ptr::null()),
            evmc_set_option_result::EVMC_SET_OPTION_SUCCESS,
        );
        assert_eq!(
            __evmc_set_option(vm, c"key".as_ptr(), c"\xF0\x90\x80".as_ptr()),
            evmc_set_option_result::EVMC_SET_OPTION_INVALID_VALUE,
        );
        assert_eq!(
            __evmc_set_option(vm, c"key".as_ptr(), c"value".as_ptr()),
            evmc_set_option_result::EVMC_SET_OPTION_SUCCESS,
        );
        __evmc_destroy(vm);
    }
}
