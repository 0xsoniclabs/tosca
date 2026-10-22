use std::{hint, process};

use evmc_vm::{
    EvmcVm, ExecutionContext, ExecutionMessage, ExecutionResult, Revision, SetOptionError,
    StatusCode as EvmcStatusCode, StepResult, StepStatusCode as EvmcStepStatusCode,
    SteppableEvmcVm, Uint256, ffi::evmc_capabilities,
};

use crate::{
    ffi::EVMC_CAPABILITY,
    interpreter::Interpreter,
    types::{
        CodeAnalysis, CodeAnalysisCache, InterpreterAllocationsCache, LoggingObserver,
        NoOpObserver, ObserverType, hash_cache::HashCache, u256,
    },
};

pub struct EvmRs {
    observer_type: ObserverType,
    hash_cache: HashCache,
    code_analysis_cache_steppable: CodeAnalysisCache<true>,
    code_analysis_cache_non_steppable: CodeAnalysisCache<false>,
    interpreter_allocations: InterpreterAllocationsCache,
}

impl EvmcVm for EvmRs {
    fn init() -> Self {
        EvmRs {
            observer_type: ObserverType::NoOp,
            hash_cache: HashCache::default(),
            code_analysis_cache_steppable: CodeAnalysisCache::default(),
            code_analysis_cache_non_steppable: CodeAnalysisCache::default(),
            interpreter_allocations: InterpreterAllocationsCache::default(),
        }
    }

    fn execute<'a>(
        &self,
        revision: Revision,
        code: &'a [u8],
        message: &'a ExecutionMessage,
        context: Option<&'a mut ExecutionContext<'a>>,
    ) -> ExecutionResult {
        const {
            assert!(
                EVMC_CAPABILITY as u32 != evmc_capabilities::EVMC_CAPABILITY_PRECOMPILES as u32
            );
        }
        let Some(context) = context else {
            // Since EVMC_CAPABILITY_PRECOMPILES is not supported context must be set.
            // If this is not the case it violates the EVMC spec and is an irrecoverable error.
            process::abort();
        };
        let (mut stack_buffer, mut memory) = self.interpreter_allocations.new_stack_and_memory();
        let code_analysis = CodeAnalysis::new(
            code,
            message.code_hash().copied().map(u256::from),
            &self.code_analysis_cache_non_steppable,
        );
        let interpreter = Interpreter::new(
            revision,
            message,
            context,
            code,
            &code_analysis,
            &mut stack_buffer,
            &mut memory,
            &self.hash_cache,
        );
        let result = match interpreter {
            Ok(interpreter) => match self.observer_type {
                ObserverType::NoOp => interpreter.run(&mut NoOpObserver()),
                ObserverType::Logging => {
                    hint::cold_path();
                    interpreter.run(&mut LoggingObserver::new(std::io::stdout()))
                }
            },
            Err(fail_status) => fail_status.into(),
        };
        self.interpreter_allocations
            .release_stack_and_memory(stack_buffer, memory);
        result
    }

    fn set_option(&mut self, key: &str, value: &str) -> Result<(), SetOptionError> {
        match (key, value) {
            ("logging", "true") => self.observer_type = ObserverType::Logging,
            ("logging", "false") => self.observer_type = ObserverType::NoOp,
            ("code-analysis-cache-size", size) => {
                let size = size.parse().map_err(|_| SetOptionError::InvalidValue)?;
                self.code_analysis_cache_steppable = CodeAnalysisCache::new(size);
                self.code_analysis_cache_non_steppable = CodeAnalysisCache::new(size);
            }
            ("hash-cache-size", size) => {
                let size = size.parse().map_err(|_| SetOptionError::InvalidValue)?;
                self.hash_cache = HashCache::new(size);
            }
            _ => (),
        }
        Ok(())
    }
}

impl SteppableEvmcVm for EvmRs {
    fn step_n<'a>(
        &self,
        revision: Revision,
        code: &'a [u8],
        message: &'a ExecutionMessage,
        context: Option<&'a mut ExecutionContext<'a>>,
        step_status_code: EvmcStepStatusCode,
        pc: u64,
        gas_refund: i64,
        stack: &'a mut [Uint256],
        memory: &'a mut [u8],
        last_call_return_data: &'a mut [u8],
        steps: i32,
    ) -> StepResult {
        if step_status_code != EvmcStepStatusCode::EVMC_STEP_RUNNING {
            return StepResult {
                step_status_code,
                status_code: match step_status_code {
                    EvmcStepStatusCode::EVMC_STEP_RUNNING
                    | EvmcStepStatusCode::EVMC_STEP_STOPPED
                    | EvmcStepStatusCode::EVMC_STEP_RETURNED => EvmcStatusCode::EVMC_SUCCESS,
                    EvmcStepStatusCode::EVMC_STEP_REVERTED => EvmcStatusCode::EVMC_REVERT,
                    EvmcStepStatusCode::EVMC_STEP_FAILED => EvmcStatusCode::EVMC_FAILURE,
                },
                revision,
                pc,
                gas_left: message.gas(),
                gas_refund,
                output: Box::default(),
                stack: stack.to_owned(),
                memory: memory.to_owned(),
                last_call_return_data: Box::from(last_call_return_data),
            };
        }
        assert_ne!(
            EVMC_CAPABILITY,
            evmc_capabilities::EVMC_CAPABILITY_PRECOMPILES
        );
        let Some(context) = context else {
            // Since EVMC_CAPABILITY_PRECOMPILES is not supported context must be set.
            // If this is not the case it violates the EVMC spec and is an irrecoverable error.
            process::abort();
        };
        let stack = stack.iter().map(|i| u256::from(*i)).collect::<Vec<_>>();
        let (mut stack_buffer, mut run_memory) =
            self.interpreter_allocations.new_stack_and_memory();
        let code_analysis = CodeAnalysis::new(
            code,
            message.code_hash().copied().map(u256::from),
            &self.code_analysis_cache_steppable,
        );
        let interpreter = Interpreter::new_steppable(
            revision,
            message,
            context,
            code,
            pc as usize,
            gas_refund,
            &stack,
            memory,
            last_call_return_data,
            Some(steps),
            &code_analysis,
            &mut stack_buffer,
            &mut run_memory,
            &self.hash_cache,
        );
        let result = match interpreter {
            Ok(interpreter) => match self.observer_type {
                ObserverType::NoOp => interpreter.run(&mut NoOpObserver()),
                ObserverType::Logging => {
                    interpreter.run(&mut LoggingObserver::new(std::io::stdout()))
                }
            },
            Err(fail_status) => fail_status.into(),
        };
        self.interpreter_allocations
            .release_stack_and_memory(stack_buffer, run_memory);
        result
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "alloc-reuse")]
    use evmc_vm::ffi::evmc_host_interface;

    use super::*;
    #[cfg(feature = "alloc-reuse")]
    use crate::types::{MockExecutionMessage, Opcode};

    #[cfg(feature = "alloc-reuse")]
    fn host_interface_without_callbacks() -> evmc_host_interface {
        evmc_host_interface {
            account_exists: None,
            get_storage: None,
            set_storage: None,
            get_balance: None,
            get_code_size: None,
            get_code_hash: None,
            copy_code: None,
            selfdestruct: None,
            call: None,
            get_tx_context: None,
            get_block_hash: None,
            emit_log: None,
            access_account: None,
            access_storage: None,
            get_transient_storage: None,
            set_transient_storage: None,
        }
    }

    #[cfg(feature = "alloc-reuse")]
    #[test]
    fn execute_returns_stack_and_memory_to_pool() {
        let evm = EvmRs::init();
        let host = host_interface_without_callbacks();
        let mut context = ExecutionContext::new(&host, std::ptr::null_mut());
        let message = MockExecutionMessage::default().into();
        let code = [
            Opcode::Push1 as u8,
            1,
            Opcode::Push0 as u8,
            Opcode::MStore8 as u8,
        ];
        evm.execute(Revision::EVMC_CANCUN, &code, &message, Some(&mut context));

        let (stack_buffer, memory) = evm.interpreter_allocations.new_stack_and_memory();
        assert_eq!(stack_buffer[0], u256::ONE);
        assert_eq!(memory.as_slice()[0], 1);
    }

    #[test]
    fn set_option_with_cache_sizes_correctly_handles_input() {
        let mut evm = EvmRs::init();

        assert!(evm.set_option("code-analysis-cache-size", "100").is_ok());
        #[cfg(feature = "code-analysis-cache")]
        {
            assert_eq!(evm.code_analysis_cache_steppable.capacity(), 100);
            assert_eq!(evm.code_analysis_cache_non_steppable.capacity(), 100);
        }

        assert!(
            evm.set_option("code-analysis-cache-size", "invalid")
                .is_err()
        );
        #[cfg(feature = "code-analysis-cache")]
        {
            assert_eq!(evm.code_analysis_cache_steppable.capacity(), 100);
            assert_eq!(evm.code_analysis_cache_non_steppable.capacity(), 100);
        }

        assert!(evm.set_option("hash-cache-size", "100").is_ok());
        #[cfg(feature = "hash-cache")]
        assert_eq!(evm.hash_cache.capacity(), 100);

        assert!(evm.set_option("hash-cache-size", "invalid").is_err());
        #[cfg(feature = "hash-cache")]
        assert_eq!(evm.hash_cache.capacity(), 100);
    }

    #[cfg(feature = "alloc-reuse")]
    #[test]
    fn step_n_returns_stack_and_memory_to_pool() {
        let evm = EvmRs::init();
        let host = host_interface_without_callbacks();
        let mut context = ExecutionContext::new(&host, std::ptr::null_mut());
        let message = MockExecutionMessage::default().into();
        let code = [
            Opcode::Push1 as u8,
            1,
            Opcode::Push0 as u8,
            Opcode::MStore8 as u8,
        ];
        evm.step_n(
            Revision::EVMC_CANCUN,
            &code,
            &message,
            Some(&mut context),
            EvmcStepStatusCode::EVMC_STEP_RUNNING,
            0,
            0,
            &mut [],
            &mut [],
            &mut [],
            i32::MAX,
        );

        let (stack_buffer, memory) = evm.interpreter_allocations.new_stack_and_memory();
        assert_eq!(stack_buffer[0], u256::ONE);
        assert_eq!(memory.as_slice()[0], 1);
    }
}
