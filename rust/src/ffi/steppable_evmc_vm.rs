use std::{ffi::c_void, panic, slice};

use ::evmc_vm::{
    ExecutionContext, ExecutionMessage, StatusCode, StepResult, StepStatusCode,
    SteppableEvmcContainer, SteppableEvmcVm,
    ffi::{
        evmc_bytes32, evmc_capabilities, evmc_host_interface, evmc_message, evmc_revision,
        evmc_step_result, evmc_step_status_code, evmc_vm_steppable,
    },
};

use crate::{
    evmrs::EvmRs,
    ffi::{
        LifetimeToken,
        evmc_vm::{self, EVMC_CAPABILITY},
        ref_from_ptr_scoped, ref_mut_from_ptr_scoped, slice_from_raw_parts_scoped,
    },
};

#[unsafe(no_mangle)]
extern "C" fn evmc_create_steppable_evmrs() -> *mut evmc_vm_steppable {
    let new_instance = evmc_vm_steppable {
        vm: evmc_vm::evmc_create_evmrs(),
        step_n: Some(__evmc_step_n),
        destroy: Some(__evmc_steppable_destroy),
    };
    let container = SteppableEvmcContainer::<EvmRs>::new(new_instance);

    // Release ownership to EVMC.
    SteppableEvmcContainer::into_ffi_pointer(container)
}

extern "C" fn __evmc_steppable_destroy(instance: *mut evmc_vm_steppable) {
    if !instance.is_null() {
        // Acquire ownership from EVMC. This will deallocate it also at the end of the scope.
        // SAFETY:
        // `instance` is not null. The caller must make sure that it points to a valid
        // `SteppableEvmcContainer::<EvmRs>`.
        unsafe {
            SteppableEvmcContainer::<EvmRs>::from_ffi_pointer(instance);
        }
    }
}

extern "C" fn __evmc_step_n(
    instance: *mut evmc_vm_steppable,
    host: *const evmc_host_interface,
    context: *mut c_void,
    revision: evmc_revision,
    message: *const evmc_message,
    code: *const u8,
    code_size: usize,
    status: evmc_step_status_code,
    pc: u64,
    gas_refunds: i64,
    stack: *mut evmc_bytes32,
    stack_size: usize,
    memory: *mut u8,
    memory_size: usize,
    last_call_result_data: *mut u8,
    last_call_result_data_size: usize,
    steps: i32,
) -> evmc_step_result {
    let token = LifetimeToken;

    if instance.is_null()
        || (host.is_null() && EVMC_CAPABILITY != evmc_capabilities::EVMC_CAPABILITY_PRECOMPILES)
        || message.is_null()
        || (code.is_null() && code_size > 0)
        || (stack.is_null() && stack_size > 0)
        || (memory.is_null() && memory_size > 0)
        || (last_call_result_data.is_null() && last_call_result_data_size > 0)
    {
        // These are irrecoverable errors that violate the EVMC spec.
        std::process::abort();
    }

    // SAFETY:
    // `message` is not null. The caller must make sure that is points to a valid
    // `ExecutionMessage`.
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
    // `instance` is not null. The caller must make sure that it points to a valid
    // `SteppableEvmcContainer::<EvmRs>` (which is the case it it was created with
    // evmc_create_steppable_evmrs) an the pointer is unique.
    let container =
        unsafe { ref_mut_from_ptr_scoped(instance as *mut SteppableEvmcContainer<EvmRs>, &token) };

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

        let stack = if stack.is_null() {
            &mut []
        } else {
            // SAFETY:
            // `stack` is not null and `stack_size > 0`. The caller must make sure that the size
            // is valid.
            unsafe { slice::from_raw_parts_mut(stack, stack_size) }
        };

        let memory = if memory.is_null() {
            &mut []
        } else {
            // SAFETY:
            // `memory` is not null and `memory_size > 0`. The caller must make sure that the
            // size is valid.
            unsafe { slice::from_raw_parts_mut(memory, memory_size) }
        };

        let last_call_result_data = if last_call_result_data.is_null() {
            &mut []
        } else {
            // SAFETY:
            // `last_call_return_data` is not null and `last_call_return_data_size > 0`. The
            // caller must make sure that the size is valid.
            unsafe { slice::from_raw_parts_mut(last_call_result_data, last_call_result_data_size) }
        };

        container.step_n(
            revision,
            code_ref,
            &execution_message,
            execution_context.as_mut(),
            status,
            pc,
            gas_refunds,
            stack,
            memory,
            last_call_result_data,
            steps,
        )
    })
    .unwrap_or_else(|_| StepResult {
        step_status_code: StepStatusCode::EVMC_STEP_FAILED,
        status_code: StatusCode::EVMC_INTERNAL_ERROR,
        revision,
        pc: 0,
        gas_left: 0,
        gas_refund: 0,
        output: Box::default(),
        stack: Vec::new(),
        memory: Vec::new(),
        last_call_return_data: Box::default(),
    })
    .into()
}

#[cfg(test)]
mod tests {
    use crate::ffi::steppable_evmc_vm::{__evmc_steppable_destroy, evmc_create_steppable_evmrs};

    #[test]
    fn create_destroy() {
        let vm = evmc_create_steppable_evmrs();
        __evmc_steppable_destroy(vm);
    }
}
