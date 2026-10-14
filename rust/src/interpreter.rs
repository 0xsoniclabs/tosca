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
        Code, CodeAnalysis, ExecStatus, ExecutionContextTrait, FailStatus, LastCallReturnData,
        Memory, Observer, Pc, Stack, StackBuffer, hash_cache::HashCache, u256,
    },
    utils::{Gas, GasRefund, SliceExt, check_min_revision, check_not_read_only, word_size},
};

type OpResult = Result<(), FailStatus>;

pub type OpFn<const STEPPABLE: bool> = fn(&mut Interpreter<STEPPABLE>) -> OpResult;

/// Tail calls `$call`, which requires caller and callee to have matching signatures. Without the
/// tail-call feature this is a plain `return` so that the crate builds on stable Rust: `become` is
/// gated even where it is removed by `cfg`, so it must not appear in any expansion.
#[cfg(feature = "tail-call")]
macro_rules! tail_call {
    ($call:expr) => {
        become $call
    };
}
#[cfg(not(feature = "tail-call"))]
macro_rules! tail_call {
    ($call:expr) => {
        return $call
    };
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
        Interpreter::create,
        Interpreter::call,
        Interpreter::call_code,
        Interpreter::return_,
        Interpreter::delegate_call,
        Interpreter::create2,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::jumptable_placeholder,
        Interpreter::static_call,
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

pub struct Interpreter<'a, const STEPPABLE: bool> {
    pub exec_status: ExecStatus,
    pub message: &'a ExecutionMessage<'a>,
    pub context: &'a mut dyn ExecutionContextTrait,
    pub revision: Revision,
    pub code: Code<'a, STEPPABLE>,
    pub pc: Pc<'a, STEPPABLE>,
    pub gas_left: Gas,
    pub gas_refund: GasRefund,
    pub output: Box<[u8]>,
    pub stack: Stack<'a>,
    pub memory: &'a mut Memory,
    pub last_call_return_data: LastCallReturnData<'a>,
    pub steps: Option<i32>,
    pub hash_cache: &'a HashCache,
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
            pc: Pc::new(code_analysis, 0),
            gas_left: Gas::new(message.gas)?,
            gas_refund: GasRefund::new(0),
            output: Box::default(),
            stack: Stack::empty(stack_buffer),
            memory,
            last_call_return_data: LastCallReturnData::Slice(&[]),
            steps: None,
            hash_cache,
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
        memory.reset_to(init_memory);
        Ok(Self {
            exec_status: ExecStatus::Running,
            message,
            context,
            revision,
            code: Code::new(code, code_analysis),
            pc: Pc::new(code_analysis, pc),
            gas_left: Gas::new(message.gas)?,
            gas_refund: GasRefund::new(gas_refund),
            output: Box::default(),
            stack,
            memory,
            last_call_return_data: LastCallReturnData::Slice(last_call_return_data),
            steps,
            hash_cache,
        })
    }
}

impl<const STEPPABLE: bool> Interpreter<'_, STEPPABLE> {
    /// R is expected to be [ExecutionResult] or [StepResult].
    #[cfg(not(feature = "tail-call"))]
    pub fn run<O, R>(mut self, observer: &mut O) -> R
    where
        O: Observer<STEPPABLE>,
        R: From<Self> + From<FailStatus>,
    {
        while self.exec_status == ExecStatus::Running {
            if STEPPABLE {
                match &mut self.steps {
                    None => (),
                    Some(0) => break,
                    Some(steps) => *steps -= 1,
                }
            }
            #[cfg(feature = "fn-ptr-conversion-dispatch")]
            let op = self.pc.get();
            #[cfg(not(feature = "fn-ptr-conversion-dispatch"))]
            let op = match self.code.get(self.pc) {
                Ok(op) => op,
                Err(GetOpcodeError::OutOfRange) => {
                    std::hint::cold_path();
                    self.exec_status = ExecStatus::Stopped;
                    break;
                }
                Err(GetOpcodeError::Invalid) => {
                    std::hint::cold_path();
                    return FailStatus::InvalidInstruction.into();
                }
            };
            observer.pre_op(&self);
            let res = std::cfg_select! {
                feature = "fn-ptr-conversion-dispatch" => op(&mut self),
                _ => get_jumptable()[op as usize](&mut self),
            };
            if let Err(err) = res {
                std::hint::cold_path();
                return err.into();
            }
            observer.post_op(&self);
        }

        self.into()
    }
    /// R is expected to be [ExecutionResult] or [StepResult].
    #[cfg(feature = "tail-call")]
    #[inline(always)]
    pub fn run<O, R>(mut self, observer: &mut O) -> R
    where
        O: Observer<STEPPABLE>,
        R: From<Self> + From<FailStatus>,
    {
        observer.log("feature \"tail-call\" does not support logging".into());
        if let Err(err) = self.next() {
            std::hint::cold_path();
            return err.into();
        }
        self.into()
    }
    #[cfg(feature = "tail-call")]
    #[inline(always)]
    pub fn next(&mut self) -> OpResult {
        if STEPPABLE {
            match &mut self.steps {
                None => (),
                Some(0) => return Ok(()),
                Some(steps) => *steps -= 1,
            }
        }
        std::cfg_select! {
            feature = "fn-ptr-conversion-dispatch" => {
                let op = self.pc.get();
                tail_call!(op(self))
            }
            _ => match self.code.get(self.pc) {
                Ok(op) => tail_call!(get_jumptable()[op as usize](self)),
                Err(GetOpcodeError::OutOfRange) => {
                    std::hint::cold_path();
                    self.exec_status = ExecStatus::Stopped;
                    Ok(())
                }
                Err(GetOpcodeError::Invalid) => {
                    std::hint::cold_path();
                    Err(FailStatus::InvalidInstruction)
                }
            },
        }
    }

    /// Returns with [`Ok`] or calls the next operation as a tail call if feature `tail-call` is
    /// enabled.
    #[allow(clippy::unused_self)]
    #[inline(always)]
    fn return_from_op(&mut self) -> OpResult {
        std::cfg_select! {
            feature = "tail-call" => tail_call!(self.next()),
            _ => Ok(()),
        }
    }

    #[cold]
    pub fn jumptable_placeholder(_interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        Err(FailStatus::Failure)
    }

    /// Performs an operation that charges `cost` gas, pops one value and pushes `op` applied to it.
    #[inline(always)]
    fn unary_op<I: Into<u256>>(&mut self, op: fn(u256) -> I, cost: u64) -> OpResult {
        self.gas_left.consume(cost)?;
        let (push_location, [value]) = self.stack.pop_with_location()?;
        push_location.push(op(value));
        self.pc.next();
        Ok(())
    }

    /// Performs an operation that charges `cost` gas, pops two values and pushes `op` applied to
    /// them. `op` receives the top of stack as its first argument.
    #[inline(always)]
    fn binary_op<I: Into<u256>>(&mut self, op: fn(u256, u256) -> I, cost: u64) -> OpResult {
        self.gas_left.consume(cost)?;
        let (push_location, [value2, value1]) = self.stack.pop_with_location()?;
        push_location.push(op(value1, value2));
        self.pc.next();
        Ok(())
    }

    /// Performs an operation that charges `cost` gas and pushes `op` applied to the interpreter
    /// state.
    #[inline(always)]
    fn push_value_op<I: Into<u256>>(&mut self, op: fn(&mut Self) -> I, cost: u64) -> OpResult {
        self.gas_left.consume(cost)?;
        let value = op(self);
        self.stack.push(value)?;
        self.pc.next();
        Ok(())
    }

    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    pub fn no_op(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.pc.next();
        tail_call!(interpreter.return_from_op())
    }

    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    pub fn skip_no_ops(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.pc.jump_to_jumpdest();
        tail_call!(interpreter.return_from_op())
    }

    fn stop(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.exec_status = ExecStatus::Stopped;
        Ok(())
    }

    fn add(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.binary_op(u256::add, 3)?;
        tail_call!(interpreter.return_from_op())
    }

    fn mul(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.binary_op(u256::mul, 5)?;
        tail_call!(interpreter.return_from_op())
    }

    fn sub(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.binary_op(u256::sub, 3)?;
        tail_call!(interpreter.return_from_op())
    }

    fn div(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.binary_op(u256::div, 5)?;
        tail_call!(interpreter.return_from_op())
    }

    fn s_div(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.binary_op(u256::sdiv, 5)?;
        tail_call!(interpreter.return_from_op())
    }

    fn mod_(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.binary_op(u256::rem, 5)?;
        tail_call!(interpreter.return_from_op())
    }

    fn s_mod(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.binary_op(u256::srem, 5)?;
        tail_call!(interpreter.return_from_op())
    }

    fn add_mod(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.gas_left.consume(8)?;
        let (push_location, [denominator, value2, value1]) =
            interpreter.stack.pop_with_location()?;
        push_location.push(u256::addmod(value1, value2, denominator));
        interpreter.pc.next();
        tail_call!(interpreter.return_from_op())
    }

    fn mul_mod(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.gas_left.consume(8)?;
        let (push_location, [denominator, fac2, fac1]) = interpreter.stack.pop_with_location()?;
        push_location.push(u256::mulmod(fac1, fac2, denominator));
        interpreter.pc.next();
        tail_call!(interpreter.return_from_op())
    }

    fn exp(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.gas_left.consume(10)?;
        let (push_location, [exp, value]) = interpreter.stack.pop_with_location()?;
        interpreter
            .gas_left
            .consume(exp.bits().div_ceil(8) as u64 * 50)?; // * does not overflow
        push_location.push(value.pow(exp));
        interpreter.pc.next();
        tail_call!(interpreter.return_from_op())
    }

    fn sign_extend(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.binary_op(u256::signextend, 5)?;
        tail_call!(interpreter.return_from_op())
    }

    fn lt(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.binary_op(|lhs, rhs| lhs < rhs, 3)?;
        tail_call!(interpreter.return_from_op())
    }

    fn gt(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.binary_op(|lhs, rhs| lhs > rhs, 3)?;
        tail_call!(interpreter.return_from_op())
    }

    fn s_lt(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.binary_op(|lhs, rhs| lhs.slt(&rhs), 3)?;
        tail_call!(interpreter.return_from_op())
    }

    fn s_gt(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.binary_op(|lhs, rhs| lhs.sgt(&rhs), 3)?;
        tail_call!(interpreter.return_from_op())
    }

    fn eq(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.binary_op(|lhs, rhs| lhs == rhs, 3)?;
        tail_call!(interpreter.return_from_op())
    }

    fn is_zero(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.unary_op(|value| value == u256::ZERO, 3)?;
        tail_call!(interpreter.return_from_op())
    }

    fn and(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.binary_op(u256::bitand, 3)?;
        tail_call!(interpreter.return_from_op())
    }

    fn or(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.binary_op(u256::bitor, 3)?;
        tail_call!(interpreter.return_from_op())
    }

    fn xor(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.binary_op(u256::bitxor, 3)?;
        tail_call!(interpreter.return_from_op())
    }

    fn not(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.unary_op(u256::not, 3)?;
        tail_call!(interpreter.return_from_op())
    }

    fn byte(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.binary_op(|offset, value| value.byte(offset), 3)?;
        tail_call!(interpreter.return_from_op())
    }

    fn shl(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.binary_op(|shift, value| value << shift, 3)?;
        tail_call!(interpreter.return_from_op())
    }

    fn shr(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.binary_op(|shift, value| value >> shift, 3)?;
        tail_call!(interpreter.return_from_op())
    }

    fn sar(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.binary_op(|shift, value| value.sar(shift), 3)?;
        tail_call!(interpreter.return_from_op())
    }

    fn clz(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        check_min_revision(Revision::EVMC_OSAKA, interpreter.revision)?;
        interpreter.unary_op(|value| value.leading_zeros(), 5)?;
        tail_call!(interpreter.return_from_op())
    }

    fn sha3(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.gas_left.consume(30)?;
        let (push_location, [len, offset]) = interpreter.stack.pop_with_location()?;

        let len = u64::try_from(len).map_err(|_| FailStatus::OutOfGas)?;
        interpreter.gas_left.consume(6 * word_size(len)?)?; // * does not overflow

        let data = interpreter
            .memory
            .get_mut_slice(offset, len, &mut interpreter.gas_left)?;
        push_location.push(interpreter.hash_cache.hash(data));
        interpreter.pc.next();
        tail_call!(interpreter.return_from_op())
    }

    fn address(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.push_value_op(|i| i.message.recipient, 2)?;
        tail_call!(interpreter.return_from_op())
    }

    fn balance(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        if interpreter.revision < Revision::EVMC_BERLIN {
            interpreter.gas_left.consume(700)?;
        }
        let (push_location, [addr]) = interpreter.stack.pop_with_location()?;
        let addr = addr.into();
        interpreter.gas_left.consume_address_access_cost(
            &addr,
            interpreter.revision,
            interpreter.context,
        )?;
        push_location.push(interpreter.context.get_balance(&addr));
        interpreter.pc.next();
        tail_call!(interpreter.return_from_op())
    }

    fn origin(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.push_value_op(|i| i.context.get_tx_context().tx_origin, 2)?;
        tail_call!(interpreter.return_from_op())
    }

    fn caller(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.push_value_op(|i| i.message.sender, 2)?;
        tail_call!(interpreter.return_from_op())
    }

    fn call_value(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.push_value_op(|i| i.message.value, 2)?;
        tail_call!(interpreter.return_from_op())
    }

    fn call_data_load(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.gas_left.consume(3)?;
        let (push_location, [offset]) = interpreter.stack.pop_with_location()?;
        let (offset, overflow) = offset.into_u64_with_overflow();
        let offset = offset as usize;
        let call_data = interpreter.message.input;
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
        interpreter.pc.next();
        tail_call!(interpreter.return_from_op())
    }

    fn call_data_size(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.push_value_op(|i| i.message.input.len(), 2)?;
        tail_call!(interpreter.return_from_op())
    }

    fn push0(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        check_min_revision(Revision::EVMC_SHANGHAI, interpreter.revision)?;
        interpreter.push_value_op(|_| u256::ZERO, 2)?;
        tail_call!(interpreter.return_from_op())
    }

    fn call_data_copy(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.gas_left.consume(3)?;
        let [len, offset, dest_offset] = interpreter.stack.pop()?;

        if len != u256::ZERO {
            let len = u64::try_from(len).map_err(|_| FailStatus::InvalidMemoryAccess)?;

            let src = interpreter.message.input.get_within_bounds(offset, len);
            let dest =
                interpreter
                    .memory
                    .get_mut_slice(dest_offset, len, &mut interpreter.gas_left)?;
            dest.copy_padded(src, &mut interpreter.gas_left)?;
        }
        interpreter.pc.next();
        tail_call!(interpreter.return_from_op())
    }

    fn code_size(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.push_value_op(|i| i.code.len(), 2)?;
        tail_call!(interpreter.return_from_op())
    }

    fn code_copy(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.gas_left.consume(3)?;
        let [len, offset, dest_offset] = interpreter.stack.pop()?;

        if len != u256::ZERO {
            let len = u64::try_from(len).map_err(|_| FailStatus::OutOfGas)?;

            let src = interpreter.code.get_within_bounds(offset, len);
            let dest =
                interpreter
                    .memory
                    .get_mut_slice(dest_offset, len, &mut interpreter.gas_left)?;
            dest.copy_padded(src, &mut interpreter.gas_left)?;
        }
        interpreter.pc.next();
        tail_call!(interpreter.return_from_op())
    }

    fn gas_price(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.push_value_op(|i| i.context.get_tx_context().tx_gas_price, 2)?;
        tail_call!(interpreter.return_from_op())
    }

    fn ext_code_size(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        if interpreter.revision < Revision::EVMC_BERLIN {
            interpreter.gas_left.consume(700)?;
        }
        let (push_location, [addr]) = interpreter.stack.pop_with_location()?;
        let addr = addr.into();
        interpreter.gas_left.consume_address_access_cost(
            &addr,
            interpreter.revision,
            interpreter.context,
        )?;
        push_location.push(interpreter.context.get_code_size(&addr));
        interpreter.pc.next();
        tail_call!(interpreter.return_from_op())
    }

    fn ext_code_copy(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        if interpreter.revision < Revision::EVMC_BERLIN {
            interpreter.gas_left.consume(700)?;
        }
        let [len, offset, dest_offset, addr] = interpreter.stack.pop()?;
        let addr = addr.into();

        interpreter.gas_left.consume_address_access_cost(
            &addr,
            interpreter.revision,
            interpreter.context,
        )?;
        if len != u256::ZERO {
            let len = u64::try_from(len).map_err(|_| FailStatus::OutOfGas)?;

            let dest =
                interpreter
                    .memory
                    .get_mut_slice(dest_offset, len, &mut interpreter.gas_left)?;
            let (offset, offset_overflow) = offset.into_u64_with_overflow();
            interpreter.gas_left.consume_copy_cost(len)?;
            let bytes_written = interpreter.context.copy_code(&addr, offset as usize, dest);
            if offset_overflow {
                dest.fill(0);
            } else if (bytes_written as u64) < len {
                dest[bytes_written..].fill(0);
            }
        }
        interpreter.pc.next();
        tail_call!(interpreter.return_from_op())
    }

    fn return_data_size(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.push_value_op(|i| i.last_call_return_data.len(), 2)?;
        tail_call!(interpreter.return_from_op())
    }

    fn return_data_copy(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.gas_left.consume(3)?;
        let [len, offset, dest_offset] = interpreter.stack.pop()?;

        let src = &interpreter.last_call_return_data;
        let (offset, offset_overflow) = offset.into_u64_with_overflow();
        let (len, len_overflow) = len.into_u64_with_overflow();
        let (end, end_overflow) = offset.overflowing_add(len);
        if offset_overflow || len_overflow || end_overflow || end > src.len() as u64 {
            std::hint::cold_path();
            return Err(FailStatus::InvalidMemoryAccess);
        }

        if len != 0 {
            let src = &src[offset as usize..end as usize];
            let dest =
                interpreter
                    .memory
                    .get_mut_slice(dest_offset, len, &mut interpreter.gas_left)?;
            dest.copy_padded(src, &mut interpreter.gas_left)?;
        }
        interpreter.pc.next();
        tail_call!(interpreter.return_from_op())
    }

    fn ext_code_hash(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        if interpreter.revision < Revision::EVMC_BERLIN {
            interpreter.gas_left.consume(700)?;
        }
        let (push_location, [addr]) = interpreter.stack.pop_with_location()?;
        let addr = addr.into();
        interpreter.gas_left.consume_address_access_cost(
            &addr,
            interpreter.revision,
            interpreter.context,
        )?;
        push_location.push(interpreter.context.get_code_hash(&addr));
        interpreter.pc.next();
        tail_call!(interpreter.return_from_op())
    }

    fn block_hash(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.gas_left.consume(20)?;
        let (push_location, [block_number]) = interpreter.stack.pop_with_location()?;
        push_location.push(
            u64::try_from(block_number)
                .map(|idx| interpreter.context.get_block_hash(idx.cast_signed()).into())
                .unwrap_or(u256::ZERO),
        );
        interpreter.pc.next();
        tail_call!(interpreter.return_from_op())
    }

    fn coinbase(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.push_value_op(|i| i.context.get_tx_context().block_coinbase, 2)?;
        tail_call!(interpreter.return_from_op())
    }

    fn timestamp(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.push_value_op(
            |i| i.context.get_tx_context().block_timestamp.cast_unsigned(),
            2,
        )?;
        tail_call!(interpreter.return_from_op())
    }

    fn number(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.push_value_op(
            |i| i.context.get_tx_context().block_number.cast_unsigned(),
            2,
        )?;
        tail_call!(interpreter.return_from_op())
    }

    fn prev_randao(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.push_value_op(|i| i.context.get_tx_context().block_prev_randao, 2)?;
        tail_call!(interpreter.return_from_op())
    }

    fn gas_limit(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.push_value_op(
            |i| i.context.get_tx_context().block_gas_limit.cast_unsigned(),
            2,
        )?;
        tail_call!(interpreter.return_from_op())
    }

    fn chain_id(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.push_value_op(|i| i.context.get_tx_context().chain_id, 2)?;
        tail_call!(interpreter.return_from_op())
    }

    fn self_balance(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        check_min_revision(Revision::EVMC_ISTANBUL, interpreter.revision)?;
        interpreter.gas_left.consume(5)?;
        interpreter.stack.check_overflow(1)?; // Check for stack overflow before querying the host
        let addr = interpreter.message.recipient;
        interpreter
            .stack
            .push(interpreter.context.get_balance(&addr))?;
        interpreter.pc.next();
        tail_call!(interpreter.return_from_op())
    }

    fn base_fee(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        check_min_revision(Revision::EVMC_LONDON, interpreter.revision)?;
        interpreter.push_value_op(|i| i.context.get_tx_context().block_base_fee, 2)?;
        tail_call!(interpreter.return_from_op())
    }

    fn blob_hash(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        check_min_revision(Revision::EVMC_CANCUN, interpreter.revision)?;
        interpreter.gas_left.consume(3)?;
        let (push_location, [idx]) = interpreter.stack.pop_with_location()?;
        let hashes = interpreter.context.get_tx_context().blob_hashes;
        if let Ok(idx) = u64::try_from(idx)
            && let Some(hash) = hashes.get(idx as usize)
        {
            push_location.push(*hash);
        } else {
            push_location.push(u256::ZERO);
        }
        interpreter.pc.next();
        tail_call!(interpreter.return_from_op())
    }

    fn blob_base_fee(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        check_min_revision(Revision::EVMC_CANCUN, interpreter.revision)?;
        interpreter.push_value_op(|i| i.context.get_tx_context().blob_base_fee, 2)?;
        tail_call!(interpreter.return_from_op())
    }

    fn pop(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.gas_left.consume(2)?;
        let [_] = interpreter.stack.pop()?;
        interpreter.pc.next();
        tail_call!(interpreter.return_from_op())
    }

    fn m_load(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.gas_left.consume(3)?;
        let (push_location, [offset]) = interpreter.stack.pop_with_location()?;

        push_location.push(
            interpreter
                .memory
                .get_word(offset, &mut interpreter.gas_left)?,
        );
        interpreter.pc.next();
        tail_call!(interpreter.return_from_op())
    }

    fn m_store(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.gas_left.consume(3)?;
        let [value, offset] = interpreter.stack.pop()?;

        *interpreter
            .memory
            .get_mut_array(offset, &mut interpreter.gas_left)? = value.to_be_bytes();
        interpreter.pc.next();
        tail_call!(interpreter.return_from_op())
    }

    fn m_store8(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.gas_left.consume(3)?;
        let [value, offset] = interpreter.stack.pop()?;

        let dest = interpreter
            .memory
            .get_mut_byte(offset, &mut interpreter.gas_left)?;
        *dest = value.least_significant_byte();
        interpreter.pc.next();
        tail_call!(interpreter.return_from_op())
    }

    fn s_load(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        if interpreter.revision < Revision::EVMC_BERLIN {
            interpreter.gas_left.consume(800)?;
        }
        let (push_location, [key]) = interpreter.stack.pop_with_location()?;
        let key = key.into();
        let addr = &interpreter.message.recipient;
        if interpreter.revision >= Revision::EVMC_BERLIN {
            if interpreter.context.access_storage(addr, &key) == AccessStatus::EVMC_ACCESS_COLD {
                interpreter.gas_left.consume(2_100)?;
            } else {
                interpreter.gas_left.consume(100)?;
            }
        }
        let value = interpreter.context.get_storage(addr, &key);
        push_location.push(value);
        interpreter.pc.next();
        tail_call!(interpreter.return_from_op())
    }

    fn jump(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter
            .gas_left
            .consume(if STEPPABLE { 8 } else { 8 + 1 })?;
        let [dest] = interpreter.stack.pop()?;
        interpreter.code.try_jump(&mut interpreter.pc, dest)?;
        if !STEPPABLE {
            interpreter.pc.next();
        }
        tail_call!(interpreter.return_from_op())
    }

    fn jump_i(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.gas_left.consume(10)?;
        let [cond, dest] = interpreter.stack.pop()?;
        if cond == u256::ZERO {
            interpreter.pc.next();
        } else {
            interpreter.code.try_jump(&mut interpreter.pc, dest)?;
            if !STEPPABLE {
                interpreter.gas_left.consume(1)?;
                interpreter.pc.next();
            }
        }
        tail_call!(interpreter.return_from_op())
    }

    fn pc(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.push_value_op(|i| i.pc.code_offset(), 2)?;
        tail_call!(interpreter.return_from_op())
    }

    fn m_size(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.push_value_op(|i| i.memory.len(), 2)?;
        tail_call!(interpreter.return_from_op())
    }

    fn gas(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.push_value_op(|i| i.gas_left.as_u64(), 2)?;
        tail_call!(interpreter.return_from_op())
    }

    fn jump_dest(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.gas_left.consume(1)?;
        interpreter.pc.next();
        tail_call!(interpreter.return_from_op())
    }

    fn t_load(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        check_min_revision(Revision::EVMC_CANCUN, interpreter.revision)?;
        interpreter.gas_left.consume(100)?;
        let (push_location, [key]) = interpreter.stack.pop_with_location()?;
        let value = interpreter
            .context
            .get_transient_storage(&interpreter.message.recipient, &key.into());
        push_location.push(value);
        interpreter.pc.next();
        tail_call!(interpreter.return_from_op())
    }

    fn t_store(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        check_min_revision(Revision::EVMC_CANCUN, interpreter.revision)?;
        check_not_read_only(interpreter.message)?;
        interpreter.gas_left.consume(100)?;
        let [value, key] = interpreter.stack.pop()?;
        interpreter.context.set_transient_storage(
            &interpreter.message.recipient,
            &key.into(),
            &value.into(),
        );
        interpreter.pc.next();
        tail_call!(interpreter.return_from_op())
    }

    fn m_copy(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        check_min_revision(Revision::EVMC_CANCUN, interpreter.revision)?;
        interpreter.gas_left.consume(3)?;
        let [len, offset, dest_offset] = interpreter.stack.pop()?;
        if len != u256::ZERO {
            interpreter
                .memory
                .copy_within(offset, dest_offset, len, &mut interpreter.gas_left)?;
        }
        interpreter.pc.next();
        tail_call!(interpreter.return_from_op())
    }

    fn return_(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        let [len, offset] = interpreter.stack.pop()?;
        let len = u64::try_from(len).map_err(|_| FailStatus::OutOfGas)?;
        let data = interpreter
            .memory
            .get_mut_slice(offset, len, &mut interpreter.gas_left)?;
        interpreter.output = Box::from(&*data);
        interpreter.exec_status = ExecStatus::Returned;
        Ok(())
    }

    fn revert(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        let [len, offset] = interpreter.stack.pop()?;
        let len = u64::try_from(len).map_err(|_| FailStatus::OutOfGas)?;
        let data = interpreter
            .memory
            .get_mut_slice(offset, len, &mut interpreter.gas_left)?;
        interpreter.output = Box::from(&*data);
        interpreter.exec_status = ExecStatus::Revert;
        Ok(())
    }

    #[cold]
    fn invalid(_interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        Err(FailStatus::InvalidInstruction)
    }

    fn self_destruct(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        check_not_read_only(interpreter.message)?;
        interpreter.gas_left.consume(5_000)?;
        let [addr] = interpreter.stack.pop()?;
        let addr = addr.into();

        if interpreter.revision >= Revision::EVMC_BERLIN
            && interpreter.context.access_account(&addr) == AccessStatus::EVMC_ACCESS_COLD
        {
            interpreter.gas_left.consume(2_600)?;
        }

        if u256::from(
            interpreter
                .context
                .get_balance(&interpreter.message.recipient),
        ) > u256::ZERO
            && !interpreter.context.account_exists(&addr)
        {
            interpreter.gas_left.consume(25_000)?;
        }

        let destructed = interpreter
            .context
            .selfdestruct(&interpreter.message.recipient, &addr);
        if interpreter.revision <= Revision::EVMC_BERLIN && destructed {
            interpreter.gas_refund.add(24_000);
        }

        interpreter.exec_status = ExecStatus::Stopped;
        Ok(())
    }

    fn sstore(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        check_not_read_only(interpreter.message)?;

        if interpreter.revision >= Revision::EVMC_ISTANBUL && interpreter.gas_left <= 2_300 {
            std::hint::cold_path();
            return Err(FailStatus::OutOfGas);
        }
        let [value, key] = interpreter.stack.pop()?;
        let key = key.into();
        let addr = &interpreter.message.recipient;

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
        interpreter.gas_left.consume(dyn_gas)?;
        interpreter.gas_refund.add(gas_refund_change);
        interpreter.pc.next();
        tail_call!(interpreter.return_from_op())
    }

    fn push<const N: usize>(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.gas_left.consume(3)?;
        std::cfg_select! {
            feature = "fn-ptr-conversion-dispatch" => {
                interpreter.stack.push(interpreter.pc.get_push_data())?;
            }
            _ => {
                interpreter.pc.next();
                interpreter.stack.push(interpreter.code.get_push_data::<N>(&mut interpreter.pc))?;
            }
        }
        tail_call!(interpreter.return_from_op())
    }

    fn dup<const N: usize>(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.gas_left.consume(3)?;
        interpreter.stack.dup::<N>()?;
        interpreter.pc.next();
        tail_call!(interpreter.return_from_op())
    }

    fn swap<const N: usize>(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        interpreter.gas_left.consume(3)?;
        interpreter.stack.swap_with_top::<N>()?;
        interpreter.pc.next();
        tail_call!(interpreter.return_from_op())
    }

    fn log<const N: usize>(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        check_not_read_only(interpreter.message)?;
        interpreter.gas_left.consume(375)?;
        let [len, offset] = interpreter.stack.pop()?;
        let topics: [u256; N] = interpreter.stack.pop()?;
        let (len, len_overflow) = len.into_u64_with_overflow();
        let (len8, len8_overflow) = len.overflowing_mul(8);
        let (cost, cost_overflow) = (375 * N as u64).overflowing_add(len8);
        if len_overflow || len8_overflow || cost_overflow {
            std::hint::cold_path();
            return Err(FailStatus::OutOfGas);
        }
        interpreter.gas_left.consume(cost)?;

        let data = interpreter
            .memory
            .get_mut_slice(offset, len, &mut interpreter.gas_left)?;
        let mut topics_uint256 = topics.map(Uint256::from);
        topics_uint256.reverse();
        interpreter
            .context
            .emit_log(&interpreter.message.recipient, data, &topics_uint256);
        interpreter.pc.next();
        tail_call!(interpreter.return_from_op())
    }

    fn create(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        tail_call!(Self::create_or_create2::<false>(interpreter))
    }

    fn create2(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        tail_call!(Self::create_or_create2::<true>(interpreter))
    }

    fn create_or_create2<const CREATE2: bool>(
        interpreter: &mut Interpreter<'_, STEPPABLE>,
    ) -> OpResult {
        interpreter.gas_left.consume(32_000)?;
        check_not_read_only(interpreter.message)?;
        let [len, offset, value] = interpreter.stack.pop()?;
        let salt = if CREATE2 {
            let [salt] = interpreter.stack.pop()?;
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
                return Err(FailStatus::OutOfGas);
            }
            let init_code_cost = 2 * init_code_word_size; // does not overflow
            interpreter.gas_left.consume(init_code_cost)?;
        }
        if CREATE2 {
            let hash_cost = 6 * init_code_word_size; // does not overflow
            interpreter.gas_left.consume(hash_cost)?;
        }

        let init_code = interpreter
            .memory
            .get_mut_slice(offset, len, &mut interpreter.gas_left)?;

        if value
            > interpreter
                .context
                .get_balance(&interpreter.message.recipient)
                .into()
        {
            interpreter.last_call_return_data = LastCallReturnData::Slice(&[]);
            interpreter.stack.push(u256::ZERO)?;
            interpreter.pc.next();
            tail_call!(interpreter.return_from_op())
        }

        let gas_left = interpreter.gas_left.as_u64();
        let gas_limit = gas_left - gas_left / 64;
        interpreter.gas_left.consume(gas_limit)?;

        let message = ExecutionMessage {
            kind: if CREATE2 {
                MessageKind::EVMC_CREATE2
            } else {
                MessageKind::EVMC_CREATE
            },
            flags: interpreter.message.flags,
            depth: interpreter.message.depth + 1,
            gas: gas_limit.cast_signed(),
            recipient: u256::ZERO.into(), // ignored
            sender: interpreter.message.recipient,
            input: init_code,
            value: value.into(),
            create2_salt: salt.into(),
            code_address: u256::ZERO.into(), // ignored
            code: &[],
            code_hash: None,
        };
        let result = interpreter.context.call(&message);

        interpreter.gas_left.add(result.gas_left())?;
        interpreter.gas_refund.add(result.gas_refund());

        if result.status_code() == StatusCode::EVMC_SUCCESS {
            interpreter.last_call_return_data = LastCallReturnData::Slice(&[]);
            interpreter.stack.push(result.create_address())?;
        } else {
            interpreter.last_call_return_data = LastCallReturnData::CallResult(result);
            interpreter.stack.push(u256::ZERO)?;
        }
        interpreter.pc.next();
        tail_call!(interpreter.return_from_op())
    }

    fn call(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        tail_call!(Self::call_or_call_code::<false>(interpreter))
    }

    fn call_code(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        tail_call!(Self::call_or_call_code::<true>(interpreter))
    }

    fn call_or_call_code<const CODE: bool>(
        interpreter: &mut Interpreter<'_, STEPPABLE>,
    ) -> OpResult {
        if interpreter.revision < Revision::EVMC_BERLIN {
            interpreter.gas_left.consume(700)?;
        }
        let [ret_len, ret_offset, args_len, args_offset, value, addr, gas] =
            interpreter.stack.pop()?;

        if !CODE && value != u256::ZERO {
            check_not_read_only(interpreter.message)?;
        }

        let addr = addr.into();
        let args_len = u64::try_from(args_len).map_err(|_| FailStatus::OutOfGas)?;
        let ret_len = u64::try_from(ret_len).map_err(|_| FailStatus::OutOfGas)?;

        interpreter.gas_left.consume_address_access_cost(
            &addr,
            interpreter.revision,
            interpreter.context,
        )?;
        interpreter.gas_left.consume_positive_value_cost(&value)?;
        if !CODE {
            interpreter.gas_left.consume_value_to_empty_account_cost(
                &value,
                &addr,
                interpreter.context,
            )?;
        }
        interpreter.gas_left.consume_delegate_resolution_cost(
            &addr,
            interpreter.revision,
            interpreter.context,
        )?;
        // access slice to consume potential memory expansion cost but drop it so that we can get
        // another mutable reference into memory for input
        let _dest =
            interpreter
                .memory
                .get_mut_slice(ret_offset, ret_len, &mut interpreter.gas_left)?;
        let input =
            interpreter
                .memory
                .get_mut_slice(args_offset, args_len, &mut interpreter.gas_left)?;

        let gas_left = interpreter.gas_left.as_u64();
        let limit = gas_left - gas_left / 64;
        let mut endowment = gas.into_u64_saturating();
        endowment = min(endowment, limit); // cap gas at all but one 64th of gas left

        let stipend: u64 = if value == u256::ZERO { 0 } else { 2_300 };
        interpreter.gas_left.add(stipend.cast_signed())?;

        if value
            > u256::from(
                interpreter
                    .context
                    .get_balance(&interpreter.message.recipient),
            )
        {
            interpreter.last_call_return_data = LastCallReturnData::Slice(&[]);
            interpreter.stack.push(u256::ZERO)?;
            interpreter.pc.next();
            tail_call!(interpreter.return_from_op())
        }

        let call_message = if CODE {
            ExecutionMessage {
                kind: MessageKind::EVMC_CALLCODE,
                flags: interpreter.message.flags,
                depth: interpreter.message.depth + 1,
                gas: (endowment + stipend).cast_signed(),
                recipient: interpreter.message.recipient,
                sender: interpreter.message.recipient,
                input,
                value: value.into(),
                create2_salt: u256::ZERO.into(), // ignored
                code_address: addr,
                code: &[],
                code_hash: None,
            }
        } else {
            ExecutionMessage {
                kind: MessageKind::EVMC_CALL,
                flags: interpreter.message.flags,
                depth: interpreter.message.depth + 1,
                gas: (endowment + stipend).cast_signed(),
                recipient: addr,
                sender: interpreter.message.recipient,
                input,
                value: value.into(),
                create2_salt: u256::ZERO.into(), // ignored
                code_address: addr,
                code: &[],
                code_hash: None,
            }
        };

        let result = interpreter.context.call(&call_message);
        let (status_code, call_gas_left, call_gas_refund) =
            (result.status_code(), result.gas_left(), result.gas_refund());
        interpreter.last_call_return_data = LastCallReturnData::CallResult(result);
        let dest =
            interpreter
                .memory
                .get_mut_slice(ret_offset, ret_len, &mut interpreter.gas_left)?;
        let output = &interpreter.last_call_return_data;
        let min_len = min(output.len(), ret_len as usize); // ret_len == dest.len()
        dest[..min_len].copy_from_slice(&output[..min_len]);

        // Deduct before crediting (like geth) so that the gas left never exceeds Gas::MAX. None can
        // fail: the endowment fits the gas left, the stipend was added above, and the callee
        // returns at most both.
        interpreter.gas_left.consume(endowment)?;
        interpreter.gas_left.consume(stipend)?;
        interpreter.gas_left.add(call_gas_left)?;
        interpreter.gas_refund.add(call_gas_refund);

        interpreter
            .stack
            .push(status_code == StatusCode::EVMC_SUCCESS)?;
        interpreter.pc.next();
        tail_call!(interpreter.return_from_op())
    }

    fn static_call(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        tail_call!(Self::static_or_delegate_call::<false>(interpreter))
    }

    fn delegate_call(interpreter: &mut Interpreter<'_, STEPPABLE>) -> OpResult {
        tail_call!(Self::static_or_delegate_call::<true>(interpreter))
    }

    fn static_or_delegate_call<const DELEGATE: bool>(
        interpreter: &mut Interpreter<'_, STEPPABLE>,
    ) -> OpResult {
        if interpreter.revision < Revision::EVMC_BERLIN {
            interpreter.gas_left.consume(700)?;
        }
        let [ret_len, ret_offset, args_len, args_offset, addr, gas] = interpreter.stack.pop()?;

        let addr = addr.into();
        let args_len = u64::try_from(args_len).map_err(|_| FailStatus::OutOfGas)?;
        let ret_len = u64::try_from(ret_len).map_err(|_| FailStatus::OutOfGas)?;

        interpreter.gas_left.consume_address_access_cost(
            &addr,
            interpreter.revision,
            interpreter.context,
        )?;
        interpreter.gas_left.consume_delegate_resolution_cost(
            &addr,
            interpreter.revision,
            interpreter.context,
        )?;
        // access slice to consume potential memory expansion cost but drop it so that we can get
        // another mutable reference into memory for input
        let _dest =
            interpreter
                .memory
                .get_mut_slice(ret_offset, ret_len, &mut interpreter.gas_left)?;
        let input =
            interpreter
                .memory
                .get_mut_slice(args_offset, args_len, &mut interpreter.gas_left)?;

        let gas_left = interpreter.gas_left.as_u64();
        let limit = gas_left - gas_left / 64;
        let mut endowment = gas.into_u64_saturating();
        endowment = min(endowment, limit); // cap gas at all but one 64th of gas left

        let call_message = if DELEGATE {
            ExecutionMessage {
                kind: MessageKind::EVMC_DELEGATECALL,
                flags: interpreter.message.flags,
                depth: interpreter.message.depth + 1,
                gas: endowment.cast_signed(),
                recipient: interpreter.message.recipient,
                sender: interpreter.message.sender,
                input,
                value: interpreter.message.value,
                create2_salt: u256::ZERO.into(), // ignored
                code_address: addr,
                code: &[],
                code_hash: None,
            }
        } else {
            ExecutionMessage {
                kind: MessageKind::EVMC_CALL,
                flags: MessageFlags::EVMC_STATIC as u32,
                depth: interpreter.message.depth + 1,
                gas: endowment.cast_signed(),
                recipient: addr,
                sender: interpreter.message.recipient,
                input,
                value: u256::ZERO.into(),        // ignored
                create2_salt: u256::ZERO.into(), // ignored
                code_address: addr,
                code: &[],
                code_hash: None,
            }
        };

        let result = interpreter.context.call(&call_message);
        let (status_code, call_gas_left, call_gas_refund) =
            (result.status_code(), result.gas_left(), result.gas_refund());
        interpreter.last_call_return_data = LastCallReturnData::CallResult(result);
        let dest =
            interpreter
                .memory
                .get_mut_slice(ret_offset, ret_len, &mut interpreter.gas_left)?;
        let output = &interpreter.last_call_return_data;
        let min_len = min(output.len(), ret_len as usize); // ret_len == dest.len()
        dest[..min_len].copy_from_slice(&output[..min_len]);

        // Deduct before crediting (like geth) so that the gas left never exceeds Gas::MAX. Neither
        // can fail: the endowment fits the gas left, and the callee returns at most the endowment.
        interpreter.gas_left.consume(endowment)?;
        interpreter.gas_left.add(call_gas_left)?;
        interpreter.gas_refund.add(call_gas_refund);

        interpreter
            .stack
            .push(status_code == StatusCode::EVMC_SUCCESS)?;
        interpreter.pc.next();
        tail_call!(interpreter.return_from_op())
    }
}

impl<const STEPPABLE: bool> From<Interpreter<'_, STEPPABLE>> for StepResult {
    fn from(value: Interpreter<STEPPABLE>) -> Self {
        let stack = value
            .stack
            .as_slice()
            .iter()
            .copied()
            .map(Into::into)
            .collect();
        Self {
            step_status_code: value.exec_status.into(),
            status_code: StatusCode::EVMC_SUCCESS,
            revision: value.revision,
            pc: value.pc.code_offset() as u64,
            gas_left: value.gas_left.as_i64(),
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
            value.gas_left.as_i64(),
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
            CodeAnalysis, CodeAnalysisCache, Memory, MockExecutionContextTrait,
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
        assert_eq!(interpreter.stack.as_slice(), []);
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
        let interpreter = Interpreter::new_steppable(
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
        assert_eq!(interpreter.stack.as_slice(), stack);
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
        interpreter.stack.reset_to(&[1u8.into(), 2u8.into()]);
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
        interpreter
            .stack
            .reset_to(&[1u8.into(), 2u8.into(), 3u8.into()]);
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
        interpreter.stack.reset_to(&[1u8.into(), 2u8.into()]);
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
                call_message.kind == MessageKind::EVMC_CALL
                    && call_message.flags == 0
                    && call_message.depth == message.depth + 1
                    && call_message.gas == gas.cast_signed()
                    && call_message.sender == message.recipient
                    && call_message.recipient == Address::from(addr)
                    && call_message.input == input
                    && call_message.value == Uint256::from(value)
                    && call_message.create2_salt == Uint256::from(u256::ZERO)
                    && call_message.code_address == Address::from(addr)
                    && call_message.code.is_empty()
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
