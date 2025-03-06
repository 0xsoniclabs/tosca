use std::fmt::Debug;

use crate::{
    interpreter::{OpFn, JUMPTABLE},
    types::CodeByteType,
    Opcode,
};

#[derive(Clone, PartialEq, Eq)]
pub struct OpFnData {
    func: Option<OpFn>,
    data: usize,
}

impl OpFnData {
    pub fn invalid() -> Self {
        Self {
            func: None,
            data: 0,
        }
    }

    pub fn skip_no_ops_iter(count: usize) -> impl Iterator<Item = Self> {
        let skip_no_ops = Self::func(Opcode::SkipNoOps as u8, count);
        let gen_no_ops = move || Self::func(Opcode::NoOp as u8, 0);
        std::iter::once(skip_no_ops).chain(std::iter::repeat_with(gen_no_ops).take(count - 1))
    }

    pub fn func(op: u8, data: usize) -> Self {
        Self {
            func: Some(JUMPTABLE[op as usize]),
            data,
        }
    }

    pub fn jump_dest() -> Self {
        Self::func(Opcode::JumpDest as u8, 0)
    }

    pub fn code_byte_type(&self) -> CodeByteType {
        match self.func {
            None => CodeByteType::DataOrInvalid,
            Some(func) if func == JUMPTABLE[Opcode::JumpDest as u8 as usize] => {
                CodeByteType::JumpDest
            }
            Some(_) => CodeByteType::Opcode,
        }
    }

    pub fn get_func(&self) -> Option<OpFn> {
        self.func
    }

    pub fn get_data(&self) -> usize {
        self.data
    }
}

impl Debug for OpFnData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpFnData")
            .field("func", &self.func.map(|f| f as *const u8))
            .field("data", &self.data)
            .finish()
    }
}
