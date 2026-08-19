use std::{
    cmp::min,
    ops::{Add, BitAnd, BitOr, BitXor, Div, Mul, Not, Rem, Sub},
};

use evmc_vm::{
    AccessStatus, ExecutionMessage, ExecutionResult, MessageFlags, MessageKind, Revision,
    StatusCode, StepResult, StorageStatus, Uint256,
};

use crate::{
    types::{
        CodeAnalysisCache, CodeReader, ExecStatus, ExecutionContextTrait, FailStatus,
        GetOpcodeError, Memory, Observer, Stack, hash_cache::HashCache, u256,
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

/// Wraps an opcode method in a function pointer, which methods themselves cannot be coerced to
/// because they capture the lifetime of the type. The wrapper tail calls the method to keep the
/// chain of tail calls intact; a closure cannot be used because `become` is not allowed in it.
macro_rules! op_fn {
    ($name:ident) => {{
        fn $name<const STEPPABLE: bool>(i: &mut Interpreter<STEPPABLE>) -> OpResult {
            tail_call!(i.$name())
        }
        $name::<STEPPABLE>
    }};
    ($name:ident::<$generic:literal>) => {{
        fn $name<const STEPPABLE: bool, const N: usize>(
            i: &mut Interpreter<STEPPABLE>,
        ) -> OpResult {
            tail_call!(i.$name::<N>())
        }
        $name::<STEPPABLE, $generic>
    }};
}

const fn gen_jumptable<const STEPPABLE: bool>() -> [OpFn<STEPPABLE>; 256] {
    [
        op_fn!(stop),
        op_fn!(add),
        op_fn!(mul),
        op_fn!(sub),
        op_fn!(div),
        op_fn!(s_div),
        op_fn!(mod_),
        op_fn!(s_mod),
        op_fn!(add_mod),
        op_fn!(mul_mod),
        op_fn!(exp),
        op_fn!(sign_extend),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(lt),
        op_fn!(gt),
        op_fn!(s_lt),
        op_fn!(s_gt),
        op_fn!(eq),
        op_fn!(is_zero),
        op_fn!(and),
        op_fn!(or),
        op_fn!(xor),
        op_fn!(not),
        op_fn!(byte),
        op_fn!(shl),
        op_fn!(shr),
        op_fn!(sar),
        op_fn!(clz),
        op_fn!(jumptable_placeholder),
        op_fn!(sha3),
        std::cfg_select! {
            feature = "fn-ptr-conversion-dispatch" => op_fn!(no_op),
            _ => op_fn!(jumptable_placeholder),
        },
        std::cfg_select! {
            feature = "fn-ptr-conversion-dispatch" => op_fn!(skip_no_ops),
            _ => op_fn!(jumptable_placeholder),
        },
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(address),
        op_fn!(balance),
        op_fn!(origin),
        op_fn!(caller),
        op_fn!(call_value),
        op_fn!(call_data_load),
        op_fn!(call_data_size),
        op_fn!(call_data_copy),
        op_fn!(code_size),
        op_fn!(code_copy),
        op_fn!(gas_price),
        op_fn!(ext_code_size),
        op_fn!(ext_code_copy),
        op_fn!(return_data_size),
        op_fn!(return_data_copy),
        op_fn!(ext_code_hash),
        op_fn!(block_hash),
        op_fn!(coinbase),
        op_fn!(timestamp),
        op_fn!(number),
        op_fn!(prev_randao),
        op_fn!(gas_limit),
        op_fn!(chain_id),
        op_fn!(self_balance),
        op_fn!(base_fee),
        op_fn!(blob_hash),
        op_fn!(blob_base_fee),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(pop),
        op_fn!(m_load),
        op_fn!(m_store),
        op_fn!(m_store8),
        op_fn!(s_load),
        op_fn!(sstore),
        op_fn!(jump),
        op_fn!(jump_i),
        op_fn!(pc),
        op_fn!(m_size),
        op_fn!(gas),
        op_fn!(jump_dest),
        op_fn!(t_load),
        op_fn!(t_store),
        op_fn!(m_copy),
        op_fn!(push0),
        op_fn!(push::<1>),
        op_fn!(push::<2>),
        op_fn!(push::<3>),
        op_fn!(push::<4>),
        op_fn!(push::<5>),
        op_fn!(push::<6>),
        op_fn!(push::<7>),
        op_fn!(push::<8>),
        op_fn!(push::<9>),
        op_fn!(push::<10>),
        op_fn!(push::<11>),
        op_fn!(push::<12>),
        op_fn!(push::<13>),
        op_fn!(push::<14>),
        op_fn!(push::<15>),
        op_fn!(push::<16>),
        op_fn!(push::<17>),
        op_fn!(push::<18>),
        op_fn!(push::<19>),
        op_fn!(push::<20>),
        op_fn!(push::<21>),
        op_fn!(push::<22>),
        op_fn!(push::<23>),
        op_fn!(push::<24>),
        op_fn!(push::<25>),
        op_fn!(push::<26>),
        op_fn!(push::<27>),
        op_fn!(push::<28>),
        op_fn!(push::<29>),
        op_fn!(push::<30>),
        op_fn!(push::<31>),
        op_fn!(push::<32>),
        op_fn!(dup::<1>),
        op_fn!(dup::<2>),
        op_fn!(dup::<3>),
        op_fn!(dup::<4>),
        op_fn!(dup::<5>),
        op_fn!(dup::<6>),
        op_fn!(dup::<7>),
        op_fn!(dup::<8>),
        op_fn!(dup::<9>),
        op_fn!(dup::<10>),
        op_fn!(dup::<11>),
        op_fn!(dup::<12>),
        op_fn!(dup::<13>),
        op_fn!(dup::<14>),
        op_fn!(dup::<15>),
        op_fn!(dup::<16>),
        op_fn!(swap::<1>),
        op_fn!(swap::<2>),
        op_fn!(swap::<3>),
        op_fn!(swap::<4>),
        op_fn!(swap::<5>),
        op_fn!(swap::<6>),
        op_fn!(swap::<7>),
        op_fn!(swap::<8>),
        op_fn!(swap::<9>),
        op_fn!(swap::<10>),
        op_fn!(swap::<11>),
        op_fn!(swap::<12>),
        op_fn!(swap::<13>),
        op_fn!(swap::<14>),
        op_fn!(swap::<15>),
        op_fn!(swap::<16>),
        op_fn!(log::<0>),
        op_fn!(log::<1>),
        op_fn!(log::<2>),
        op_fn!(log::<3>),
        op_fn!(log::<4>),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(create),
        op_fn!(call),
        op_fn!(call_code),
        op_fn!(return_),
        op_fn!(delegate_call),
        op_fn!(create2),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(static_call),
        op_fn!(jumptable_placeholder),
        op_fn!(jumptable_placeholder),
        op_fn!(revert),
        op_fn!(invalid),
        op_fn!(self_destruct),
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
    pub code_reader: CodeReader<'a, STEPPABLE>,
    pub gas_left: Gas,
    pub gas_refund: GasRefund,
    pub output: Box<[u8]>,
    pub stack: Stack,
    pub memory: Memory,
    pub last_call_return_data: Box<[u8]>,
    pub steps: Option<i32>,
    pub hash_cache: &'a HashCache,
}

impl<'a> Interpreter<'a, false> {
    pub fn new(
        revision: Revision,
        message: &'a ExecutionMessage,
        context: &'a mut dyn ExecutionContextTrait,
        code: &'a [u8],
        code_analysis_cache: &'a CodeAnalysisCache<false>,
        hash_cache: &'a HashCache,
    ) -> Self {
        Self {
            exec_status: ExecStatus::Running,
            message,
            context,
            revision,
            code_reader: CodeReader::new(
                code,
                message.code_hash.map(u256::from),
                0,
                code_analysis_cache,
            ),
            gas_left: Gas::new(message.gas),
            gas_refund: GasRefund::new(0),
            output: Box::default(),
            stack: Stack::new(&[]),
            memory: Memory::new(&[]),
            last_call_return_data: Box::default(),
            steps: None,
            hash_cache,
        }
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
        stack: Stack,
        memory: Memory,
        last_call_return_data: Box<[u8]>,
        steps: Option<i32>,
        code_analysis_cache: &'a CodeAnalysisCache<true>,
        hash_cache: &'a HashCache,
    ) -> Self {
        Self {
            exec_status: ExecStatus::Running,
            message,
            context,
            revision,
            code_reader: CodeReader::new(
                code,
                message.code_hash.map(u256::from),
                pc,
                code_analysis_cache,
            ),
            gas_left: Gas::new(message.gas),
            gas_refund: GasRefund::new(gas_refund),
            output: Box::default(),
            stack,
            memory,
            last_call_return_data,
            steps,
            hash_cache,
        }
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
            let op = match self.code_reader.get() {
                Ok(op) => op,
                Err(GetOpcodeError::OutOfRange) => {
                    self.exec_status = ExecStatus::Stopped;
                    break;
                }
                Err(GetOpcodeError::Invalid) => {
                    return FailStatus::InvalidInstruction.into();
                }
            };
            observer.pre_op(&self);
            let res = std::cfg_select! {
                feature = "fn-ptr-conversion-dispatch" => op(&mut self),
                _ => get_jumptable()[op as usize](&mut self),
            };
            if let Err(err) = res {
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
        let op = match self.code_reader.get() {
            Ok(op) => op,
            Err(GetOpcodeError::OutOfRange) => {
                self.exec_status = ExecStatus::Stopped;
                return Ok(());
            }
            Err(GetOpcodeError::Invalid) => {
                return Err(FailStatus::InvalidInstruction);
            }
        };
        std::cfg_select! {
            feature = "fn-ptr-conversion-dispatch" => tail_call!(op(self)),
            _ => tail_call!(get_jumptable()[op as usize](self)),
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

    #[expect(clippy::unused_self)]
    pub fn jumptable_placeholder(&mut self) -> OpResult {
        Err(FailStatus::Failure)
    }

    /// Performs an operation that charges `cost` gas, pops one value and pushes `op` applied to it.
    #[inline(always)]
    fn unary_op<I: Into<u256>>(&mut self, op: fn(u256) -> I, cost: u64) -> OpResult {
        self.gas_left.consume(cost)?;
        let (push_location, [value]) = self.stack.pop_with_location()?;
        push_location.push(op(value));
        self.code_reader.next();
        Ok(())
    }

    /// Performs an operation that charges `cost` gas, pops two values and pushes `op` applied to
    /// them. `op` receives the top of stack as its first argument.
    #[inline(always)]
    fn binary_op<I: Into<u256>>(&mut self, op: fn(u256, u256) -> I, cost: u64) -> OpResult {
        self.gas_left.consume(cost)?;
        let (push_location, [value2, value1]) = self.stack.pop_with_location()?;
        push_location.push(op(value1, value2));
        self.code_reader.next();
        Ok(())
    }

    /// Performs an operation that charges `cost` gas and pushes `op` applied to the interpreter
    /// state.
    #[inline(always)]
    fn push_value_op<I: Into<u256>>(&mut self, op: fn(&mut Self) -> I, cost: u64) -> OpResult {
        self.gas_left.consume(cost)?;
        let value = op(self);
        self.stack.push(value)?;
        self.code_reader.next();
        Ok(())
    }

    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    pub fn no_op(&mut self) -> OpResult {
        self.code_reader.next();
        tail_call!(self.return_from_op())
    }

    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    pub fn skip_no_ops(&mut self) -> OpResult {
        self.code_reader.jump_to();
        tail_call!(self.return_from_op())
    }

    fn stop(&mut self) -> OpResult {
        self.exec_status = ExecStatus::Stopped;
        Ok(())
    }

    fn add(&mut self) -> OpResult {
        self.binary_op(u256::add, 3)?;
        tail_call!(self.return_from_op())
    }

    fn mul(&mut self) -> OpResult {
        self.binary_op(u256::mul, 5)?;
        tail_call!(self.return_from_op())
    }

    fn sub(&mut self) -> OpResult {
        self.binary_op(u256::sub, 3)?;
        tail_call!(self.return_from_op())
    }

    fn div(&mut self) -> OpResult {
        self.binary_op(u256::div, 5)?;
        tail_call!(self.return_from_op())
    }

    fn s_div(&mut self) -> OpResult {
        self.binary_op(u256::sdiv, 5)?;
        tail_call!(self.return_from_op())
    }

    fn mod_(&mut self) -> OpResult {
        self.binary_op(u256::rem, 5)?;
        tail_call!(self.return_from_op())
    }

    fn s_mod(&mut self) -> OpResult {
        self.binary_op(u256::srem, 5)?;
        tail_call!(self.return_from_op())
    }

    fn add_mod(&mut self) -> OpResult {
        self.gas_left.consume(8)?;
        let (push_location, [denominator, value2, value1]) = self.stack.pop_with_location()?;
        push_location.push(u256::addmod(value1, value2, denominator));
        self.code_reader.next();
        tail_call!(self.return_from_op())
    }

    fn mul_mod(&mut self) -> OpResult {
        self.gas_left.consume(8)?;
        let (push_location, [denominator, fac2, fac1]) = self.stack.pop_with_location()?;
        push_location.push(u256::mulmod(fac1, fac2, denominator));
        self.code_reader.next();
        tail_call!(self.return_from_op())
    }

    fn exp(&mut self) -> OpResult {
        self.gas_left.consume(10)?;
        let (push_location, [exp, value]) = self.stack.pop_with_location()?;
        self.gas_left.consume(exp.bits().div_ceil(8) as u64 * 50)?; // * does not overflow
        push_location.push(value.pow(exp));
        self.code_reader.next();
        tail_call!(self.return_from_op())
    }

    fn sign_extend(&mut self) -> OpResult {
        self.binary_op(u256::signextend, 5)?;
        tail_call!(self.return_from_op())
    }

    fn lt(&mut self) -> OpResult {
        self.binary_op(|lhs, rhs| lhs < rhs, 3)?;
        tail_call!(self.return_from_op())
    }

    fn gt(&mut self) -> OpResult {
        self.binary_op(|lhs, rhs| lhs > rhs, 3)?;
        tail_call!(self.return_from_op())
    }

    fn s_lt(&mut self) -> OpResult {
        self.binary_op(|lhs, rhs| lhs.slt(&rhs), 3)?;
        tail_call!(self.return_from_op())
    }

    fn s_gt(&mut self) -> OpResult {
        self.binary_op(|lhs, rhs| lhs.sgt(&rhs), 3)?;
        tail_call!(self.return_from_op())
    }

    fn eq(&mut self) -> OpResult {
        self.binary_op(|lhs, rhs| lhs == rhs, 3)?;
        tail_call!(self.return_from_op())
    }

    fn is_zero(&mut self) -> OpResult {
        self.unary_op(|value| value == u256::ZERO, 3)?;
        tail_call!(self.return_from_op())
    }

    fn and(&mut self) -> OpResult {
        self.binary_op(u256::bitand, 3)?;
        tail_call!(self.return_from_op())
    }

    fn or(&mut self) -> OpResult {
        self.binary_op(u256::bitor, 3)?;
        tail_call!(self.return_from_op())
    }

    fn xor(&mut self) -> OpResult {
        self.binary_op(u256::bitxor, 3)?;
        tail_call!(self.return_from_op())
    }

    fn not(&mut self) -> OpResult {
        self.unary_op(u256::not, 3)?;
        tail_call!(self.return_from_op())
    }

    fn byte(&mut self) -> OpResult {
        self.binary_op(|offset, value| value.byte(offset), 3)?;
        tail_call!(self.return_from_op())
    }

    fn shl(&mut self) -> OpResult {
        self.binary_op(|shift, value| value << shift, 3)?;
        tail_call!(self.return_from_op())
    }

    fn shr(&mut self) -> OpResult {
        self.binary_op(|shift, value| value >> shift, 3)?;
        tail_call!(self.return_from_op())
    }

    fn sar(&mut self) -> OpResult {
        self.binary_op(|shift, value| value.sar(shift), 3)?;
        tail_call!(self.return_from_op())
    }

    fn clz(&mut self) -> OpResult {
        check_min_revision(Revision::EVMC_OSAKA, self.revision)?;
        self.unary_op(|value| value.leading_zeros(), 5)?;
        tail_call!(self.return_from_op())
    }

    fn sha3(&mut self) -> OpResult {
        self.gas_left.consume(30)?;
        let (push_location, [len, offset]) = self.stack.pop_with_location()?;

        let len = u64::try_from(len).map_err(|_| FailStatus::OutOfGas)?;
        self.gas_left.consume(6 * word_size(len)?)?; // * does not overflow

        let data = self.memory.get_mut_slice(offset, len, &mut self.gas_left)?;
        push_location.push(self.hash_cache.hash(data));
        self.code_reader.next();
        tail_call!(self.return_from_op())
    }

    fn address(&mut self) -> OpResult {
        self.push_value_op(|i| i.message.recipient, 2)?;
        tail_call!(self.return_from_op())
    }

    fn balance(&mut self) -> OpResult {
        if self.revision < Revision::EVMC_BERLIN {
            self.gas_left.consume(700)?;
        }
        let (push_location, [addr]) = self.stack.pop_with_location()?;
        let addr = addr.into();
        self.gas_left
            .consume_address_access_cost(&addr, self.revision, self.context)?;
        push_location.push(self.context.get_balance(&addr));
        self.code_reader.next();
        tail_call!(self.return_from_op())
    }

    fn origin(&mut self) -> OpResult {
        self.push_value_op(|i| i.context.get_tx_context().tx_origin, 2)?;
        tail_call!(self.return_from_op())
    }

    fn caller(&mut self) -> OpResult {
        self.push_value_op(|i| i.message.sender, 2)?;
        tail_call!(self.return_from_op())
    }

    fn call_value(&mut self) -> OpResult {
        self.push_value_op(|i| i.message.value, 2)?;
        tail_call!(self.return_from_op())
    }

    fn call_data_load(&mut self) -> OpResult {
        self.gas_left.consume(3)?;
        let (push_location, [offset]) = self.stack.pop_with_location()?;
        let (offset, overflow) = offset.into_u64_with_overflow();
        let offset = offset as usize;
        let call_data = self.message.input;
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
        self.code_reader.next();
        tail_call!(self.return_from_op())
    }

    fn call_data_size(&mut self) -> OpResult {
        self.push_value_op(|i| i.message.input.len(), 2)?;
        tail_call!(self.return_from_op())
    }

    fn push0(&mut self) -> OpResult {
        check_min_revision(Revision::EVMC_SHANGHAI, self.revision)?;
        self.push_value_op(|_| u256::ZERO, 2)?;
        tail_call!(self.return_from_op())
    }

    fn call_data_copy(&mut self) -> OpResult {
        self.gas_left.consume(3)?;
        let [len, offset, dest_offset] = self.stack.pop()?;

        if len != u256::ZERO {
            let len = u64::try_from(len).map_err(|_| FailStatus::InvalidMemoryAccess)?;

            let src = self.message.input.get_within_bounds(offset, len);
            let dest = self
                .memory
                .get_mut_slice(dest_offset, len, &mut self.gas_left)?;
            dest.copy_padded(src, &mut self.gas_left)?;
        }
        self.code_reader.next();
        tail_call!(self.return_from_op())
    }

    fn code_size(&mut self) -> OpResult {
        self.push_value_op(|i| i.code_reader.len(), 2)?;
        tail_call!(self.return_from_op())
    }

    fn code_copy(&mut self) -> OpResult {
        self.gas_left.consume(3)?;
        let [len, offset, dest_offset] = self.stack.pop()?;

        if len != u256::ZERO {
            let len = u64::try_from(len).map_err(|_| FailStatus::OutOfGas)?;

            let src = self.code_reader.get_within_bounds(offset, len);
            let dest = self
                .memory
                .get_mut_slice(dest_offset, len, &mut self.gas_left)?;
            dest.copy_padded(src, &mut self.gas_left)?;
        }
        self.code_reader.next();
        tail_call!(self.return_from_op())
    }

    fn gas_price(&mut self) -> OpResult {
        self.push_value_op(|i| i.context.get_tx_context().tx_gas_price, 2)?;
        tail_call!(self.return_from_op())
    }

    fn ext_code_size(&mut self) -> OpResult {
        if self.revision < Revision::EVMC_BERLIN {
            self.gas_left.consume(700)?;
        }
        let (push_location, [addr]) = self.stack.pop_with_location()?;
        let addr = addr.into();
        self.gas_left
            .consume_address_access_cost(&addr, self.revision, self.context)?;
        push_location.push(self.context.get_code_size(&addr));
        self.code_reader.next();
        tail_call!(self.return_from_op())
    }

    fn ext_code_copy(&mut self) -> OpResult {
        if self.revision < Revision::EVMC_BERLIN {
            self.gas_left.consume(700)?;
        }
        let [len, offset, dest_offset, addr] = self.stack.pop()?;
        let addr = addr.into();

        self.gas_left
            .consume_address_access_cost(&addr, self.revision, self.context)?;
        if len != u256::ZERO {
            let len = u64::try_from(len).map_err(|_| FailStatus::OutOfGas)?;

            let dest = self
                .memory
                .get_mut_slice(dest_offset, len, &mut self.gas_left)?;
            let (offset, offset_overflow) = offset.into_u64_with_overflow();
            self.gas_left.consume_copy_cost(len)?;
            let bytes_written = self.context.copy_code(&addr, offset as usize, dest);
            if offset_overflow {
                dest.fill(0);
            } else if (bytes_written as u64) < len {
                dest[bytes_written..].fill(0);
            }
        }
        self.code_reader.next();
        tail_call!(self.return_from_op())
    }

    fn return_data_size(&mut self) -> OpResult {
        self.push_value_op(|i| i.last_call_return_data.len(), 2)?;
        tail_call!(self.return_from_op())
    }

    fn return_data_copy(&mut self) -> OpResult {
        self.gas_left.consume(3)?;
        let [len, offset, dest_offset] = self.stack.pop()?;

        let src = &self.last_call_return_data;
        let (offset, offset_overflow) = offset.into_u64_with_overflow();
        let (len, len_overflow) = len.into_u64_with_overflow();
        let (end, end_overflow) = offset.overflowing_add(len);
        if offset_overflow || len_overflow || end_overflow || end > src.len() as u64 {
            return Err(FailStatus::InvalidMemoryAccess);
        }

        if len != 0 {
            let src = &src[offset as usize..end as usize];
            let dest = self
                .memory
                .get_mut_slice(dest_offset, len, &mut self.gas_left)?;
            dest.copy_padded(src, &mut self.gas_left)?;
        }
        self.code_reader.next();
        tail_call!(self.return_from_op())
    }

    fn ext_code_hash(&mut self) -> OpResult {
        if self.revision < Revision::EVMC_BERLIN {
            self.gas_left.consume(700)?;
        }
        let (push_location, [addr]) = self.stack.pop_with_location()?;
        let addr = addr.into();
        self.gas_left
            .consume_address_access_cost(&addr, self.revision, self.context)?;
        push_location.push(self.context.get_code_hash(&addr));
        self.code_reader.next();
        tail_call!(self.return_from_op())
    }

    fn block_hash(&mut self) -> OpResult {
        self.gas_left.consume(20)?;
        let (push_location, [block_number]) = self.stack.pop_with_location()?;
        push_location.push(
            u64::try_from(block_number)
                .map(|idx| self.context.get_block_hash(idx.cast_signed()).into())
                .unwrap_or(u256::ZERO),
        );
        self.code_reader.next();
        tail_call!(self.return_from_op())
    }

    fn coinbase(&mut self) -> OpResult {
        self.push_value_op(|i| i.context.get_tx_context().block_coinbase, 2)?;
        tail_call!(self.return_from_op())
    }

    fn timestamp(&mut self) -> OpResult {
        self.push_value_op(
            |i| i.context.get_tx_context().block_timestamp.cast_unsigned(),
            2,
        )?;
        tail_call!(self.return_from_op())
    }

    fn number(&mut self) -> OpResult {
        self.push_value_op(
            |i| i.context.get_tx_context().block_number.cast_unsigned(),
            2,
        )?;
        tail_call!(self.return_from_op())
    }

    fn prev_randao(&mut self) -> OpResult {
        self.push_value_op(|i| i.context.get_tx_context().block_prev_randao, 2)?;
        tail_call!(self.return_from_op())
    }

    fn gas_limit(&mut self) -> OpResult {
        self.push_value_op(
            |i| i.context.get_tx_context().block_gas_limit.cast_unsigned(),
            2,
        )?;
        tail_call!(self.return_from_op())
    }

    fn chain_id(&mut self) -> OpResult {
        self.push_value_op(|i| i.context.get_tx_context().chain_id, 2)?;
        tail_call!(self.return_from_op())
    }

    fn self_balance(&mut self) -> OpResult {
        check_min_revision(Revision::EVMC_ISTANBUL, self.revision)?;
        self.gas_left.consume(5)?;
        self.stack.check_overflow(1)?; // Check for stack overflow before querying the host
        let addr = self.message.recipient;
        self.stack.push(self.context.get_balance(&addr))?;
        self.code_reader.next();
        tail_call!(self.return_from_op())
    }

    fn base_fee(&mut self) -> OpResult {
        check_min_revision(Revision::EVMC_LONDON, self.revision)?;
        self.push_value_op(|i| i.context.get_tx_context().block_base_fee, 2)?;
        tail_call!(self.return_from_op())
    }

    fn blob_hash(&mut self) -> OpResult {
        check_min_revision(Revision::EVMC_CANCUN, self.revision)?;
        self.gas_left.consume(3)?;
        let (push_location, [idx]) = self.stack.pop_with_location()?;
        let hashes = self.context.get_tx_context().blob_hashes;
        if let Ok(idx) = u64::try_from(idx)
            && let Some(hash) = hashes.get(idx as usize)
        {
            push_location.push(*hash);
        } else {
            push_location.push(u256::ZERO);
        }
        self.code_reader.next();
        tail_call!(self.return_from_op())
    }

    fn blob_base_fee(&mut self) -> OpResult {
        check_min_revision(Revision::EVMC_CANCUN, self.revision)?;
        self.push_value_op(|i| i.context.get_tx_context().blob_base_fee, 2)?;
        tail_call!(self.return_from_op())
    }

    fn pop(&mut self) -> OpResult {
        self.gas_left.consume(2)?;
        let [_] = self.stack.pop()?;
        self.code_reader.next();
        tail_call!(self.return_from_op())
    }

    fn m_load(&mut self) -> OpResult {
        self.gas_left.consume(3)?;
        let (push_location, [offset]) = self.stack.pop_with_location()?;

        push_location.push(self.memory.get_word(offset, &mut self.gas_left)?);
        self.code_reader.next();
        tail_call!(self.return_from_op())
    }

    fn m_store(&mut self) -> OpResult {
        self.gas_left.consume(3)?;
        let [value, offset] = self.stack.pop()?;

        *self.memory.get_mut_array(offset, &mut self.gas_left)? = value.to_be_bytes();
        self.code_reader.next();
        tail_call!(self.return_from_op())
    }

    fn m_store8(&mut self) -> OpResult {
        self.gas_left.consume(3)?;
        let [value, offset] = self.stack.pop()?;

        let dest = self.memory.get_mut_byte(offset, &mut self.gas_left)?;
        *dest = value.least_significant_byte();
        self.code_reader.next();
        tail_call!(self.return_from_op())
    }

    fn s_load(&mut self) -> OpResult {
        if self.revision < Revision::EVMC_BERLIN {
            self.gas_left.consume(800)?;
        }
        let (push_location, [key]) = self.stack.pop_with_location()?;
        let key = key.into();
        let addr = &self.message.recipient;
        if self.revision >= Revision::EVMC_BERLIN {
            if self.context.access_storage(addr, &key) == AccessStatus::EVMC_ACCESS_COLD {
                self.gas_left.consume(2_100)?;
            } else {
                self.gas_left.consume(100)?;
            }
        }
        let value = self.context.get_storage(addr, &key);
        push_location.push(value);
        self.code_reader.next();
        tail_call!(self.return_from_op())
    }

    fn jump(&mut self) -> OpResult {
        self.gas_left.consume(if STEPPABLE { 8 } else { 8 + 1 })?;
        let [dest] = self.stack.pop()?;
        self.code_reader.try_jump(dest)?;
        if !STEPPABLE {
            self.code_reader.next();
        }
        tail_call!(self.return_from_op())
    }

    fn jump_i(&mut self) -> OpResult {
        self.gas_left.consume(10)?;
        let [cond, dest] = self.stack.pop()?;
        if cond == u256::ZERO {
            self.code_reader.next();
        } else {
            self.code_reader.try_jump(dest)?;
            if !STEPPABLE {
                self.gas_left.consume(1)?;
                self.code_reader.next();
            }
        }
        tail_call!(self.return_from_op())
    }

    fn pc(&mut self) -> OpResult {
        self.push_value_op(|i| i.code_reader.pc(), 2)?;
        tail_call!(self.return_from_op())
    }

    fn m_size(&mut self) -> OpResult {
        self.push_value_op(|i| i.memory.len(), 2)?;
        tail_call!(self.return_from_op())
    }

    fn gas(&mut self) -> OpResult {
        self.push_value_op(|i| i.gas_left.as_u64(), 2)?;
        tail_call!(self.return_from_op())
    }

    fn jump_dest(&mut self) -> OpResult {
        self.gas_left.consume(1)?;
        self.code_reader.next();
        tail_call!(self.return_from_op())
    }

    fn t_load(&mut self) -> OpResult {
        check_min_revision(Revision::EVMC_CANCUN, self.revision)?;
        self.gas_left.consume(100)?;
        let (push_location, [key]) = self.stack.pop_with_location()?;
        let value = self
            .context
            .get_transient_storage(&self.message.recipient, &key.into());
        push_location.push(value);
        self.code_reader.next();
        tail_call!(self.return_from_op())
    }

    fn t_store(&mut self) -> OpResult {
        check_min_revision(Revision::EVMC_CANCUN, self.revision)?;
        check_not_read_only(self.message)?;
        self.gas_left.consume(100)?;
        let [value, key] = self.stack.pop()?;
        self.context
            .set_transient_storage(&self.message.recipient, &key.into(), &value.into());
        self.code_reader.next();
        tail_call!(self.return_from_op())
    }

    fn m_copy(&mut self) -> OpResult {
        check_min_revision(Revision::EVMC_CANCUN, self.revision)?;
        self.gas_left.consume(3)?;
        let [len, offset, dest_offset] = self.stack.pop()?;
        if len != u256::ZERO {
            self.memory
                .copy_within(offset, dest_offset, len, &mut self.gas_left)?;
        }
        self.code_reader.next();
        tail_call!(self.return_from_op())
    }

    fn return_(&mut self) -> OpResult {
        let [len, offset] = self.stack.pop()?;
        let len = u64::try_from(len).map_err(|_| FailStatus::OutOfGas)?;
        let data = self.memory.get_mut_slice(offset, len, &mut self.gas_left)?;
        self.output = Box::from(&*data);
        self.exec_status = ExecStatus::Returned;
        Ok(())
    }

    fn revert(&mut self) -> OpResult {
        let [len, offset] = self.stack.pop()?;
        let len = u64::try_from(len).map_err(|_| FailStatus::OutOfGas)?;
        let data = self.memory.get_mut_slice(offset, len, &mut self.gas_left)?;
        self.output = Box::from(&*data);
        self.exec_status = ExecStatus::Revert;
        Ok(())
    }

    #[expect(clippy::unused_self)]
    fn invalid(&mut self) -> OpResult {
        Err(FailStatus::InvalidInstruction)
    }

    fn self_destruct(&mut self) -> OpResult {
        check_not_read_only(self.message)?;
        self.gas_left.consume(5_000)?;
        let [addr] = self.stack.pop()?;
        let addr = addr.into();

        if self.revision >= Revision::EVMC_BERLIN
            && self.context.access_account(&addr) == AccessStatus::EVMC_ACCESS_COLD
        {
            self.gas_left.consume(2_600)?;
        }

        if u256::from(self.context.get_balance(&self.message.recipient)) > u256::ZERO
            && !self.context.account_exists(&addr)
        {
            self.gas_left.consume(25_000)?;
        }

        let destructed = self.context.selfdestruct(&self.message.recipient, &addr);
        if self.revision <= Revision::EVMC_BERLIN && destructed {
            self.gas_refund.add(24_000);
        }

        self.exec_status = ExecStatus::Stopped;
        Ok(())
    }

    fn sstore(&mut self) -> OpResult {
        check_not_read_only(self.message)?;

        if self.revision >= Revision::EVMC_ISTANBUL && self.gas_left <= 2_300 {
            return Err(FailStatus::OutOfGas);
        }
        let [value, key] = self.stack.pop()?;
        let key = key.into();
        let addr = &self.message.recipient;

        let (dyn_gas_1, dyn_gas_2, dyn_gas_3, refund_1, refund_2, refund_3) =
            if self.revision >= Revision::EVMC_LONDON {
                (100, 2_900, 20_000, 5_000 - 2_100 - 100, 4_800, 20_000 - 100)
            } else if self.revision >= Revision::EVMC_BERLIN {
                (
                    100,
                    2_900,
                    20_000,
                    5_000 - 2_100 - 100,
                    15_000,
                    20_000 - 100,
                )
            } else if self.revision >= Revision::EVMC_ISTANBUL {
                (800, 5_000, 20_000, 4_200, 15_000, 19_200)
            } else {
                (5_000, 5_000, 20_000, 0, 0, 0)
            };

        let status = self.context.set_storage(addr, &key, &value.into());
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
        if self.revision >= Revision::EVMC_BERLIN
            && self.context.access_storage(addr, &key) == AccessStatus::EVMC_ACCESS_COLD
        {
            dyn_gas += 2_100;
        }
        self.gas_left.consume(dyn_gas)?;
        self.gas_refund.add(gas_refund_change);
        self.code_reader.next();
        tail_call!(self.return_from_op())
    }

    fn push<const N: usize>(&mut self) -> OpResult {
        self.gas_left.consume(3)?;
        std::cfg_select! {
            feature = "fn-ptr-conversion-dispatch" => {
                self.stack.push(self.code_reader.get_push_data())?;
            }
            _ => {
                self.code_reader.next();
                self.stack.push(self.code_reader.get_push_data::<N>())?;
            }
        }
        tail_call!(self.return_from_op())
    }

    fn dup<const N: usize>(&mut self) -> OpResult {
        self.gas_left.consume(3)?;
        self.stack.dup::<N>()?;
        self.code_reader.next();
        tail_call!(self.return_from_op())
    }

    fn swap<const N: usize>(&mut self) -> OpResult {
        self.gas_left.consume(3)?;
        self.stack.swap_with_top::<N>()?;
        self.code_reader.next();
        tail_call!(self.return_from_op())
    }

    fn log<const N: usize>(&mut self) -> OpResult {
        check_not_read_only(self.message)?;
        self.gas_left.consume(375)?;
        let [len, offset] = self.stack.pop()?;
        let topics: [u256; N] = self.stack.pop()?;
        let (len, len_overflow) = len.into_u64_with_overflow();
        let (len8, len8_overflow) = len.overflowing_mul(8);
        let (cost, cost_overflow) = (375 * N as u64).overflowing_add(len8);
        if len_overflow || len8_overflow || cost_overflow {
            return Err(FailStatus::OutOfGas);
        }
        self.gas_left.consume(cost)?;

        let data = self.memory.get_mut_slice(offset, len, &mut self.gas_left)?;
        let mut topics_uint256 = topics.map(Uint256::from);
        topics_uint256.reverse();
        self.context
            .emit_log(&self.message.recipient, data, &topics_uint256);
        self.code_reader.next();
        tail_call!(self.return_from_op())
    }

    fn create(&mut self) -> OpResult {
        tail_call!(self.create_or_create2::<false>())
    }

    fn create2(&mut self) -> OpResult {
        tail_call!(self.create_or_create2::<true>())
    }

    fn create_or_create2<const CREATE2: bool>(&mut self) -> OpResult {
        self.gas_left.consume(32_000)?;
        check_not_read_only(self.message)?;
        let [len, offset, value] = self.stack.pop()?;
        let salt = if CREATE2 {
            let [salt] = self.stack.pop()?;
            salt
        } else {
            u256::ZERO // ignored
        };
        let len = u64::try_from(len).map_err(|_| FailStatus::OutOfGas)?;

        let init_code_word_size = word_size(len)?;
        if self.revision >= Revision::EVMC_SHANGHAI {
            const MAX_INIT_CODE_LEN: u64 = 2 * 24576;
            if len > MAX_INIT_CODE_LEN {
                return Err(FailStatus::OutOfGas);
            }
            let init_code_cost = 2 * init_code_word_size; // does not overflow
            self.gas_left.consume(init_code_cost)?;
        }
        if CREATE2 {
            let hash_cost = 6 * init_code_word_size; // does not overflow
            self.gas_left.consume(hash_cost)?;
        }

        let init_code = self.memory.get_mut_slice(offset, len, &mut self.gas_left)?;

        if value > self.context.get_balance(&self.message.recipient).into() {
            self.last_call_return_data = Box::default();
            self.stack.push(u256::ZERO)?;
            self.code_reader.next();
            tail_call!(self.return_from_op())
        }

        let gas_left = self.gas_left.as_u64();
        let gas_limit = gas_left - gas_left / 64;
        self.gas_left.consume(gas_limit)?;

        let message = ExecutionMessage {
            kind: if CREATE2 {
                MessageKind::EVMC_CREATE2
            } else {
                MessageKind::EVMC_CREATE
            },
            flags: self.message.flags,
            depth: self.message.depth + 1,
            gas: gas_limit.cast_signed(),
            recipient: u256::ZERO.into(), // ignored
            sender: self.message.recipient,
            input: init_code,
            value: value.into(),
            create2_salt: salt.into(),
            code_address: u256::ZERO.into(), // ignored
            code: &[],
            code_hash: None,
        };
        let result = self.context.call(&message);

        self.gas_left.add(result.gas_left)?;
        self.gas_refund.add(result.gas_refund);

        if result.status_code == StatusCode::EVMC_SUCCESS {
            let Some(addr) = result.create_address else {
                return Err(FailStatus::InternalError);
            };

            self.last_call_return_data = Box::default();
            self.stack.push(addr)?;
        } else {
            self.last_call_return_data = result.output;
            self.stack.push(u256::ZERO)?;
        }
        self.code_reader.next();
        tail_call!(self.return_from_op())
    }

    fn call(&mut self) -> OpResult {
        tail_call!(self.call_or_call_code::<false>())
    }

    fn call_code(&mut self) -> OpResult {
        tail_call!(self.call_or_call_code::<true>())
    }

    fn call_or_call_code<const CODE: bool>(&mut self) -> OpResult {
        if self.revision < Revision::EVMC_BERLIN {
            self.gas_left.consume(700)?;
        }
        let [ret_len, ret_offset, args_len, args_offset, value, addr, gas] = self.stack.pop()?;

        if !CODE && value != u256::ZERO {
            check_not_read_only(self.message)?;
        }

        let addr = addr.into();
        let args_len = u64::try_from(args_len).map_err(|_| FailStatus::OutOfGas)?;
        let ret_len = u64::try_from(ret_len).map_err(|_| FailStatus::OutOfGas)?;

        self.gas_left
            .consume_address_access_cost(&addr, self.revision, self.context)?;
        self.gas_left.consume_positive_value_cost(&value)?;
        if !CODE {
            self.gas_left
                .consume_value_to_empty_account_cost(&value, &addr, self.context)?;
        }
        self.gas_left
            .consume_delegate_resolution_cost(&addr, self.revision, self.context)?;
        // access slice to consume potential memory expansion cost but drop it so that we can get
        // another mutable reference into memory for input
        let _dest = self
            .memory
            .get_mut_slice(ret_offset, ret_len, &mut self.gas_left)?;
        let input = self
            .memory
            .get_mut_slice(args_offset, args_len, &mut self.gas_left)?;

        let gas_left = self.gas_left.as_u64();
        let limit = gas_left - gas_left / 64;
        let mut endowment = gas.into_u64_saturating();
        endowment = min(endowment, limit); // cap gas at all but one 64th of gas left

        let stipend: u64 = if value == u256::ZERO { 0 } else { 2_300 };
        self.gas_left.add(stipend.cast_signed())?;

        if value > u256::from(self.context.get_balance(&self.message.recipient)) {
            self.last_call_return_data = Box::default();
            self.stack.push(u256::ZERO)?;
            self.code_reader.next();
            tail_call!(self.return_from_op())
        }

        let call_message = if CODE {
            ExecutionMessage {
                kind: MessageKind::EVMC_CALLCODE,
                flags: self.message.flags,
                depth: self.message.depth + 1,
                gas: (endowment + stipend).cast_signed(),
                recipient: self.message.recipient,
                sender: self.message.recipient,
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
                flags: self.message.flags,
                depth: self.message.depth + 1,
                gas: (endowment + stipend).cast_signed(),
                recipient: addr,
                sender: self.message.recipient,
                input,
                value: value.into(),
                create2_salt: u256::ZERO.into(), // ignored
                code_address: addr,
                code: &[],
                code_hash: None,
            }
        };

        let result = self.context.call(&call_message);
        self.last_call_return_data = result.output;
        let dest = self
            .memory
            .get_mut_slice(ret_offset, ret_len, &mut self.gas_left)?;
        let output = &self.last_call_return_data;
        let min_len = min(output.len(), ret_len as usize); // ret_len == dest.len()
        dest[..min_len].copy_from_slice(&output[..min_len]);

        self.gas_left.add(result.gas_left)?;
        self.gas_left.consume(endowment)?;
        self.gas_left.consume(stipend)?;
        self.gas_refund.add(result.gas_refund);

        self.stack
            .push(result.status_code == StatusCode::EVMC_SUCCESS)?;
        self.code_reader.next();
        tail_call!(self.return_from_op())
    }

    fn static_call(&mut self) -> OpResult {
        tail_call!(self.static_or_delegate_call::<false>())
    }

    fn delegate_call(&mut self) -> OpResult {
        tail_call!(self.static_or_delegate_call::<true>())
    }

    fn static_or_delegate_call<const DELEGATE: bool>(&mut self) -> OpResult {
        if self.revision < Revision::EVMC_BERLIN {
            self.gas_left.consume(700)?;
        }
        let [ret_len, ret_offset, args_len, args_offset, addr, gas] = self.stack.pop()?;

        let addr = addr.into();
        let args_len = u64::try_from(args_len).map_err(|_| FailStatus::OutOfGas)?;
        let ret_len = u64::try_from(ret_len).map_err(|_| FailStatus::OutOfGas)?;

        self.gas_left
            .consume_address_access_cost(&addr, self.revision, self.context)?;
        self.gas_left
            .consume_delegate_resolution_cost(&addr, self.revision, self.context)?;
        // access slice to consume potential memory expansion cost but drop it so that we can get
        // another mutable reference into memory for input
        let _dest = self
            .memory
            .get_mut_slice(ret_offset, ret_len, &mut self.gas_left)?;
        let input = self
            .memory
            .get_mut_slice(args_offset, args_len, &mut self.gas_left)?;

        let gas_left = self.gas_left.as_u64();
        let limit = gas_left - gas_left / 64;
        let mut endowment = gas.into_u64_saturating();
        endowment = min(endowment, limit); // cap gas at all but one 64th of gas left

        let call_message = if DELEGATE {
            ExecutionMessage {
                kind: MessageKind::EVMC_DELEGATECALL,
                flags: self.message.flags,
                depth: self.message.depth + 1,
                gas: endowment.cast_signed(),
                recipient: self.message.recipient,
                sender: self.message.sender,
                input,
                value: self.message.value,
                create2_salt: u256::ZERO.into(), // ignored
                code_address: addr,
                code: &[],
                code_hash: None,
            }
        } else {
            ExecutionMessage {
                kind: MessageKind::EVMC_CALL,
                flags: MessageFlags::EVMC_STATIC as u32,
                depth: self.message.depth + 1,
                gas: endowment.cast_signed(),
                recipient: addr,
                sender: self.message.recipient,
                input,
                value: u256::ZERO.into(),        // ignored
                create2_salt: u256::ZERO.into(), // ignored
                code_address: addr,
                code: &[],
                code_hash: None,
            }
        };

        let result = self.context.call(&call_message);
        self.last_call_return_data = result.output;
        let dest = self
            .memory
            .get_mut_slice(ret_offset, ret_len, &mut self.gas_left)?;
        let output = &self.last_call_return_data;
        let min_len = min(output.len(), ret_len as usize); // ret_len == dest.len()
        dest[..min_len].copy_from_slice(&output[..min_len]);

        self.gas_left.add(result.gas_left)?;
        self.gas_left.consume(endowment)?;
        self.gas_refund.add(result.gas_refund);

        self.stack
            .push(result.status_code == StatusCode::EVMC_SUCCESS)?;
        self.code_reader.next();
        tail_call!(self.return_from_op())
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
            pc: value.code_reader.pc() as u64,
            gas_left: value.gas_left.as_u64().cast_signed(),
            gas_refund: value.gas_refund.as_i64(),
            output: value.output,
            stack,
            memory: value.memory.as_slice().to_vec(),
            last_call_return_data: value.last_call_return_data,
        }
    }
}

impl<const STEPPABLE: bool> From<Interpreter<'_, STEPPABLE>> for ExecutionResult {
    fn from(value: Interpreter<STEPPABLE>) -> Self {
        Self {
            status_code: value.exec_status.into(),
            gas_left: value.gas_left.as_u64().cast_signed(),
            gas_refund: value.gas_refund.as_i64(),
            output: value.output,
            create_address: None,
        }
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
            CodeAnalysisCache, Memory, MockExecutionContextTrait, MockExecutionMessage,
            NoOpObserver, Opcode, Stack, hash_cache::HashCache, u256,
        },
    };

    #[test]
    fn empty_code() {
        let code_analysis_cache = CodeAnalysisCache::default();
        let hash_cache = HashCache::default();
        let mut context = MockExecutionContextTrait::new();
        let message = MockExecutionMessage::default().into();
        let interpreter = Interpreter::new(
            Revision::EVMC_ISTANBUL,
            &message,
            &mut context,
            &[],
            &code_analysis_cache,
            &hash_cache,
        );
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
        let interpreter = Interpreter::new_steppable(
            Revision::EVMC_ISTANBUL,
            &message,
            &mut context,
            &[Opcode::Add as u8],
            1,
            0,
            Stack::new(&[]),
            Memory::new(&[]),
            Box::default(),
            None,
            &code_analysis_cache,
            &hash_cache,
        );
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
        let result: ExecutionResult = Interpreter::new_steppable(
            Revision::EVMC_ISTANBUL,
            &message,
            &mut context,
            &[Opcode::Push1 as u8, 0x00],
            1,
            0,
            Stack::new(&[]),
            Memory::new(&[]),
            Box::default(),
            None,
            &code_analysis_cache,
            &hash_cache,
        )
        .run(&mut NoOpObserver());
        assert_eq!(result.status_code, StatusCode::EVMC_INVALID_INSTRUCTION);
    }

    #[test]
    fn zero_steps() {
        let code_analysis_cache = CodeAnalysisCache::default();
        let hash_cache = HashCache::default();
        let mut context = MockExecutionContextTrait::new();
        let message = MockExecutionMessage::default().into();
        let interpreter = Interpreter::new_steppable(
            Revision::EVMC_ISTANBUL,
            &message,
            &mut context,
            &[Opcode::Add as u8],
            0,
            0,
            Stack::new(&[]),
            Memory::new(&[]),
            Box::default(),
            Some(0),
            &code_analysis_cache,
            &hash_cache,
        );
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
        let interpreter = Interpreter::new_steppable(
            Revision::EVMC_ISTANBUL,
            &message,
            &mut context,
            &[Opcode::Add as u8, Opcode::Add as u8],
            0,
            0,
            Stack::new(&[1u8.into(), 2u8.into()]),
            Memory::new(&[]),
            Box::default(),
            Some(1),
            &code_analysis_cache,
            &hash_cache,
        );
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
        let mut interpreter = Interpreter::new(
            Revision::EVMC_ISTANBUL,
            &message,
            &mut context,
            &[Opcode::Add as u8],
            &code_analysis_cache,
            &hash_cache,
        );
        interpreter.stack = Stack::new(&[1u8.into(), 2u8.into()]);
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
        let mut interpreter = Interpreter::new(
            Revision::EVMC_ISTANBUL,
            &message,
            &mut context,
            &[Opcode::Add as u8, Opcode::Add as u8],
            &code_analysis_cache,
            &hash_cache,
        );
        interpreter.stack = Stack::new(&[1u8.into(), 2u8.into(), 3u8.into()]);
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
        let interpreter = Interpreter::new(
            Revision::EVMC_ISTANBUL,
            &message,
            &mut context,
            &[Opcode::JumpDest as u8; 10_000_000],
            &code_analysis_cache,
            &hash_cache,
        );
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
        let mut interpreter = Interpreter::new(
            Revision::EVMC_ISTANBUL,
            &message,
            &mut context,
            &[Opcode::Add as u8],
            &code_analysis_cache,
            &hash_cache,
        );
        interpreter.stack = Stack::new(&[1u8.into(), 2u8.into()]);
        let result: ExecutionResult = interpreter.run(&mut NoOpObserver());
        assert_eq!(result.status_code, StatusCode::EVMC_OUT_OF_GAS);
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
            .returning(move |_| ExecutionResult {
                status_code: StatusCode::EVMC_SUCCESS,
                gas_left: 0,
                gas_refund: 0,
                output: Box::from(ret_data.as_slice()),
                create_address: None,
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

        let interpreter = Interpreter::new_steppable(
            Revision::EVMC_ISTANBUL,
            &message,
            &mut context,
            &[Opcode::Call as u8],
            0,
            0,
            Stack::new(&stack),
            Memory::new(&memory),
            Box::default(),
            None,
            &code_analysis_cache,
            &hash_cache,
        );
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
