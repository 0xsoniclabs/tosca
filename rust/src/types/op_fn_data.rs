use std::fmt::Debug;

use crate::{
    Opcode,
    interpreter::{self, OpFn},
    types::CodeByteType,
    u256,
};

#[derive(Clone, PartialEq, Eq)]
pub struct OpFnData<const STEPPABLE: bool> {
    func: Option<OpFn<STEPPABLE>>,
    data: u256,
}

impl<const STEPPABLE: bool> OpFnData<STEPPABLE> {
    pub fn data(data: u256) -> Self {
        Self { func: None, data }
    }

    pub fn skip_no_ops_iter(count: usize) -> impl Iterator<Item = Self> {
        let skip_no_ops = Self::func(Opcode::SkipNoOps as u8, (count as u64).into());
        let gen_no_ops = move || Self::func(Opcode::NoOp as u8, u256::ZERO);
        std::iter::once(skip_no_ops).chain(std::iter::repeat_with(gen_no_ops).take(count - 1))
    }

    pub fn func(op: u8, data: u256) -> Self {
        Self {
            func: Some(interpreter::get_jumptable()[op as usize]),
            data,
        }
    }

    pub fn jump_dest() -> Self {
        Self::func(Opcode::JumpDest as u8, u256::ZERO)
    }

    pub fn code_byte_type(&self) -> CodeByteType {
        match self.func {
            None => CodeByteType::DataOrInvalid,
            Some(func) => {
                if std::ptr::fn_addr_eq(
                    func,
                    interpreter::get_jumptable::<STEPPABLE>()[Opcode::JumpDest as u8 as usize],
                ) {
                    CodeByteType::JumpDest
                } else {
                    CodeByteType::Opcode
                }
            }
        }
    }

    pub fn get_func(&self) -> Option<OpFn<STEPPABLE>> {
        self.func
    }

    pub fn get_data(&self) -> u256 {
        self.data
    }
}

impl<const STEPPABLE: bool> Debug for OpFnData<STEPPABLE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpFnData")
            .field("func", &self.func.map(|f| f as *const u8))
            .field("data", &self.data)
            .finish()
    }
}
