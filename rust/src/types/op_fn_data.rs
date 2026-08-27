use std::fmt::Debug;

use crate::{
    Opcode,
    interpreter::{self, OpFn},
    types::CodeByteType,
    u256,
};

// All function pointers stored here are pointers to functions implementing opcodes and come from
// the same jumptable. This means that for the same opcode we always get the same function pointer.
// Since the pointers are the top entry point of the function implementing the opcode, two different
// opcodes cannot have the same function pointer because then they would do the same thing. This
// means that comparing function pointers for equality is equivalent to comparing the opcodes they
// implement for equality.
#[allow(unpredictable_function_pointer_comparisons)]
#[derive(Clone, PartialEq, Eq)]
pub struct OpFnData<const STEPPABLE: bool> {
    func: OpFn<STEPPABLE>,
    data: u256,
}

impl<const STEPPABLE: bool> OpFnData<STEPPABLE> {
    pub fn data(data: u256) -> Self {
        // Data entries hold the [Opcode::Invalid] handler so that executing them (only possible for
        // invalid opcodes, push data is never materialized as an entry) fails like an invalid
        // opcode without a separate check during dispatch.
        Self {
            func: interpreter::get_jumptable()[Opcode::Invalid as u8 as usize],
            data,
        }
    }

    pub fn skip_no_ops_iter(count: usize) -> impl Iterator<Item = Self> {
        let skip_no_ops = Self::func(Opcode::SkipNoOps as u8, (count as u64).into());
        let no_op = Self::func(Opcode::NoOp as u8, u256::ZERO);
        std::iter::once(skip_no_ops).chain(std::iter::repeat_n(no_op, count - 1))
    }

    pub fn func(op: u8, data: u256) -> Self {
        Self {
            func: interpreter::get_jumptable()[op as usize],
            data,
        }
    }

    pub fn jump_dest() -> Self {
        Self::func(Opcode::JumpDest as u8, u256::ZERO)
    }

    /// The terminator appended to every code analysis. Running past the end of the code stops
    /// execution, so dispatching needs no bounds check as long as the program counter cannot jump
    /// past this entry.
    pub fn terminator() -> Self {
        Self::func(Opcode::Stop as u8, u256::ZERO)
    }

    pub fn code_byte_type(&self) -> CodeByteType {
        let jumptable = interpreter::get_jumptable::<STEPPABLE>();
        if std::ptr::fn_addr_eq(self.func, jumptable[Opcode::JumpDest as u8 as usize]) {
            CodeByteType::JumpDest
        } else if std::ptr::fn_addr_eq(self.func, jumptable[Opcode::Invalid as u8 as usize]) {
            CodeByteType::DataOrInvalid
        } else {
            CodeByteType::Opcode
        }
    }

    pub fn get_func(&self) -> OpFn<STEPPABLE> {
        self.func
    }

    pub fn get_data(&self) -> u256 {
        self.data
    }
}

impl<const STEPPABLE: bool> Debug for OpFnData<STEPPABLE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpFnData")
            .field("func", &(self.func as *const u8))
            .field("data", &self.data)
            .finish()
    }
}
