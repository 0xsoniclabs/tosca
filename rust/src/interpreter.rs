//! The interpreter runs the opcode handlers of a jump table, as a chain of tail calls with
//! `tail-call` and from a loop without. The choices below keep the hot state of a run in
//! registers.
//!
//! # State in registers
//!
//! With function-pointer dispatch every handler hands the [`Interpreter`] to a function LLVM cannot
//! see, which reads the state from memory, so each handler would store the state it changes before
//! the call and the next one load it back. `noalias` on the `&mut` does not help: it covers only
//! the duration of one call, and the callee is a legitimate user of the pointer. Only arguments and
//! return values stay in registers, one word each; an aggregate wider than two words goes through
//! memory. So the handlers take the program counter ([`Pc`], one word), the gas counter ([`Gas`],
//! one word) and the [`Stack`] (two words) by value and hand them on. The [`Interpreter`] holds
//! everything else, and parks the state while no handler runs.
//!
//! # Return value
//!
//! Without `tail-call` every handler returns its state to the loop, so its result has to fit into
//! two registers: the program counter and a [`GasAndSize`], the gas counter and the stack size
//! packed into one word. The stack size takes 16 bits, which leaves 48 for the gas counter, hence
//! [`Gas::MAX`].
//!
//! # Calling conventions
//!
//! With `tail-call` the handlers use `extern "rust-preserve-none"`, which leaves no register
//! callee-saved, so a handler saves nothing before its tail call. It returns values like the Rust
//! convention, an [`OpFnResult`] through memory, but only the handler that ends the chain returns.
//!
//! Without `tail-call` they use `extern "C-unwind"`: the Rust convention returns a `Result` with
//! data in `Err` through memory whatever its size, the C convention returns its 16 bytes in two
//! registers. Only Rust calls the handlers, so their Rust types never cross into C. [`FailStatus`]
//! is a full word because, as a one-byte enum, LLVM assembles the second return register byte by
//! byte.
//!
//! The handlers are spelled `extern "C-unwind"`, and the `preserve_none` attribute rewrites that
//! with `tail-call`.
//!
//! # Ending a run
//!
//! A handler ends with `next!`, which tail calls the next handler or returns to the loop, or with
//! `Interpreter::exit` when its operation ends the run. In the loop `Ok` means going on, so `exit`
//! parks the final state in the [`Interpreter`] and returns `Err(Exit::Stopped)`, which keeps one
//! test per operation. A failure is `Err(Exit::Failed(_))`.

// See "Calling conventions" above.
#![cfg_attr(not(feature = "tail-call"), allow(improper_ctypes_definitions))]

use std::{
    cmp::min,
    ops::{Add, BitAnd, BitOr, BitXor, Div, Mul, Not, Rem, Sub},
};

use evmc_vm::{
    AccessStatus, Address, ExecutionMessage, ExecutionResult, MessageFlags, MessageKind, Revision,
    StatusCode, StepResult, StorageStatus, Uint256,
};

#[cfg(not(feature = "fn-ptr-conversion-dispatch"))]
use crate::types::GetOpcodeError;
use crate::{
    types::{
        Code, CodeAnalysis, ExecStatus, ExecutionContextTrait, FailStatus, GasAndSize,
        LastCallReturnData, Memory, Observer, Pc, Stack, StackBuffer, hash_cache::HashCache, u256,
    },
    utils::{Gas, GasRefund, SliceExt, check_min_revision, check_not_read_only, word_size},
};

/// How a handler ends the run early.
#[derive(Debug)]
pub enum Exit {
    /// The run stopped, and the operation parked its final state in the [`Interpreter`].
    Stopped,
    /// The run failed.
    Failed(FailStatus),
}

impl From<FailStatus> for Exit {
    fn from(fail_status: FailStatus) -> Self {
        Self::Failed(fail_status)
    }
}

/// The return type of the handlers: the program counter and, packed into one word, the gas counter
/// and the size of the stack. The error ends the run, see [`Exit`].
type OpFnResult<'a, const STEPPABLE: bool> = Result<(Pc<'a, STEPPABLE>, GasAndSize), Exit>;

/// The signature of a handler. With `tail-call` the handlers use the `rust-preserve-none` calling
/// convention.
#[cfg(feature = "tail-call")]
pub type OpFn<const STEPPABLE: bool> =
    for<'a> extern "rust-preserve-none" fn(
        &mut Interpreter<'a, STEPPABLE>,
        Pc<'a, STEPPABLE>,
        Gas,
        Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE>;
/// The signature of a handler. Without `tail-call` the handlers use the `C-unwind` calling
/// convention.
#[cfg(not(feature = "tail-call"))]
pub type OpFn<const STEPPABLE: bool> = for<'a> extern "C-unwind" fn(
    &mut Interpreter<'a, STEPPABLE>,
    Pc<'a, STEPPABLE>,
    Gas,
    Stack<'_>,
) -> OpFnResult<'a, STEPPABLE>;

/// Gives a handler the `rust-preserve-none` calling convention of [`OpFn`] with `tail-call`, in
/// place of the `C-unwind` calling convention it has without.
#[cfg(feature = "tail-call")]
macro_rules! preserve_none {
    attr() ($(#[$attr:meta])* $vis:vis extern "C-unwind" fn $($rest:tt)*) => {
        $(#[$attr])* $vis extern "rust-preserve-none" fn $($rest)*
    };
}

/// Ends a handler by handing the state on to the next operation. With `tail-call` that is a tail
/// call of [`Interpreter::next`]; `become` is gated even where `cfg` removes it, so only a macro
/// can spell it.
#[cfg(feature = "tail-call")]
macro_rules! next {
    ($interpreter:expr, $pc:expr, $gas_left:expr, $stack:expr) => {
        become Interpreter::next($interpreter, $pc, $gas_left, $stack)
    };
}
/// Ends a handler by handing the state on to the next operation. Without `tail-call` the handler
/// returns the state to the loop, which dispatches the next operation.
#[cfg(not(feature = "tail-call"))]
macro_rules! next {
    ($interpreter:expr, $pc:expr, $gas_left:expr, $stack:expr) => {{
        let _ = $interpreter;
        return Ok(($pc, GasAndSize::new($gas_left, $stack)));
    }};
}

const fn gen_jumptable<const STEPPABLE: bool>() -> [OpFn<STEPPABLE>; 256] {
    [
        Interpreter::stop,
        Interpreter::add,
        Interpreter::mul,
        Interpreter::sub,
        Interpreter::div,
        Interpreter::s_div,
        Interpreter::mod_,
        Interpreter::s_mod,
        Interpreter::add_mod,
        Interpreter::mul_mod,
        Interpreter::exp,
        Interpreter::sign_extend,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::lt,
        Interpreter::gt,
        Interpreter::s_lt,
        Interpreter::s_gt,
        Interpreter::eq,
        Interpreter::is_zero,
        Interpreter::and,
        Interpreter::or,
        Interpreter::xor,
        Interpreter::not,
        Interpreter::byte,
        Interpreter::shl,
        Interpreter::shr,
        Interpreter::sar,
        Interpreter::clz,
        Interpreter::jumptable_placeholder,
        Interpreter::sha3,
        std::cfg_select! {
            feature = "fn-ptr-conversion-dispatch" => Interpreter::no_op,
            _ => Interpreter::jumptable_placeholder,
        },
        std::cfg_select! {
            feature = "fn-ptr-conversion-dispatch" => Interpreter::skip_no_ops,
            _ => Interpreter::jumptable_placeholder,
        },
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::address,
        Interpreter::balance,
        Interpreter::origin,
        Interpreter::caller,
        Interpreter::call_value,
        Interpreter::call_data_load,
        Interpreter::call_data_size,
        Interpreter::call_data_copy,
        Interpreter::code_size,
        Interpreter::code_copy,
        Interpreter::gas_price,
        Interpreter::ext_code_size,
        Interpreter::ext_code_copy,
        Interpreter::return_data_size,
        Interpreter::return_data_copy,
        Interpreter::ext_code_hash,
        Interpreter::block_hash,
        Interpreter::coinbase,
        Interpreter::timestamp,
        Interpreter::number,
        Interpreter::prev_randao,
        Interpreter::gas_limit,
        Interpreter::chain_id,
        Interpreter::self_balance,
        Interpreter::base_fee,
        Interpreter::blob_hash,
        Interpreter::blob_base_fee,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::pop,
        Interpreter::m_load,
        Interpreter::m_store,
        Interpreter::m_store8,
        Interpreter::s_load,
        Interpreter::sstore,
        Interpreter::jump,
        Interpreter::jump_i,
        Interpreter::pc,
        Interpreter::m_size,
        Interpreter::gas,
        Interpreter::jump_dest,
        Interpreter::t_load,
        Interpreter::t_store,
        Interpreter::m_copy,
        Interpreter::push0,
        Interpreter::push::<1>,
        Interpreter::push::<2>,
        Interpreter::push::<3>,
        Interpreter::push::<4>,
        Interpreter::push::<5>,
        Interpreter::push::<6>,
        Interpreter::push::<7>,
        Interpreter::push::<8>,
        Interpreter::push::<9>,
        Interpreter::push::<10>,
        Interpreter::push::<11>,
        Interpreter::push::<12>,
        Interpreter::push::<13>,
        Interpreter::push::<14>,
        Interpreter::push::<15>,
        Interpreter::push::<16>,
        Interpreter::push::<17>,
        Interpreter::push::<18>,
        Interpreter::push::<19>,
        Interpreter::push::<20>,
        Interpreter::push::<21>,
        Interpreter::push::<22>,
        Interpreter::push::<23>,
        Interpreter::push::<24>,
        Interpreter::push::<25>,
        Interpreter::push::<26>,
        Interpreter::push::<27>,
        Interpreter::push::<28>,
        Interpreter::push::<29>,
        Interpreter::push::<30>,
        Interpreter::push::<31>,
        Interpreter::push::<32>,
        Interpreter::dup::<1>,
        Interpreter::dup::<2>,
        Interpreter::dup::<3>,
        Interpreter::dup::<4>,
        Interpreter::dup::<5>,
        Interpreter::dup::<6>,
        Interpreter::dup::<7>,
        Interpreter::dup::<8>,
        Interpreter::dup::<9>,
        Interpreter::dup::<10>,
        Interpreter::dup::<11>,
        Interpreter::dup::<12>,
        Interpreter::dup::<13>,
        Interpreter::dup::<14>,
        Interpreter::dup::<15>,
        Interpreter::dup::<16>,
        Interpreter::swap::<1>,
        Interpreter::swap::<2>,
        Interpreter::swap::<3>,
        Interpreter::swap::<4>,
        Interpreter::swap::<5>,
        Interpreter::swap::<6>,
        Interpreter::swap::<7>,
        Interpreter::swap::<8>,
        Interpreter::swap::<9>,
        Interpreter::swap::<10>,
        Interpreter::swap::<11>,
        Interpreter::swap::<12>,
        Interpreter::swap::<13>,
        Interpreter::swap::<14>,
        Interpreter::swap::<15>,
        Interpreter::swap::<16>,
        Interpreter::log::<0>,
        Interpreter::log::<1>,
        Interpreter::log::<2>,
        Interpreter::log::<3>,
        Interpreter::log::<4>,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::create_or_create2::<false>,
        Interpreter::call_or_call_code::<false>,
        Interpreter::call_or_call_code::<true>,
        Interpreter::return_,
        Interpreter::static_or_delegate_call::<true>,
        Interpreter::create_or_create2::<true>,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::static_or_delegate_call::<false>,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::revert,
        Interpreter::invalid,
        Interpreter::self_destruct,
    ]
}

pub const fn get_jumptable<const STEPPABLE: bool>() -> &'static [OpFn<STEPPABLE>; 256] {
    static JUMPTABLE_STEPPABLE: [OpFn<true>; 256] = gen_jumptable();
    static JUMPTABLE_NON_STEPPABLE: [OpFn<false>; 256] = gen_jumptable();
    if STEPPABLE {
        // SAFETY:
        // STEPPABLE is true
        unsafe {
            std::mem::transmute::<&'static [OpFn<true>; 256], &'static [OpFn<STEPPABLE>; 256]>(
                &JUMPTABLE_STEPPABLE,
            )
        }
    } else {
        // SAFETY:
        // STEPPABLE is false
        unsafe {
            std::mem::transmute::<&'static [OpFn<false>; 256], &'static [OpFn<STEPPABLE>; 256]>(
                &JUMPTABLE_NON_STEPPABLE,
            )
        }
    }
}

/// The state of a run. The program counter, the gas counter and the stack are passed to the
/// handlers in registers and parked here while they do not.
pub struct Interpreter<'a, const STEPPABLE: bool> {
    pub exec_status: ExecStatus,
    pub message: &'a ExecutionMessage<'a>,
    pub context: &'a mut dyn ExecutionContextTrait,
    pub revision: Revision,
    pub code: Code<'a, STEPPABLE>,
    pub gas_refund: GasRefund,
    pub output: Box<[u8]>,
    pub memory: &'a mut Memory,
    pub last_call_return_data: LastCallReturnData<'a>,
    pub steps: Option<i32>,
    pub hash_cache: &'a HashCache,
    pub parked: Parked<'a, STEPPABLE>,
    pub stack: Option<&'a mut StackBuffer>,
}

/// The program counter, the gas counter and the size of the stack while the handlers do not run.
#[derive(Debug)]
pub struct Parked<'a, const STEPPABLE: bool> {
    pc: Pc<'a, STEPPABLE>,
    state: GasAndSize,
}

impl<'a> Interpreter<'a, false> {
    #[expect(clippy::too_many_arguments)]
    pub fn new(
        revision: Revision,
        message: &'a ExecutionMessage,
        context: &'a mut dyn ExecutionContextTrait,
        code: &'a [u8],
        code_analysis: &'a CodeAnalysis<false>,
        stack_buffer: &'a mut StackBuffer,
        memory: &'a mut Memory,
        hash_cache: &'a HashCache,
    ) -> Result<Self, FailStatus> {
        memory.reset_to(&[]);
        Ok(Self {
            exec_status: ExecStatus::Running,
            message,
            context,
            revision,
            code: Code::new(code, code_analysis),
            gas_refund: GasRefund::new(0),
            output: Box::default(),
            memory,
            last_call_return_data: LastCallReturnData::Slice(&[]),
            steps: None,
            hash_cache,
            parked: Parked {
                pc: Pc::new(code_analysis, 0),
                state: GasAndSize::new(Gas::new(message.gas())?, Stack::empty(stack_buffer)),
            },
            stack: Some(stack_buffer),
        })
    }
}

impl<'a> Interpreter<'a, true> {
    #[expect(clippy::too_many_arguments)]
    pub fn new_steppable(
        revision: Revision,
        message: &'a ExecutionMessage,
        context: &'a mut dyn ExecutionContextTrait,
        code: &'a [u8],
        pc: usize,
        gas_refund: i64,
        init_stack: &[u256],
        init_memory: &[u8],
        last_call_return_data: &'a [u8],
        steps: Option<i32>,
        code_analysis: &'a CodeAnalysis<true>,
        stack_buffer: &'a mut StackBuffer,
        memory: &'a mut Memory,
        hash_cache: &'a HashCache,
    ) -> Result<Self, FailStatus> {
        let mut stack = Stack::empty(stack_buffer);
        stack.reset_to(init_stack);
        let state = GasAndSize::new(Gas::new(message.gas())?, stack);
        memory.reset_to(init_memory);
        Ok(Self {
            exec_status: ExecStatus::Running,
            message,
            context,
            revision,
            code: Code::new(code, code_analysis),
            gas_refund: GasRefund::new(gas_refund),
            output: Box::default(),
            memory,
            last_call_return_data: LastCallReturnData::Slice(last_call_return_data),
            steps,
            hash_cache,
            parked: Parked {
                pc: Pc::new(code_analysis, pc),
                state,
            },
            stack: Some(stack_buffer),
        })
    }
}

impl<'i, const STEPPABLE: bool> Interpreter<'i, STEPPABLE> {
    /// R is expected to be [ExecutionResult] or [StepResult].
    #[cfg(not(feature = "tail-call"))]
    pub fn run<O, R>(mut self, observer: &mut O) -> R
    where
        O: Observer<STEPPABLE>,
        R: From<Self> + From<FailStatus>,
    {
        let stack_buffer = self.stack.take().expect("only a run takes the stack");
        let mut pc = self.parked.pc;
        // Packed like the handlers return it and unpacked into their arguments, so that the test of
        // their result closes the loop.
        let mut state = self.parked.state;
        let mut fail = None;
        // A handler that ends the run parks its final state and returns an error, like one that
        // fails; the loop parks the state itself when it ends the run.
        loop {
            if STEPPABLE {
                match &mut self.steps {
                    None => (),
                    Some(0) => {
                        self.park(pc, state);
                        break;
                    }
                    Some(steps) => *steps -= 1,
                }
            }
            #[cfg(feature = "fn-ptr-conversion-dispatch")]
            let op = pc.get();
            #[cfg(not(feature = "fn-ptr-conversion-dispatch"))]
            let op = match self.code.get(pc) {
                Ok(op) => op,
                Err(GetOpcodeError::OutOfRange) => {
                    std::hint::cold_path();
                    self.exec_status = ExecStatus::Stopped;
                    self.park(pc, state);
                    break;
                }
                Err(GetOpcodeError::Invalid) => {
                    std::hint::cold_path();
                    fail = Some(FailStatus::InvalidInstruction);
                    break;
                }
            };
            let stack = Stack::new(stack_buffer, state);
            let gas_left = state.gas();
            observer.pre_op(&self, pc, &gas_left, &stack);
            let result = std::cfg_select! {
                feature = "fn-ptr-conversion-dispatch" => op(&mut self, pc, gas_left, stack),
                _ => get_jumptable()[op as usize](&mut self, pc, gas_left, stack),
            };
            (pc, state) = match result {
                Ok(next) => next,
                Err(exit) => {
                    std::hint::cold_path();
                    if let Exit::Failed(err) = exit {
                        fail = Some(err);
                    }
                    break;
                }
            };
            observer.post_op(&self, pc, &state.gas(), &Stack::new(stack_buffer, state));
        }
        self.stack = Some(stack_buffer);
        if let Some(err) = fail {
            std::hint::cold_path();
            return err.into();
        }

        self.into()
    }

    /// R is expected to be [ExecutionResult] or [StepResult].
    #[cfg(feature = "tail-call")]
    pub fn run<O, R>(mut self, observer: &mut O) -> R
    where
        O: Observer<STEPPABLE>,
        R: From<Self> + From<FailStatus>,
    {
        observer.log("feature \"tail-call\" does not support logging".into());
        // The chain hands back the final state when it runs out of steps or code; an operation that
        // ends the run parks it itself.
        let stack_buffer = self.stack.take().expect("only a run takes the stack");
        let stack = Stack::new(stack_buffer, self.parked.state);
        let pc = self.parked.pc;
        let gas_left = self.parked.state.gas();
        let result = Self::next(&mut self, pc, gas_left, stack);
        self.stack = Some(stack_buffer);
        match result {
            Ok((pc, state)) => self.park(pc, state),
            Err(Exit::Failed(err)) => {
                std::hint::cold_path();
                return err.into();
            }
            Err(Exit::Stopped) => (),
        }
        self.into()
    }

    /// Dispatches the operation at `pc` as a tail call, or ends the chain when the run is out of
    /// steps or code.
    #[cfg(feature = "tail-call")]
    #[inline(always)]
    extern "rust-preserve-none" fn next(
        interpreter: &mut Interpreter<'i, STEPPABLE>,
        pc: Pc<'i, STEPPABLE>,
        gas_left: Gas,
        stack: Stack<'_>,
    ) -> OpFnResult<'i, STEPPABLE> {
        if STEPPABLE {
            match &mut interpreter.steps {
                None => (),
                Some(0) => return Ok((pc, GasAndSize::new(gas_left, stack))),
                Some(steps) => *steps -= 1,
            }
        }
        std::cfg_select! {
            feature = "fn-ptr-conversion-dispatch" => {
                let op = pc.get();
                become op(interpreter, pc, gas_left, stack)
            }
            _ => match interpreter.code.get(pc) {
                Ok(op) => become get_jumptable()[op as usize](interpreter, pc, gas_left, stack),
                Err(GetOpcodeError::OutOfRange) => {
                    std::hint::cold_path();
                    interpreter.exec_status = ExecStatus::Stopped;
                    Ok((pc, GasAndSize::new(gas_left, stack)))
                }
                Err(GetOpcodeError::Invalid) => {
                    std::hint::cold_path();
                    Err(Exit::Failed(FailStatus::InvalidInstruction))
                }
            },
        }
    }

    /// Parks the program counter, the gas counter and the size of the stack for after the run.
    fn park(&mut self, pc: Pc<'i, STEPPABLE>, state: GasAndSize) {
        self.parked = Parked { pc, state };
    }

    /// Ends the handler of an operation that ends the run: parks the final state and stops the run.
    #[inline(always)]
    fn exit(
        &mut self,
        pc: Pc<'i, STEPPABLE>,
        gas_left: Gas,
        stack: Stack<'_>,
    ) -> OpFnResult<'i, STEPPABLE> {
        self.park(pc, GasAndSize::new(gas_left, stack));
        Err(Exit::Stopped)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    #[cold]
    pub extern "C-unwind" fn jumptable_placeholder<'a>(
        _interpreter: &mut Interpreter<'a, STEPPABLE>,
        _pc: Pc<'a, STEPPABLE>,
        _gas_left: Gas,
        _stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        Err(Exit::Failed(FailStatus::Failure))
    }

    /// Performs an operation that charges `cost` gas, pops one value and pushes `op` applied to it.
    #[inline(always)]
    fn unary_op<'a, 's, I: Into<u256>>(
        mut pc: Pc<'a, STEPPABLE>,
        mut gas_left: Gas,
        mut stack: Stack<'s>,
        op: fn(u256) -> I,
        cost: u64,
    ) -> Result<(Pc<'a, STEPPABLE>, Gas, Stack<'s>), FailStatus> {
        gas_left.consume(cost)?;
        let (push_location, [value]) = stack.pop_with_location()?;
        push_location.push(op(value));
        pc.next();
        Ok((pc, gas_left, stack))
    }

    /// Performs an operation that charges `cost` gas, pops two values and pushes `op` applied to
    /// them. `op` receives the top of stack as its first argument.
    #[inline(always)]
    fn binary_op<'a, 's, I: Into<u256>>(
        mut pc: Pc<'a, STEPPABLE>,
        mut gas_left: Gas,
        mut stack: Stack<'s>,
        op: fn(u256, u256) -> I,
        cost: u64,
    ) -> Result<(Pc<'a, STEPPABLE>, Gas, Stack<'s>), FailStatus> {
        gas_left.consume(cost)?;
        let (push_location, [value2, value1]) = stack.pop_with_location()?;
        push_location.push(op(value1, value2));
        pc.next();
        Ok((pc, gas_left, stack))
    }

    /// Performs an operation that charges `cost` gas and pushes `op` applied to the interpreter.
    #[inline(always)]
    fn push_value_op<'a, 's, I: Into<u256>>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        mut pc: Pc<'a, STEPPABLE>,
        mut gas_left: Gas,
        mut stack: Stack<'s>,
        op: fn(&mut Interpreter<'a, STEPPABLE>) -> I,
        cost: u64,
    ) -> Result<(Pc<'a, STEPPABLE>, Gas, Stack<'s>), FailStatus> {
        gas_left.consume(cost)?;
        let value = op(interpreter);
        stack.push(value)?;
        pc.next();
        Ok((pc, gas_left, stack))
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    pub extern "C-unwind" fn no_op<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        mut pc: Pc<'a, STEPPABLE>,
        gas_left: Gas,
        stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        pc.next();
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    pub extern "C-unwind" fn skip_no_ops<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        mut pc: Pc<'a, STEPPABLE>,
        gas_left: Gas,
        stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        pc.jump_to_jumpdest();
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn stop<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        pc: Pc<'a, STEPPABLE>,
        gas_left: Gas,
        stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        interpreter.exec_status = ExecStatus::Stopped;
        interpreter.exit(pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn add<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        pc: Pc<'a, STEPPABLE>,
        gas_left: Gas,
        stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        let (pc, gas_left, stack) = Self::binary_op(pc, gas_left, stack, u256::add, 3)?;
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn mul<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        pc: Pc<'a, STEPPABLE>,
        gas_left: Gas,
        stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        let (pc, gas_left, stack) = Self::binary_op(pc, gas_left, stack, u256::mul, 5)?;
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn sub<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        pc: Pc<'a, STEPPABLE>,
        gas_left: Gas,
        stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        let (pc, gas_left, stack) = Self::binary_op(pc, gas_left, stack, u256::sub, 3)?;
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn div<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        pc: Pc<'a, STEPPABLE>,
        gas_left: Gas,
        stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        let (pc, gas_left, stack) = Self::binary_op(pc, gas_left, stack, u256::div, 5)?;
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn s_div<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        pc: Pc<'a, STEPPABLE>,
        gas_left: Gas,
        stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        let (pc, gas_left, stack) = Self::binary_op(pc, gas_left, stack, u256::sdiv, 5)?;
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn mod_<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        pc: Pc<'a, STEPPABLE>,
        gas_left: Gas,
        stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        let (pc, gas_left, stack) = Self::binary_op(pc, gas_left, stack, u256::rem, 5)?;
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn s_mod<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        pc: Pc<'a, STEPPABLE>,
        gas_left: Gas,
        stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        let (pc, gas_left, stack) = Self::binary_op(pc, gas_left, stack, u256::srem, 5)?;
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn add_mod<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        mut pc: Pc<'a, STEPPABLE>,
        mut gas_left: Gas,
        mut stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        gas_left.consume(8)?;
        let (push_location, [denominator, value2, value1]) = stack.pop_with_location()?;
        push_location.push(u256::addmod(value1, value2, denominator));
        pc.next();
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn mul_mod<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        mut pc: Pc<'a, STEPPABLE>,
        mut gas_left: Gas,
        mut stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        gas_left.consume(8)?;
        let (push_location, [denominator, fac2, fac1]) = stack.pop_with_location()?;
        push_location.push(u256::mulmod(fac1, fac2, denominator));
        pc.next();
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn exp<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        mut pc: Pc<'a, STEPPABLE>,
        mut gas_left: Gas,
        mut stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        gas_left.consume(10)?;
        let (push_location, [exp, value]) = stack.pop_with_location()?;
        gas_left.consume(exp.bits().div_ceil(8) as u64 * 50)?; // * does not overflow
        push_location.push(value.pow(exp));
        pc.next();
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn sign_extend<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        pc: Pc<'a, STEPPABLE>,
        gas_left: Gas,
        stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        let (pc, gas_left, stack) = Self::binary_op(pc, gas_left, stack, u256::signextend, 5)?;
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn lt<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        pc: Pc<'a, STEPPABLE>,
        gas_left: Gas,
        stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        let (pc, gas_left, stack) = Self::binary_op(pc, gas_left, stack, |lhs, rhs| lhs < rhs, 3)?;
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn gt<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        pc: Pc<'a, STEPPABLE>,
        gas_left: Gas,
        stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        let (pc, gas_left, stack) = Self::binary_op(pc, gas_left, stack, |lhs, rhs| lhs > rhs, 3)?;
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn s_lt<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        pc: Pc<'a, STEPPABLE>,
        gas_left: Gas,
        stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        let (pc, gas_left, stack) =
            Self::binary_op(pc, gas_left, stack, |lhs, rhs| lhs.slt(&rhs), 3)?;
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn s_gt<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        pc: Pc<'a, STEPPABLE>,
        gas_left: Gas,
        stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        let (pc, gas_left, stack) =
            Self::binary_op(pc, gas_left, stack, |lhs, rhs| lhs.sgt(&rhs), 3)?;
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn eq<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        pc: Pc<'a, STEPPABLE>,
        gas_left: Gas,
        stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        let (pc, gas_left, stack) = Self::binary_op(pc, gas_left, stack, |lhs, rhs| lhs == rhs, 3)?;
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn is_zero<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        pc: Pc<'a, STEPPABLE>,
        gas_left: Gas,
        stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        let (pc, gas_left, stack) =
            Self::unary_op(pc, gas_left, stack, |value| value == u256::ZERO, 3)?;
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn and<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        pc: Pc<'a, STEPPABLE>,
        gas_left: Gas,
        stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        let (pc, gas_left, stack) = Self::binary_op(pc, gas_left, stack, u256::bitand, 3)?;
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn or<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        pc: Pc<'a, STEPPABLE>,
        gas_left: Gas,
        stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        let (pc, gas_left, stack) = Self::binary_op(pc, gas_left, stack, u256::bitor, 3)?;
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn xor<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        pc: Pc<'a, STEPPABLE>,
        gas_left: Gas,
        stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        let (pc, gas_left, stack) = Self::binary_op(pc, gas_left, stack, u256::bitxor, 3)?;
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn not<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        pc: Pc<'a, STEPPABLE>,
        gas_left: Gas,
        stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        let (pc, gas_left, stack) = Self::unary_op(pc, gas_left, stack, u256::not, 3)?;
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn byte<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        pc: Pc<'a, STEPPABLE>,
        gas_left: Gas,
        stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        let (pc, gas_left, stack) =
            Self::binary_op(pc, gas_left, stack, |offset, value| value.byte(offset), 3)?;
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn shl<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        pc: Pc<'a, STEPPABLE>,
        gas_left: Gas,
        stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        let (pc, gas_left, stack) =
            Self::binary_op(pc, gas_left, stack, |shift, value| value << shift, 3)?;
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn shr<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        pc: Pc<'a, STEPPABLE>,
        gas_left: Gas,
        stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        let (pc, gas_left, stack) =
            Self::binary_op(pc, gas_left, stack, |shift, value| value >> shift, 3)?;
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn sar<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        pc: Pc<'a, STEPPABLE>,
        gas_left: Gas,
        stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        let (pc, gas_left, stack) =
            Self::binary_op(pc, gas_left, stack, |shift, value| value.sar(shift), 3)?;
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn clz<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        pc: Pc<'a, STEPPABLE>,
        gas_left: Gas,
        stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        check_min_revision(Revision::EVMC_OSAKA, interpreter.revision)?;
        let (pc, gas_left, stack) =
            Self::unary_op(pc, gas_left, stack, |value| value.leading_zeros(), 5)?;
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn sha3<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        mut pc: Pc<'a, STEPPABLE>,
        mut gas_left: Gas,
        mut stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        gas_left.consume(30)?;
        let (push_location, [len, offset]) = stack.pop_with_location()?;

        let len = u64::try_from(len).map_err(|_| FailStatus::OutOfGas)?;
        gas_left.consume(6 * word_size(len)?)?; // * does not overflow

        let data = interpreter
            .memory
            .get_mut_slice(offset, len, &mut gas_left)?;
        push_location.push(interpreter.hash_cache.hash(data));
        pc.next();
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn address<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        pc: Pc<'a, STEPPABLE>,
        gas_left: Gas,
        stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        let (pc, gas_left, stack) = Self::push_value_op(
            interpreter,
            pc,
            gas_left,
            stack,
            |interpreter| *interpreter.message.recipient(),
            2,
        )?;
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn balance<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        mut pc: Pc<'a, STEPPABLE>,
        mut gas_left: Gas,
        mut stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        if interpreter.revision < Revision::EVMC_BERLIN {
            gas_left.consume(700)?;
        }
        let (push_location, [addr]) = stack.pop_with_location()?;
        let addr = addr.into();
        gas_left.consume_address_access_cost(&addr, interpreter.revision, interpreter.context)?;
        push_location.push(interpreter.context.get_balance(&addr));
        pc.next();
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn origin<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        pc: Pc<'a, STEPPABLE>,
        gas_left: Gas,
        stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        let (pc, gas_left, stack) = Self::push_value_op(
            interpreter,
            pc,
            gas_left,
            stack,
            |interpreter| interpreter.context.get_tx_context().tx_origin,
            2,
        )?;
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn caller<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        pc: Pc<'a, STEPPABLE>,
        gas_left: Gas,
        stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        let (pc, gas_left, stack) = Self::push_value_op(
            interpreter,
            pc,
            gas_left,
            stack,
            |interpreter| *interpreter.message.sender(),
            2,
        )?;
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn call_value<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        pc: Pc<'a, STEPPABLE>,
        gas_left: Gas,
        stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        let (pc, gas_left, stack) = Self::push_value_op(
            interpreter,
            pc,
            gas_left,
            stack,
            |interpreter| *interpreter.message.value(),
            2,
        )?;
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn call_data_load<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        mut pc: Pc<'a, STEPPABLE>,
        mut gas_left: Gas,
        mut stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        gas_left.consume(3)?;
        let (push_location, [offset]) = stack.pop_with_location()?;
        let (offset, overflow) = offset.into_u64_with_overflow();
        let offset = offset as usize;
        let call_data = interpreter.message.input();
        if overflow || offset >= call_data.len() {
            push_location.push(u256::ZERO);
        } else if let Some(word) = call_data[offset..].first_chunk() {
            push_location.push(u256::from_be_bytes(*word));
        } else {
            // offset < call_data.len() && call_data[offset..].len() < 32
            let mut bytes = [0; 32];
            let data = &call_data[offset..];
            bytes[..data.len()].copy_from_slice(data);
            push_location.push(u256::from_be_bytes(bytes));
        }
        pc.next();
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn call_data_size<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        pc: Pc<'a, STEPPABLE>,
        gas_left: Gas,
        stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        let (pc, gas_left, stack) = Self::push_value_op(
            interpreter,
            pc,
            gas_left,
            stack,
            |interpreter| interpreter.message.input().len(),
            2,
        )?;
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn push0<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        pc: Pc<'a, STEPPABLE>,
        gas_left: Gas,
        stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        check_min_revision(Revision::EVMC_SHANGHAI, interpreter.revision)?;
        let (pc, gas_left, stack) =
            Self::push_value_op(interpreter, pc, gas_left, stack, |_| u256::ZERO, 2)?;
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn call_data_copy<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        mut pc: Pc<'a, STEPPABLE>,
        mut gas_left: Gas,
        mut stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        gas_left.consume(3)?;
        let [len, offset, dest_offset] = stack.pop()?;

        if len != u256::ZERO {
            let len = u64::try_from(len).map_err(|_| FailStatus::InvalidMemoryAccess)?;

            let src = interpreter.message.input().get_within_bounds(offset, len);
            let dest = interpreter
                .memory
                .get_mut_slice(dest_offset, len, &mut gas_left)?;
            dest.copy_padded(src, &mut gas_left)?;
        }
        pc.next();
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn code_size<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        pc: Pc<'a, STEPPABLE>,
        gas_left: Gas,
        stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        let (pc, gas_left, stack) = Self::push_value_op(
            interpreter,
            pc,
            gas_left,
            stack,
            |interpreter| interpreter.code.len(),
            2,
        )?;
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn code_copy<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        mut pc: Pc<'a, STEPPABLE>,
        mut gas_left: Gas,
        mut stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        gas_left.consume(3)?;
        let [len, offset, dest_offset] = stack.pop()?;

        if len != u256::ZERO {
            let len = u64::try_from(len).map_err(|_| FailStatus::OutOfGas)?;

            let src = interpreter.code.get_within_bounds(offset, len);
            let dest = interpreter
                .memory
                .get_mut_slice(dest_offset, len, &mut gas_left)?;
            dest.copy_padded(src, &mut gas_left)?;
        }
        pc.next();
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn gas_price<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        pc: Pc<'a, STEPPABLE>,
        gas_left: Gas,
        stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        let (pc, gas_left, stack) = Self::push_value_op(
            interpreter,
            pc,
            gas_left,
            stack,
            |interpreter| interpreter.context.get_tx_context().tx_gas_price,
            2,
        )?;
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn ext_code_size<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        mut pc: Pc<'a, STEPPABLE>,
        mut gas_left: Gas,
        mut stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        if interpreter.revision < Revision::EVMC_BERLIN {
            gas_left.consume(700)?;
        }
        let (push_location, [addr]) = stack.pop_with_location()?;
        let addr = addr.into();
        gas_left.consume_address_access_cost(&addr, interpreter.revision, interpreter.context)?;
        push_location.push(interpreter.context.get_code_size(&addr));
        pc.next();
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn ext_code_copy<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        mut pc: Pc<'a, STEPPABLE>,
        mut gas_left: Gas,
        mut stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        if interpreter.revision < Revision::EVMC_BERLIN {
            gas_left.consume(700)?;
        }
        let [len, offset, dest_offset, addr] = stack.pop()?;
        let addr = addr.into();

        gas_left.consume_address_access_cost(&addr, interpreter.revision, interpreter.context)?;
        if len != u256::ZERO {
            let len = u64::try_from(len).map_err(|_| FailStatus::OutOfGas)?;

            let dest = interpreter
                .memory
                .get_mut_slice(dest_offset, len, &mut gas_left)?;
            let (offset, offset_overflow) = offset.into_u64_with_overflow();
            gas_left.consume_copy_cost(len)?;
            let bytes_written = interpreter.context.copy_code(&addr, offset as usize, dest);
            if offset_overflow {
                dest.fill(0);
            } else if (bytes_written as u64) < len {
                dest[bytes_written..].fill(0);
            }
        }
        pc.next();
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn return_data_size<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        pc: Pc<'a, STEPPABLE>,
        gas_left: Gas,
        stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        let (pc, gas_left, stack) = Self::push_value_op(
            interpreter,
            pc,
            gas_left,
            stack,
            |interpreter| interpreter.last_call_return_data.len(),
            2,
        )?;
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn return_data_copy<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        mut pc: Pc<'a, STEPPABLE>,
        mut gas_left: Gas,
        mut stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        gas_left.consume(3)?;
        let [len, offset, dest_offset] = stack.pop()?;

        let src = &interpreter.last_call_return_data;
        let (offset, offset_overflow) = offset.into_u64_with_overflow();
        let (len, len_overflow) = len.into_u64_with_overflow();
        let (end, end_overflow) = offset.overflowing_add(len);
        if offset_overflow || len_overflow || end_overflow || end > src.len() as u64 {
            std::hint::cold_path();
            return Err(Exit::Failed(FailStatus::InvalidMemoryAccess));
        }

        if len != 0 {
            let src = &src[offset as usize..end as usize];
            let dest = interpreter
                .memory
                .get_mut_slice(dest_offset, len, &mut gas_left)?;
            dest.copy_padded(src, &mut gas_left)?;
        }
        pc.next();
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn ext_code_hash<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        mut pc: Pc<'a, STEPPABLE>,
        mut gas_left: Gas,
        mut stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        if interpreter.revision < Revision::EVMC_BERLIN {
            gas_left.consume(700)?;
        }
        let (push_location, [addr]) = stack.pop_with_location()?;
        let addr = addr.into();
        gas_left.consume_address_access_cost(&addr, interpreter.revision, interpreter.context)?;
        push_location.push(interpreter.context.get_code_hash(&addr));
        pc.next();
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn block_hash<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        mut pc: Pc<'a, STEPPABLE>,
        mut gas_left: Gas,
        mut stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        gas_left.consume(20)?;
        let (push_location, [block_number]) = stack.pop_with_location()?;
        let current_block_number = interpreter
            .context
            .get_tx_context()
            .block_number
            .cast_unsigned();
        let block_hash = if let Ok(idx) = u64::try_from(block_number)
            && idx < current_block_number
            && current_block_number - idx <= 256
        {
            interpreter.context.get_block_hash(idx.cast_signed()).into()
        } else {
            u256::ZERO
        };
        push_location.push(block_hash);
        pc.next();
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn coinbase<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        pc: Pc<'a, STEPPABLE>,
        gas_left: Gas,
        stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        let (pc, gas_left, stack) = Self::push_value_op(
            interpreter,
            pc,
            gas_left,
            stack,
            |interpreter| interpreter.context.get_tx_context().block_coinbase,
            2,
        )?;
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn timestamp<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        pc: Pc<'a, STEPPABLE>,
        gas_left: Gas,
        stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        let (pc, gas_left, stack) = Self::push_value_op(
            interpreter,
            pc,
            gas_left,
            stack,
            |interpreter| {
                interpreter
                    .context
                    .get_tx_context()
                    .block_timestamp
                    .cast_unsigned()
            },
            2,
        )?;
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn number<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        pc: Pc<'a, STEPPABLE>,
        gas_left: Gas,
        stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        let (pc, gas_left, stack) = Self::push_value_op(
            interpreter,
            pc,
            gas_left,
            stack,
            |interpreter| {
                interpreter
                    .context
                    .get_tx_context()
                    .block_number
                    .cast_unsigned()
            },
            2,
        )?;
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn prev_randao<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        pc: Pc<'a, STEPPABLE>,
        gas_left: Gas,
        stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        let (pc, gas_left, stack) = Self::push_value_op(
            interpreter,
            pc,
            gas_left,
            stack,
            |interpreter| interpreter.context.get_tx_context().block_prev_randao,
            2,
        )?;
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn gas_limit<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        pc: Pc<'a, STEPPABLE>,
        gas_left: Gas,
        stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        let (pc, gas_left, stack) = Self::push_value_op(
            interpreter,
            pc,
            gas_left,
            stack,
            |interpreter| {
                interpreter
                    .context
                    .get_tx_context()
                    .block_gas_limit
                    .cast_unsigned()
            },
            2,
        )?;
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn chain_id<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        pc: Pc<'a, STEPPABLE>,
        gas_left: Gas,
        stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        let (pc, gas_left, stack) = Self::push_value_op(
            interpreter,
            pc,
            gas_left,
            stack,
            |interpreter| interpreter.context.get_tx_context().chain_id,
            2,
        )?;
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn self_balance<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        mut pc: Pc<'a, STEPPABLE>,
        mut gas_left: Gas,
        mut stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        check_min_revision(Revision::EVMC_ISTANBUL, interpreter.revision)?;
        gas_left.consume(5)?;
        stack.check_overflow(1)?; // Check for stack overflow before querying the host
        let addr = *interpreter.message.recipient();
        stack.push(interpreter.context.get_balance(&addr))?;
        pc.next();
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn base_fee<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        pc: Pc<'a, STEPPABLE>,
        gas_left: Gas,
        stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        check_min_revision(Revision::EVMC_LONDON, interpreter.revision)?;
        let (pc, gas_left, stack) = Self::push_value_op(
            interpreter,
            pc,
            gas_left,
            stack,
            |interpreter| interpreter.context.get_tx_context().block_base_fee,
            2,
        )?;
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn blob_hash<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        mut pc: Pc<'a, STEPPABLE>,
        mut gas_left: Gas,
        mut stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        check_min_revision(Revision::EVMC_CANCUN, interpreter.revision)?;
        gas_left.consume(3)?;
        let (push_location, [idx]) = stack.pop_with_location()?;
        let hashes = interpreter.context.get_tx_context().blob_hashes;
        if let Ok(idx) = u64::try_from(idx)
            && let Some(hash) = hashes.get(idx as usize)
        {
            push_location.push(*hash);
        } else {
            push_location.push(u256::ZERO);
        }
        pc.next();
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn blob_base_fee<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        pc: Pc<'a, STEPPABLE>,
        gas_left: Gas,
        stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        check_min_revision(Revision::EVMC_CANCUN, interpreter.revision)?;
        let (pc, gas_left, stack) = Self::push_value_op(
            interpreter,
            pc,
            gas_left,
            stack,
            |interpreter| interpreter.context.get_tx_context().blob_base_fee,
            2,
        )?;
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn pop<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        mut pc: Pc<'a, STEPPABLE>,
        mut gas_left: Gas,
        mut stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        gas_left.consume(2)?;
        let [_] = stack.pop()?;
        pc.next();
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn m_load<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        mut pc: Pc<'a, STEPPABLE>,
        mut gas_left: Gas,
        mut stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        gas_left.consume(3)?;
        let (push_location, [offset]) = stack.pop_with_location()?;

        push_location.push(interpreter.memory.get_word(offset, &mut gas_left)?);
        pc.next();
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn m_store<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        mut pc: Pc<'a, STEPPABLE>,
        mut gas_left: Gas,
        mut stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        gas_left.consume(3)?;
        let [value, offset] = stack.pop()?;

        *interpreter.memory.get_mut_array(offset, &mut gas_left)? = value.to_be_bytes();
        pc.next();
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn m_store8<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        mut pc: Pc<'a, STEPPABLE>,
        mut gas_left: Gas,
        mut stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        gas_left.consume(3)?;
        let [value, offset] = stack.pop()?;

        let dest = interpreter.memory.get_mut_byte(offset, &mut gas_left)?;
        *dest = value.least_significant_byte();
        pc.next();
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn s_load<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        mut pc: Pc<'a, STEPPABLE>,
        mut gas_left: Gas,
        mut stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        if interpreter.revision < Revision::EVMC_BERLIN {
            gas_left.consume(800)?;
        }
        let (push_location, [key]) = stack.pop_with_location()?;
        let key = key.into();
        let addr = interpreter.message.recipient();
        if interpreter.revision >= Revision::EVMC_BERLIN {
            if interpreter.context.access_storage(addr, &key) == AccessStatus::EVMC_ACCESS_COLD {
                gas_left.consume(2_100)?;
            } else {
                gas_left.consume(100)?;
            }
        }
        let value = interpreter.context.get_storage(addr, &key);
        push_location.push(value);
        pc.next();
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn jump<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        mut pc: Pc<'a, STEPPABLE>,
        mut gas_left: Gas,
        mut stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        gas_left.consume(if STEPPABLE { 8 } else { 8 + 1 })?;
        let [dest] = stack.pop()?;
        interpreter.code.try_jump(&mut pc, dest)?;
        if !STEPPABLE {
            pc.next();
        }
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn jump_i<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        mut pc: Pc<'a, STEPPABLE>,
        mut gas_left: Gas,
        mut stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        gas_left.consume(10)?;
        let [cond, dest] = stack.pop()?;
        if cond == u256::ZERO {
            pc.next();
        } else {
            interpreter.code.try_jump(&mut pc, dest)?;
            if !STEPPABLE {
                gas_left.consume(1)?;
                pc.next();
            }
        }
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn pc<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        mut pc: Pc<'a, STEPPABLE>,
        mut gas_left: Gas,
        mut stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        gas_left.consume(2)?;
        stack.push(pc.code_offset())?;
        pc.next();
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn m_size<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        pc: Pc<'a, STEPPABLE>,
        gas_left: Gas,
        stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        let (pc, gas_left, stack) = Self::push_value_op(
            interpreter,
            pc,
            gas_left,
            stack,
            |interpreter| interpreter.memory.len(),
            2,
        )?;
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn gas<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        mut pc: Pc<'a, STEPPABLE>,
        mut gas_left: Gas,
        mut stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        gas_left.consume(2)?;
        stack.push(gas_left.as_u64())?;
        pc.next();
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn jump_dest<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        mut pc: Pc<'a, STEPPABLE>,
        mut gas_left: Gas,
        stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        gas_left.consume(1)?;
        pc.next();
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn t_load<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        mut pc: Pc<'a, STEPPABLE>,
        mut gas_left: Gas,
        mut stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        check_min_revision(Revision::EVMC_CANCUN, interpreter.revision)?;
        gas_left.consume(100)?;
        let (push_location, [key]) = stack.pop_with_location()?;
        let value = interpreter
            .context
            .get_transient_storage(interpreter.message.recipient(), &key.into());
        push_location.push(value);
        pc.next();
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn t_store<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        mut pc: Pc<'a, STEPPABLE>,
        mut gas_left: Gas,
        mut stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        check_min_revision(Revision::EVMC_CANCUN, interpreter.revision)?;
        check_not_read_only(interpreter.message)?;
        gas_left.consume(100)?;
        let [value, key] = stack.pop()?;
        interpreter.context.set_transient_storage(
            interpreter.message.recipient(),
            &key.into(),
            &value.into(),
        );
        pc.next();
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn m_copy<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        mut pc: Pc<'a, STEPPABLE>,
        mut gas_left: Gas,
        mut stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        check_min_revision(Revision::EVMC_CANCUN, interpreter.revision)?;
        gas_left.consume(3)?;
        let [len, offset, dest_offset] = stack.pop()?;
        if len != u256::ZERO {
            interpreter
                .memory
                .copy_within(offset, dest_offset, len, &mut gas_left)?;
        }
        pc.next();
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn return_<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        pc: Pc<'a, STEPPABLE>,
        mut gas_left: Gas,
        mut stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        let [len, offset] = stack.pop()?;
        let len = u64::try_from(len).map_err(|_| FailStatus::OutOfGas)?;
        let data = interpreter
            .memory
            .get_mut_slice(offset, len, &mut gas_left)?;
        interpreter.output = Box::from(&*data);
        interpreter.exec_status = ExecStatus::Returned;
        interpreter.exit(pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn revert<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        pc: Pc<'a, STEPPABLE>,
        mut gas_left: Gas,
        mut stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        let [len, offset] = stack.pop()?;
        let len = u64::try_from(len).map_err(|_| FailStatus::OutOfGas)?;
        let data = interpreter
            .memory
            .get_mut_slice(offset, len, &mut gas_left)?;
        interpreter.output = Box::from(&*data);
        interpreter.exec_status = ExecStatus::Revert;
        interpreter.exit(pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    #[cold]
    extern "C-unwind" fn invalid<'a>(
        _interpreter: &mut Interpreter<'a, STEPPABLE>,
        _pc: Pc<'a, STEPPABLE>,
        _gas_left: Gas,
        _stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        Err(Exit::Failed(FailStatus::InvalidInstruction))
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn self_destruct<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        pc: Pc<'a, STEPPABLE>,
        mut gas_left: Gas,
        mut stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        check_not_read_only(interpreter.message)?;
        gas_left.consume(5_000)?;
        let [addr] = stack.pop()?;
        let addr = addr.into();

        if interpreter.revision >= Revision::EVMC_BERLIN
            && interpreter.context.access_account(&addr) == AccessStatus::EVMC_ACCESS_COLD
        {
            gas_left.consume(2_600)?;
        }

        if u256::from(
            interpreter
                .context
                .get_balance(interpreter.message.recipient()),
        ) > u256::ZERO
            && !interpreter.context.account_exists(&addr)
        {
            gas_left.consume(25_000)?;
        }

        let destructed = interpreter
            .context
            .selfdestruct(interpreter.message.recipient(), &addr);
        if interpreter.revision <= Revision::EVMC_BERLIN && destructed {
            interpreter.gas_refund.add(24_000);
        }

        interpreter.exec_status = ExecStatus::Stopped;
        interpreter.exit(pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn sstore<'a>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        mut pc: Pc<'a, STEPPABLE>,
        mut gas_left: Gas,
        mut stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        check_not_read_only(interpreter.message)?;

        if interpreter.revision >= Revision::EVMC_ISTANBUL && gas_left <= 2_300 {
            std::hint::cold_path();
            return Err(Exit::Failed(FailStatus::OutOfGas));
        }
        let [value, key] = stack.pop()?;
        let key = key.into();
        let addr = interpreter.message.recipient();

        let (dyn_gas_1, dyn_gas_2, dyn_gas_3, refund_1, refund_2, refund_3) =
            if interpreter.revision >= Revision::EVMC_LONDON {
                (100, 2_900, 20_000, 5_000 - 2_100 - 100, 4_800, 20_000 - 100)
            } else if interpreter.revision >= Revision::EVMC_BERLIN {
                (
                    100,
                    2_900,
                    20_000,
                    5_000 - 2_100 - 100,
                    15_000,
                    20_000 - 100,
                )
            } else if interpreter.revision >= Revision::EVMC_ISTANBUL {
                (800, 5_000, 20_000, 4_200, 15_000, 19_200)
            } else {
                (5_000, 5_000, 20_000, 0, 0, 0)
            };

        let status = interpreter.context.set_storage(addr, &key, &value.into());
        let (mut dyn_gas, gas_refund_change) = match status {
            StorageStatus::EVMC_STORAGE_ASSIGNED => (dyn_gas_1, 0),
            StorageStatus::EVMC_STORAGE_ADDED => (dyn_gas_3, 0),
            StorageStatus::EVMC_STORAGE_DELETED => (dyn_gas_2, refund_2),
            StorageStatus::EVMC_STORAGE_MODIFIED => (dyn_gas_2, 0),
            StorageStatus::EVMC_STORAGE_DELETED_ADDED => (dyn_gas_1, -refund_2),
            StorageStatus::EVMC_STORAGE_MODIFIED_DELETED => (dyn_gas_1, refund_2),
            StorageStatus::EVMC_STORAGE_DELETED_RESTORED => (dyn_gas_1, -refund_2 + refund_1),
            StorageStatus::EVMC_STORAGE_ADDED_DELETED => (dyn_gas_1, refund_3),
            StorageStatus::EVMC_STORAGE_MODIFIED_RESTORED => (dyn_gas_1, refund_1),
        };
        if interpreter.revision >= Revision::EVMC_BERLIN
            && interpreter.context.access_storage(addr, &key) == AccessStatus::EVMC_ACCESS_COLD
        {
            dyn_gas += 2_100;
        }
        gas_left.consume(dyn_gas)?;
        interpreter.gas_refund.add(gas_refund_change);
        pc.next();
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn push<'a, const N: usize>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        mut pc: Pc<'a, STEPPABLE>,
        mut gas_left: Gas,
        mut stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        gas_left.consume(3)?;
        std::cfg_select! {
            feature = "fn-ptr-conversion-dispatch" => {
                stack.push(pc.get_push_data())?;
            }
            _ => {
                pc.next();
                stack.push(interpreter.code.get_push_data::<N>(&mut pc))?;
            }
        }
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn dup<'a, const N: usize>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        mut pc: Pc<'a, STEPPABLE>,
        mut gas_left: Gas,
        mut stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        gas_left.consume(3)?;
        stack.dup::<N>()?;
        pc.next();
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn swap<'a, const N: usize>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        mut pc: Pc<'a, STEPPABLE>,
        mut gas_left: Gas,
        mut stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        gas_left.consume(3)?;
        stack.swap_with_top::<N>()?;
        pc.next();
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn log<'a, const N: usize>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        mut pc: Pc<'a, STEPPABLE>,
        mut gas_left: Gas,
        mut stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        check_not_read_only(interpreter.message)?;
        gas_left.consume(375)?;
        let [len, offset] = stack.pop()?;
        let topics: [u256; N] = stack.pop()?;
        let (len, len_overflow) = len.into_u64_with_overflow();
        let (len8, len8_overflow) = len.overflowing_mul(8);
        let (cost, cost_overflow) = (375 * N as u64).overflowing_add(len8);
        if len_overflow || len8_overflow || cost_overflow {
            std::hint::cold_path();
            return Err(Exit::Failed(FailStatus::OutOfGas));
        }
        gas_left.consume(cost)?;

        let data = interpreter
            .memory
            .get_mut_slice(offset, len, &mut gas_left)?;
        let mut topics_uint256 = topics.map(Uint256::from);
        topics_uint256.reverse();
        interpreter
            .context
            .emit_log(interpreter.message.recipient(), data, &topics_uint256);
        pc.next();
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn create_or_create2<'a, const CREATE2: bool>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        mut pc: Pc<'a, STEPPABLE>,
        mut gas_left: Gas,
        mut stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        gas_left.consume(32_000)?;
        check_not_read_only(interpreter.message)?;
        let [len, offset, value] = stack.pop()?;
        let salt = if CREATE2 {
            let [salt] = stack.pop()?;
            salt
        } else {
            u256::ZERO // ignored
        };
        let len = u64::try_from(len).map_err(|_| FailStatus::OutOfGas)?;

        let init_code_word_size = word_size(len)?;
        if interpreter.revision >= Revision::EVMC_SHANGHAI {
            const MAX_INIT_CODE_LEN: u64 = 2 * 24576;
            if len > MAX_INIT_CODE_LEN {
                std::hint::cold_path();
                return Err(Exit::Failed(FailStatus::OutOfGas));
            }
            let init_code_cost = 2 * init_code_word_size; // does not overflow
            gas_left.consume(init_code_cost)?;
        }
        if CREATE2 {
            let hash_cost = 6 * init_code_word_size; // does not overflow
            gas_left.consume(hash_cost)?;
        }

        let init_code = interpreter
            .memory
            .get_mut_slice(offset, len, &mut gas_left)?;

        if value
            > interpreter
                .context
                .get_balance(interpreter.message.recipient())
                .into()
        {
            interpreter.last_call_return_data = LastCallReturnData::Slice(&[]);
            stack.push(u256::ZERO)?;
            pc.next();
            next!(interpreter, pc, gas_left, stack);
        }

        let gas_limit = gas_left.as_u64() - gas_left.as_u64() / 64;
        gas_left.consume(gas_limit)?;

        let message = ExecutionMessage::new(
            if CREATE2 {
                MessageKind::EVMC_CREATE2
            } else {
                MessageKind::EVMC_CREATE
            },
            interpreter.message.flags(),
            interpreter.message.depth() + 1,
            gas_limit.cast_signed(),
            u256::ZERO.into(), // ignored
            *interpreter.message.recipient(),
            init_code,
            value.into(),
            salt.into(),
            u256::ZERO.into(), // ignored
            &[],
            None,
        );
        let result = interpreter.context.call(&message);

        gas_left.add(result.gas_left())?;
        interpreter.gas_refund.add(result.gas_refund());

        if result.status_code() == StatusCode::EVMC_SUCCESS {
            interpreter.last_call_return_data = LastCallReturnData::Slice(&[]);
            stack.push(result.create_address())?;
        } else {
            interpreter.last_call_return_data = LastCallReturnData::CallResult(result);
            stack.push(u256::ZERO)?;
        }
        pc.next();
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn call_or_call_code<'a, const CODE: bool>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        mut pc: Pc<'a, STEPPABLE>,
        mut gas_left: Gas,
        mut stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        if interpreter.revision < Revision::EVMC_BERLIN {
            gas_left.consume(700)?;
        }
        let [ret_len, ret_offset, args_len, args_offset, value, addr, gas] = stack.pop()?;

        if !CODE && value != u256::ZERO {
            check_not_read_only(interpreter.message)?;
        }

        let addr = addr.into();
        let args_len = u64::try_from(args_len).map_err(|_| FailStatus::OutOfGas)?;
        let ret_len = u64::try_from(ret_len).map_err(|_| FailStatus::OutOfGas)?;

        gas_left.consume_address_access_cost(&addr, interpreter.revision, interpreter.context)?;
        gas_left.consume_positive_value_cost(&value)?;
        if !CODE {
            gas_left.consume_value_to_empty_account_cost(&value, &addr, interpreter.context)?;
        }
        gas_left.consume_delegate_resolution_cost(
            &addr,
            interpreter.revision,
            interpreter.context,
        )?;
        // access slice to consume potential memory expansion cost but drop it so that we can get
        // another mutable reference into memory for input
        let _dest = interpreter
            .memory
            .get_mut_slice(ret_offset, ret_len, &mut gas_left)?;
        let input = interpreter
            .memory
            .get_mut_slice(args_offset, args_len, &mut gas_left)?;

        let limit = gas_left.as_u64() - gas_left.as_u64() / 64;
        let mut endowment = gas.into_u64_saturating();
        endowment = min(endowment, limit); // cap gas at all but one 64th of gas left

        let stipend: u64 = if value == u256::ZERO { 0 } else { 2_300 };
        gas_left.add(stipend.cast_signed())?;

        if value
            > u256::from(
                interpreter
                    .context
                    .get_balance(interpreter.message.recipient()),
            )
        {
            interpreter.last_call_return_data = LastCallReturnData::Slice(&[]);
            stack.push(u256::ZERO)?;
            pc.next();
            next!(interpreter, pc, gas_left, stack);
        }

        let call_message = if CODE {
            ExecutionMessage::new(
                MessageKind::EVMC_CALLCODE,
                interpreter.message.flags(),
                interpreter.message.depth() + 1,
                (endowment + stipend).cast_signed(),
                *interpreter.message.recipient(),
                *interpreter.message.recipient(),
                input,
                value.into(),
                u256::ZERO.into(), // ignored
                addr,
                &[],
                None,
            )
        } else {
            ExecutionMessage::new(
                MessageKind::EVMC_CALL,
                interpreter.message.flags(),
                interpreter.message.depth() + 1,
                (endowment + stipend).cast_signed(),
                addr,
                *interpreter.message.recipient(),
                input,
                value.into(),
                u256::ZERO.into(), // ignored
                addr,
                &[],
                None,
            )
        };

        let result = interpreter.context.call(&call_message);
        let (status_code, call_gas_left, call_gas_refund) =
            (result.status_code(), result.gas_left(), result.gas_refund());
        interpreter.last_call_return_data = LastCallReturnData::CallResult(result);
        let dest = interpreter
            .memory
            .get_mut_slice(ret_offset, ret_len, &mut gas_left)?;
        let output = &interpreter.last_call_return_data;
        let min_len = min(output.len(), ret_len as usize); // ret_len == dest.len()
        dest[..min_len].copy_from_slice(&output[..min_len]);

        // Deduct before crediting (like geth) so that the gas left never exceeds Gas::MAX. None can
        // fail: the endowment fits the gas left, the stipend was added above, and the callee
        // returns at most both.
        gas_left.consume(endowment)?;
        gas_left.consume(stipend)?;
        gas_left.add(call_gas_left)?;
        interpreter.gas_refund.add(call_gas_refund);

        stack.push(status_code == StatusCode::EVMC_SUCCESS)?;
        pc.next();
        next!(interpreter, pc, gas_left, stack)
    }

    #[cfg_attr(feature = "tail-call", preserve_none)]
    extern "C-unwind" fn static_or_delegate_call<'a, const DELEGATE: bool>(
        interpreter: &mut Interpreter<'a, STEPPABLE>,
        mut pc: Pc<'a, STEPPABLE>,
        mut gas_left: Gas,
        mut stack: Stack<'_>,
    ) -> OpFnResult<'a, STEPPABLE> {
        if interpreter.revision < Revision::EVMC_BERLIN {
            gas_left.consume(700)?;
        }
        let [ret_len, ret_offset, args_len, args_offset, addr, gas] = stack.pop()?;

        let addr = addr.into();
        let args_len = u64::try_from(args_len).map_err(|_| FailStatus::OutOfGas)?;
        let ret_len = u64::try_from(ret_len).map_err(|_| FailStatus::OutOfGas)?;

        gas_left.consume_address_access_cost(&addr, interpreter.revision, interpreter.context)?;
        gas_left.consume_delegate_resolution_cost(
            &addr,
            interpreter.revision,
            interpreter.context,
        )?;
        // access slice to consume potential memory expansion cost but drop it so that we can get
        // another mutable reference into memory for input
        let _dest = interpreter
            .memory
            .get_mut_slice(ret_offset, ret_len, &mut gas_left)?;
        let input = interpreter
            .memory
            .get_mut_slice(args_offset, args_len, &mut gas_left)?;

        let limit = gas_left.as_u64() - gas_left.as_u64() / 64;
        let mut endowment = gas.into_u64_saturating();
        endowment = min(endowment, limit); // cap gas at all but one 64th of gas left

        let call_message = if DELEGATE {
            ExecutionMessage::new(
                MessageKind::EVMC_DELEGATECALL,
                interpreter.message.flags(),
                interpreter.message.depth() + 1,
                endowment.cast_signed(),
                *interpreter.message.recipient(),
                *interpreter.message.sender(),
                input,
                *interpreter.message.value(),
                u256::ZERO.into(), // ignored
                addr,
                &[],
                None,
            )
        } else {
            ExecutionMessage::new(
                MessageKind::EVMC_CALL,
                MessageFlags::EVMC_STATIC as u32,
                interpreter.message.depth() + 1,
                endowment.cast_signed(),
                addr,
                *interpreter.message.recipient(),
                input,
                u256::ZERO.into(), // ignored
                u256::ZERO.into(), // ignored
                addr,
                &[],
                None,
            )
        };

        let result = interpreter.context.call(&call_message);
        let (status_code, call_gas_left, call_gas_refund) =
            (result.status_code(), result.gas_left(), result.gas_refund());
        interpreter.last_call_return_data = LastCallReturnData::CallResult(result);
        let dest = interpreter
            .memory
            .get_mut_slice(ret_offset, ret_len, &mut gas_left)?;
        let output = &interpreter.last_call_return_data;
        let min_len = min(output.len(), ret_len as usize); // ret_len == dest.len()
        dest[..min_len].copy_from_slice(&output[..min_len]);

        // Deduct before crediting (like geth) so that the gas left never exceeds Gas::MAX. Neither
        // can fail: the endowment fits the gas left, and the callee returns at most the endowment.
        gas_left.consume(endowment)?;
        gas_left.add(call_gas_left)?;
        interpreter.gas_refund.add(call_gas_refund);

        stack.push(status_code == StatusCode::EVMC_SUCCESS)?;
        pc.next();
        next!(interpreter, pc, gas_left, stack)
    }
}

impl<const STEPPABLE: bool> From<Interpreter<'_, STEPPABLE>> for StepResult {
    fn from(value: Interpreter<STEPPABLE>) -> Self {
        let stack = value
            .stack
            .map(|stack_buffer| {
                Stack::new(stack_buffer, value.parked.state)
                    .as_slice()
                    .iter()
                    .copied()
                    .map(Into::into)
                    .collect()
            })
            .unwrap_or_default();
        Self {
            step_status_code: value.exec_status.into(),
            status_code: StatusCode::EVMC_SUCCESS,
            revision: value.revision,
            pc: value.parked.pc.code_offset() as u64,
            gas_left: value.parked.state.gas().as_i64(),
            gas_refund: value.gas_refund.as_i64(),
            output: value.output,
            stack,
            memory: value.memory.as_slice().to_vec(),
            last_call_return_data: Box::from(&*value.last_call_return_data),
        }
    }
}

impl<const STEPPABLE: bool> From<Interpreter<'_, STEPPABLE>> for ExecutionResult {
    fn from(value: Interpreter<STEPPABLE>) -> Self {
        Self::new(
            value.exec_status.into(),
            value.parked.state.gas().as_i64(),
            value.gas_refund.as_i64(),
            value.output,
            Address::default(),
        )
    }
}

#[cfg(test)]
mod tests {
    use evmc_vm::{
        Address, ExecutionResult, MessageKind, Revision, StatusCode, StepResult, StepStatusCode,
        Uint256,
    };
    use mockall::predicate;

    use crate::{
        interpreter::Interpreter,
        types::{
            CodeAnalysis, CodeAnalysisCache, GasAndSize, Memory, MockExecutionContextTrait,
            MockExecutionMessage, NoOpObserver, Opcode, Stack, hash_cache::HashCache, u256,
        },
    };

    #[test]
    fn new_empties_stack_and_memory() {
        let code_analysis_cache = CodeAnalysisCache::default();
        let hash_cache = HashCache::default();
        let mut context = MockExecutionContextTrait::new();
        let message = MockExecutionMessage::default().into();
        let analysis = CodeAnalysis::new(&[], None, &code_analysis_cache);
        let (mut stack_buffer, mut memory) = (Stack::new_buffer(), Memory::new());
        memory.reset_to(&[9; 4]);
        let mut interpreter = Interpreter::new(
            Revision::EVMC_ISTANBUL,
            &message,
            &mut context,
            &[],
            &analysis,
            &mut stack_buffer,
            &mut memory,
            &hash_cache,
        )
        .unwrap();
        let stack_buffer = interpreter.stack.take().unwrap();
        assert_eq!(
            Stack::new(stack_buffer, interpreter.parked.state).as_slice(),
            []
        );
        assert_eq!(interpreter.memory.as_slice(), []);
    }

    #[test]
    fn new_steppable_resets_stack_and_memory_to_initial_values() {
        let code_analysis_cache = CodeAnalysisCache::default();
        let hash_cache = HashCache::default();
        let mut context = MockExecutionContextTrait::new();
        let message = MockExecutionMessage::default().into();
        let stack = [u256::from(1u8), u256::from(2u8)];
        let analysis = CodeAnalysis::new(&[], None, &code_analysis_cache);
        let (mut stack_buffer, mut memory) = (Stack::new_buffer(), Memory::new());
        stack_buffer.fill(u256::MAX);
        memory.reset_to(&[9; 4]);
        let mut interpreter = Interpreter::new_steppable(
            Revision::EVMC_ISTANBUL,
            &message,
            &mut context,
            &[],
            0,
            0,
            &stack,
            &[3],
            &[],
            None,
            &analysis,
            &mut stack_buffer,
            &mut memory,
            &hash_cache,
        )
        .unwrap();
        let stack_buffer = interpreter.stack.take().unwrap();
        assert_eq!(
            Stack::new(stack_buffer, interpreter.parked.state).as_slice(),
            stack
        );
        assert_eq!(interpreter.memory.as_slice(), [3]);
    }

    #[test]
    fn empty_code() {
        let code_analysis_cache = CodeAnalysisCache::default();
        let hash_cache = HashCache::default();
        let mut context = MockExecutionContextTrait::new();
        let message = MockExecutionMessage::default().into();
        let analysis = CodeAnalysis::new(&[], None, &code_analysis_cache);
        let (mut stack_buffer, mut memory) = (Stack::new_buffer(), Memory::new());
        let interpreter = Interpreter::new(
            Revision::EVMC_ISTANBUL,
            &message,
            &mut context,
            &[],
            &analysis,
            &mut stack_buffer,
            &mut memory,
            &hash_cache,
        )
        .unwrap();
        let result: StepResult = interpreter.run(&mut NoOpObserver());
        assert_eq!(result.step_status_code, StepStatusCode::EVMC_STEP_STOPPED);
        assert_eq!(result.pc, 0);
        assert_eq!(
            result.gas_left,
            MockExecutionMessage::DEFAULT_INIT_GAS.cast_signed()
        );
    }

    #[test]
    fn pc_after_end() {
        let code_analysis_cache = CodeAnalysisCache::default();
        let hash_cache = HashCache::default();
        let mut context = MockExecutionContextTrait::new();
        let message = MockExecutionMessage::default().into();
        let code = [Opcode::Add as u8];
        let analysis = CodeAnalysis::new(&code, None, &code_analysis_cache);
        let (mut stack_buffer, mut memory) = (Stack::new_buffer(), Memory::new());
        let interpreter = Interpreter::new_steppable(
            Revision::EVMC_ISTANBUL,
            &message,
            &mut context,
            &code,
            1,
            0,
            &[],
            &[],
            &[],
            None,
            &analysis,
            &mut stack_buffer,
            &mut memory,
            &hash_cache,
        )
        .unwrap();
        let result: StepResult = interpreter.run(&mut NoOpObserver());
        assert_eq!(result.step_status_code, StepStatusCode::EVMC_STEP_STOPPED);
        assert_eq!(result.pc, 1);
        assert_eq!(
            result.gas_left,
            MockExecutionMessage::DEFAULT_INIT_GAS.cast_signed()
        );
    }

    // when features "fn-ptr-conversion-dispatch" is enabled this in undefined behavior
    #[cfg(not(feature = "fn-ptr-conversion-dispatch"))]
    #[test]
    fn pc_on_data() {
        let code_analysis_cache = CodeAnalysisCache::default();
        let hash_cache = HashCache::default();
        let mut context = MockExecutionContextTrait::new();
        let message = MockExecutionMessage::default().into();
        let code = [Opcode::Push1 as u8, 0x00];
        let analysis = CodeAnalysis::new(&code, None, &code_analysis_cache);
        let (mut stack_buffer, mut memory) = (Stack::new_buffer(), Memory::new());
        let result: ExecutionResult = Interpreter::new_steppable(
            Revision::EVMC_ISTANBUL,
            &message,
            &mut context,
            &code,
            1,
            0,
            &[],
            &[],
            &[],
            None,
            &analysis,
            &mut stack_buffer,
            &mut memory,
            &hash_cache,
        )
        .unwrap()
        .run(&mut NoOpObserver());
        assert_eq!(result.status_code(), StatusCode::EVMC_INVALID_INSTRUCTION);
    }

    #[test]
    fn zero_steps() {
        let code_analysis_cache = CodeAnalysisCache::default();
        let hash_cache = HashCache::default();
        let mut context = MockExecutionContextTrait::new();
        let message = MockExecutionMessage::default().into();
        let code = [Opcode::Add as u8];
        let analysis = CodeAnalysis::new(&code, None, &code_analysis_cache);
        let (mut stack_buffer, mut memory) = (Stack::new_buffer(), Memory::new());
        let interpreter = Interpreter::new_steppable(
            Revision::EVMC_ISTANBUL,
            &message,
            &mut context,
            &code,
            0,
            0,
            &[],
            &[],
            &[],
            Some(0),
            &analysis,
            &mut stack_buffer,
            &mut memory,
            &hash_cache,
        )
        .unwrap();
        let result: StepResult = interpreter.run(&mut NoOpObserver());
        assert_eq!(result.step_status_code, StepStatusCode::EVMC_STEP_RUNNING);
        assert_eq!(result.pc, 0);
        assert_eq!(
            result.gas_left,
            MockExecutionMessage::DEFAULT_INIT_GAS.cast_signed()
        );
    }

    #[test]
    fn add_one_step() {
        let code_analysis_cache = CodeAnalysisCache::default();
        let hash_cache = HashCache::default();
        let mut context = MockExecutionContextTrait::new();
        let message = MockExecutionMessage::default().into();
        let code = [Opcode::Add as u8, Opcode::Add as u8];
        let analysis = CodeAnalysis::new(&code, None, &code_analysis_cache);
        let (mut stack_buffer, mut memory) = (Stack::new_buffer(), Memory::new());
        let interpreter = Interpreter::new_steppable(
            Revision::EVMC_ISTANBUL,
            &message,
            &mut context,
            &code,
            0,
            0,
            &[1u8.into(), 2u8.into()],
            &[],
            &[],
            Some(1),
            &analysis,
            &mut stack_buffer,
            &mut memory,
            &hash_cache,
        )
        .unwrap();
        let result: StepResult = interpreter.run(&mut NoOpObserver());
        assert_eq!(result.step_status_code, StepStatusCode::EVMC_STEP_RUNNING);
        assert_eq!(result.stack.as_slice(), [u256::from(3u8).into()]);
        assert_eq!(
            result.gas_left,
            MockExecutionMessage::DEFAULT_INIT_GAS.cast_signed() - 3
        );
    }

    #[test]
    fn add_single_op() {
        let code_analysis_cache = CodeAnalysisCache::default();
        let hash_cache = HashCache::default();
        let mut context = MockExecutionContextTrait::new();
        let message = MockExecutionMessage::default().into();
        let code = [Opcode::Add as u8];
        let analysis = CodeAnalysis::new(&code, None, &code_analysis_cache);
        let (mut stack_buffer, mut memory) = (Stack::new_buffer(), Memory::new());
        let mut interpreter = Interpreter::new(
            Revision::EVMC_ISTANBUL,
            &message,
            &mut context,
            &code,
            &analysis,
            &mut stack_buffer,
            &mut memory,
            &hash_cache,
        )
        .unwrap();
        let mut stack = Stack::empty(interpreter.stack.as_mut().unwrap());
        stack.reset_to(&[1u8.into(), 2u8.into()]);
        interpreter.parked.state = GasAndSize::new(interpreter.parked.state.gas(), stack);
        let result: StepResult = interpreter.run(&mut NoOpObserver());
        assert_eq!(result.step_status_code, StepStatusCode::EVMC_STEP_STOPPED);
        assert_eq!(result.stack.as_slice(), [u256::from(3u8).into()]);
        assert_eq!(
            result.gas_left,
            MockExecutionMessage::DEFAULT_INIT_GAS.cast_signed() - 3
        );
    }

    #[test]
    fn add_twice() {
        let code_analysis_cache = CodeAnalysisCache::default();
        let hash_cache = HashCache::default();
        let mut context = MockExecutionContextTrait::new();
        let message = MockExecutionMessage::default().into();
        let code = [Opcode::Add as u8, Opcode::Add as u8];
        let analysis = CodeAnalysis::new(&code, None, &code_analysis_cache);
        let (mut stack_buffer, mut memory) = (Stack::new_buffer(), Memory::new());
        let mut interpreter = Interpreter::new(
            Revision::EVMC_ISTANBUL,
            &message,
            &mut context,
            &code,
            &analysis,
            &mut stack_buffer,
            &mut memory,
            &hash_cache,
        )
        .unwrap();
        let mut stack = Stack::empty(interpreter.stack.as_mut().unwrap());
        stack.reset_to(&[1u8.into(), 2u8.into(), 3u8.into()]);
        interpreter.parked.state = GasAndSize::new(interpreter.parked.state.gas(), stack);
        let result: StepResult = interpreter.run(&mut NoOpObserver());
        assert_eq!(result.step_status_code, StepStatusCode::EVMC_STEP_STOPPED);
        assert_eq!(result.stack.as_slice(), [u256::from(6u8).into()]);
        assert_eq!(
            result.gas_left,
            MockExecutionMessage::DEFAULT_INIT_GAS.cast_signed() - 2 * 3
        );
    }

    #[cfg(not(debug_assertions))]
    #[test]
    // When feature tail-call is enabled, but the tail calls are not eliminated the stack will
    // overflow if enough operations are executed. This test makes sure that does not happen.
    // Because it will fail when compiled without optimizations, it is only enabled when
    // debug_assertions are not enabled (the default in release mode).
    fn tail_call_elimination() {
        let code_analysis_cache = CodeAnalysisCache::default();
        let hash_cache = HashCache::default();
        let mut context = MockExecutionContextTrait::new();
        let message = MockExecutionMessage::default().into();
        let code = &[Opcode::JumpDest as u8; 10_000_000];
        let analysis = CodeAnalysis::new(code, None, &code_analysis_cache);
        let (mut stack_buffer, mut memory) = (Stack::new_buffer(), Memory::new());
        let interpreter = Interpreter::new(
            Revision::EVMC_ISTANBUL,
            &message,
            &mut context,
            code,
            &analysis,
            &mut stack_buffer,
            &mut memory,
            &hash_cache,
        )
        .unwrap();
        let result: StepResult = interpreter.run(&mut NoOpObserver());
        assert_eq!(result.step_status_code, StepStatusCode::EVMC_STEP_STOPPED);
    }

    #[test]
    fn add_not_enough_gas() {
        let code_analysis_cache = CodeAnalysisCache::default();
        let hash_cache = HashCache::default();
        let mut context = MockExecutionContextTrait::new();
        let message = MockExecutionMessage {
            gas: 2,
            ..Default::default()
        };
        let message = message.into();
        let code = [Opcode::Add as u8];
        let analysis = CodeAnalysis::new(&code, None, &code_analysis_cache);
        let (mut stack_buffer, mut memory) = (Stack::new_buffer(), Memory::new());
        let interpreter = Interpreter::new(
            Revision::EVMC_ISTANBUL,
            &message,
            &mut context,
            &code,
            &analysis,
            &mut stack_buffer,
            &mut memory,
            &hash_cache,
        )
        .unwrap();
        let result: ExecutionResult = interpreter.run(&mut NoOpObserver());
        assert_eq!(result.status_code(), StatusCode::EVMC_OUT_OF_GAS);
    }

    #[test]
    fn call() {
        let code_analysis_cache = CodeAnalysisCache::default();
        let hash_cache = HashCache::default();
        // helpers to generate unique values; random values are not needed
        let mut unique_values = 1u8..;
        let mut next_value = || unique_values.next().unwrap();

        let memory = [next_value(), next_value(), next_value(), next_value()];
        let ret_data = [next_value(), next_value()];

        let gas = next_value() as u64;
        let addr = next_value().into();
        let value = u256::ZERO;
        let args_offset = 1usize;
        let args_len = memory.len() - args_offset - 1;
        let ret_offset = 1usize;
        let ret_len = ret_data.len();

        let input = memory[args_offset..args_offset + args_len].to_vec();

        let message = MockExecutionMessage {
            recipient: u256::from(next_value()).into(),
            ..Default::default()
        };

        let mut context = MockExecutionContextTrait::new();
        context
            .expect_get_balance()
            .times(1)
            .with(predicate::eq(Address::from(message.recipient)))
            .return_const(Uint256::from(u256::ZERO));
        context
            .expect_call()
            .times(1)
            .withf(move |call_message| {
                call_message.kind() == MessageKind::EVMC_CALL
                    && call_message.flags() == 0
                    && call_message.depth() == message.depth + 1
                    && call_message.gas() == gas.cast_signed()
                    && *call_message.sender() == message.recipient
                    && *call_message.recipient() == Address::from(addr)
                    && call_message.input() == input
                    && *call_message.value() == Uint256::from(value)
                    && *call_message.create2_salt() == Uint256::from(u256::ZERO)
                    && *call_message.code_address() == Address::from(addr)
                    && call_message.code().is_empty()
            })
            .returning(move |_| {
                ExecutionResult::new(
                    StatusCode::EVMC_SUCCESS,
                    0,
                    0,
                    Box::from(ret_data.as_slice()),
                    Address::default(),
                )
            });

        let message = message.into();

        let stack = [
            ret_len.into(),
            ret_offset.into(),
            args_len.into(),
            args_offset.into(),
            value,
            addr,
            gas.into(),
        ];

        let code = [Opcode::Call as u8];
        let analysis = CodeAnalysis::new(&code, None, &code_analysis_cache);
        let (mut stack_buffer, mut run_memory) = (Stack::new_buffer(), Memory::new());
        let interpreter = Interpreter::new_steppable(
            Revision::EVMC_ISTANBUL,
            &message,
            &mut context,
            &code,
            0,
            0,
            &stack,
            &memory,
            &[],
            None,
            &analysis,
            &mut stack_buffer,
            &mut run_memory,
            &hash_cache,
        )
        .unwrap();
        let result: StepResult = interpreter.run(&mut NoOpObserver());
        assert_eq!(result.step_status_code, StepStatusCode::EVMC_STEP_STOPPED);
        assert_eq!(result.pc, 1);
        assert_eq!(
            result.gas_left,
            MockExecutionMessage::DEFAULT_INIT_GAS.cast_signed() - 700 - gas.cast_signed()
        );
        assert_eq!(result.last_call_return_data.as_ref(), ret_data.as_slice());
        assert_eq!(
            &result.memory[ret_offset..ret_offset + ret_len],
            ret_data.as_slice()
        );
    }
}
