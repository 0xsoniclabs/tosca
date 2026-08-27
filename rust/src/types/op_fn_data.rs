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
    code_offset: u64,
}

const _: () = assert!(size_of::<OpFnData<false>>() == 48);

impl<const STEPPABLE: bool> OpFnData<STEPPABLE> {
    pub fn data(data: u256, code_offset: usize) -> Self {
        // Data entries hold the [Opcode::Invalid] handler so that executing them (only possible for
        // invalid opcodes, push data is never materialized as an entry) fails like an invalid
        // opcode without a separate check during dispatch.
        Self {
            func: interpreter::get_jumptable()[Opcode::Invalid as u8 as usize],
            data,
            code_offset: code_offset as u64,
        }
    }

    /// The padding entries in front of a jump destination. They all report the code offset of that
    /// jump destination, which is the only offset reachable through them.
    pub fn skip_no_ops_iter(count: usize, code_offset: usize) -> impl Iterator<Item = Self> {
        let skip_no_ops = Self::func(Opcode::SkipNoOps as u8, (count as u64).into(), code_offset);
        let no_op = Self::func(Opcode::NoOp as u8, u256::ZERO, code_offset);
        std::iter::once(skip_no_ops).chain(std::iter::repeat_n(no_op, count - 1))
    }

    pub fn func(op: u8, data: u256, code_offset: usize) -> Self {
        Self {
            func: interpreter::get_jumptable()[op as usize],
            data,
            code_offset: code_offset as u64,
        }
    }

    pub fn jump_dest(code_offset: usize) -> Self {
        Self::func(Opcode::JumpDest as u8, u256::ZERO, code_offset)
    }

    /// The terminator appended to every code analysis. Running past the end of the code stops
    /// execution, so dispatching needs no bounds check as long as the program counter cannot jump
    /// past this entry. Its code offset is the pc reached after the last instruction, which is the
    /// code length or, if the data of a trailing push is truncated, past it.
    pub fn terminator(code_offset: usize) -> Self {
        Self::func(Opcode::Stop as u8, u256::ZERO, code_offset)
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

    pub fn get_code_offset(&self) -> usize {
        self.code_offset as usize
    }
}

impl<const STEPPABLE: bool> Debug for OpFnData<STEPPABLE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpFnData")
            .field("func", &(self.func as *const u8))
            .field("data", &self.data)
            .field("code_offset", &self.code_offset)
            .finish()
    }
}
