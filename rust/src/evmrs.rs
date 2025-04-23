use std::process;

use evmc_vm::{
    EvmcVm, ExecutionContext, ExecutionMessage, ExecutionResult, Revision, SetOptionError,
    StatusCode as EvmcStatusCode, StepResult, StepStatusCode as EvmcStepStatusCode,
    SteppableEvmcVm, Uint256, ffi::evmc_capabilities,
};

use crate::{
    ffi::EVMC_CAPABILITY,
    interpreter::Interpreter,
    types::{
        CodeAnalysisCache, LoggingObserver, Memory, NoOpObserver, ObserverType, Stack,
        hash_cache::HashCache, u256,
    },
};

pub struct EvmRs {
    observer_type: ObserverType,
    hash_cache: HashCache,
    code_analysis_cache_steppable: CodeAnalysisCache<true>,
    code_analysis_cache_non_steppable: CodeAnalysisCache<false>,
}

impl EvmcVm for EvmRs {
    fn init() -> Self {
        EvmRs {
            observer_type: ObserverType::NoOp,
            hash_cache: HashCache::default(),
            code_analysis_cache_steppable: CodeAnalysisCache::default(),
            code_analysis_cache_non_steppable: CodeAnalysisCache::default(),
        }
    }

    fn execute<'a>(
        &self,
        revision: Revision,
        code: &'a [u8],
        message: &'a ExecutionMessage,
        context: Option<&'a mut ExecutionContext<'a>>,
    ) -> ExecutionResult {
        assert_ne!(
            EVMC_CAPABILITY,
            evmc_capabilities::EVMC_CAPABILITY_PRECOMPILES
        );
        let Some(context) = context else {
            // Since EVMC_CAPABILITY_PRECOMPILES is not supported context must be set.
            // If this is not the case it violates the EVMC spec and is an irrecoverable error.
            process::abort();
        };
        let interpreter = Interpreter::new(
            revision,
            message,
            context,
            code,
            &self.code_analysis_cache_non_steppable,
            &self.hash_cache,
        );
        match self.observer_type {
            ObserverType::NoOp => interpreter.run(&mut NoOpObserver()),
            ObserverType::Logging => interpreter.run(&mut LoggingObserver::new(std::io::stdout())),
        }
    }

    fn set_option(&mut self, key: &str, value: &str) -> Result<(), SetOptionError> {
        match (key, value) {
            ("logging", "true") => self.observer_type = ObserverType::Logging,
            ("logging", "false") => self.observer_type = ObserverType::NoOp,
            ("code-analysis-cache-size", size) => {
                if let Ok(size) = size.parse::<usize>() {
                    self.code_analysis_cache_steppable = CodeAnalysisCache::new(size);
                    self.code_analysis_cache_non_steppable = CodeAnalysisCache::new(size);
                } else {
                    return Err(SetOptionError::InvalidValue);
                }
            }
            ("hash-cache-size", size) => {
                if let Ok(size) = size.parse::<usize>() {
                    self.hash_cache = HashCache::new(size);
                } else {
                    return Err(SetOptionError::InvalidValue);
                }
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
                gas_left: gas_refund,
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
        let stack = Stack::new(&stack.iter().map(|i| u256::from(*i)).collect::<Vec<_>>());
        let memory = Memory::new(memory);
        let interpreter = Interpreter::new_steppable(
            revision,
            message,
            context,
            code,
            pc as usize,
            gas_refund,
            stack,
            memory,
            Box::from(last_call_return_data),
            Some(steps),
            &self.code_analysis_cache_steppable,
            &self.hash_cache,
        );
        match self.observer_type {
            ObserverType::NoOp => interpreter.run(&mut NoOpObserver()),
            ObserverType::Logging => interpreter.run(&mut LoggingObserver::new(std::io::stdout())),
        }
    }
}

#[cfg(test)]
mod tests {
    use evmc_vm::EvmcVm;

    use crate::evmrs::EvmRs;

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
}
