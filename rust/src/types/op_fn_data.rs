use std::fmt::Debug;

use crate::{
    Opcode,
    interpreter::{self, OpFn},
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

    pub fn get_func(&self) -> OpFn<STEPPABLE> {
        self.func
    }

    pub fn get_data(&self) -> u256 {
        self.data
    }

    pub fn set_data(&mut self, data: u256) {
        self.data = data;
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
