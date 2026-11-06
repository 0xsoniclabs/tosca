use crate::{
    Opcode,
    interpreter::{self, OpFn},
    types::CodeByteType,
    u256,
};

/// An entry of a [`crate::types::CodeAnalysis`]: the handler to dispatch to, the data it reads and
/// the code offset it reports.
// All function pointers stored here are pointers to functions implementing opcodes and come from
// the same jumptable. This means that for the same opcode we always get the same function pointer.
// Since the pointers are the top entry point of the function implementing the opcode, two different
// opcodes cannot have the same function pointer because then they would do the same thing. This
// means that comparing function pointers for equality is equivalent to comparing the opcodes they
// implement for equality.
#[derive(Clone, Debug)]
#[cfg_attr(
    test,
    derive(PartialEq),
    allow(unpredictable_function_pointer_comparisons)
)]
pub struct OpFnData<const STEPPABLE: bool> {
    /// The handler of the opcode.
    func: OpFn<STEPPABLE>,
    /// The push data.
    data: u256,
    /// The code offset of the instruction.
    code_offset: u64,
}

const _: () = assert!(size_of::<OpFnData<false>>() == 48);

impl<const STEPPABLE: bool> OpFnData<STEPPABLE> {
    /// Creates a new [`OpFnData`] with the handler of `op`.
    pub fn func(op: u8, data: u256, code_offset: usize) -> Self {
        Self {
            func: interpreter::get_jumptable()[op as usize],
            data,
            code_offset: code_offset as u64,
        }
    }

    /// Creates a new [`OpFnData`] for an invalid opcode. It holds the [`Opcode::Invalid`] handler
    /// so that executing it fails without a separate check during dispatch.
    pub fn invalid(code_offset: usize) -> Self {
        Self::func(Opcode::Invalid as u8, u256::ZERO, code_offset)
    }

    /// Creates a new [`OpFnData`] for a jump destination.
    pub fn jump_dest(code_offset: usize) -> Self {
        Self::func(Opcode::JumpDest as u8, u256::ZERO, code_offset)
    }

    /// Creates a new [`OpFnData`] for the terminator appended to every code analysis. Running past
    /// the end of the code stops execution, so dispatching needs no bounds check as long as the
    /// program counter cannot jump past this entry. Its code offset is the pc reached after the
    /// last instruction, which is the code length or, if the data of a trailing push is truncated,
    /// past it.
    pub fn terminator(code_offset: usize) -> Self {
        Self::func(Opcode::Stop as u8, u256::ZERO, code_offset)
    }

    /// Returns the [`CodeByteType`] of the entry, derived from its handler. Push opcodes report
    /// [`CodeByteType::Opcode`].
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

    /// Returns the handler.
    pub fn get_func(&self) -> OpFn<STEPPABLE> {
        self.func
    }

    /// Returns the data.
    pub fn get_data(&self) -> u256 {
        self.data
    }

    /// Returns the code offset.
    pub fn get_code_offset(&self) -> usize {
        self.code_offset as usize
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    const ADD: u8 = Opcode::Add as u8;

    #[test]
    fn op_fn_data_func_sets_the_handler_of_op_the_data_and_the_code_offset() {
        let entry = OpFnData::<false>::func(ADD, 1u8.into(), 2);
        assert!(std::ptr::fn_addr_eq(
            entry.get_func(),
            interpreter::get_jumptable::<false>()[ADD as usize]
        ));
        assert_eq!(entry.get_data(), 1u8.into());
        assert_eq!(entry.get_code_offset(), 2);
    }

    #[test]
    fn op_fn_data_invalid_holds_the_invalid_handler_zero_data_and_the_code_offset() {
        assert_eq!(
            OpFnData::<false>::invalid(2),
            OpFnData::func(Opcode::Invalid as u8, u256::ZERO, 2)
        );
    }

    #[test]
    fn op_fn_data_jump_dest_holds_the_jump_dest_handler_zero_data_and_the_code_offset() {
        assert_eq!(
            OpFnData::<false>::jump_dest(2),
            OpFnData::func(Opcode::JumpDest as u8, u256::ZERO, 2)
        );
    }

    #[test]
    fn op_fn_data_terminator_holds_the_stop_handler_zero_data_and_the_code_offset() {
        assert_eq!(
            OpFnData::<false>::terminator(2),
            OpFnData::func(Opcode::Stop as u8, u256::ZERO, 2)
        );
    }

    #[rstest]
    #[case::jump_dest(OpFnData::jump_dest(0), CodeByteType::JumpDest)]
    #[case::invalid(OpFnData::invalid(0), CodeByteType::DataOrInvalid)]
    #[case::opcode(OpFnData::func(ADD, u256::ZERO, 0), CodeByteType::Opcode)]
    #[case::push(OpFnData::func(Opcode::Push1 as u8, u256::ONE, 0), CodeByteType::Opcode)]
    #[case::terminator(OpFnData::terminator(0), CodeByteType::Opcode)]
    fn op_fn_data_code_byte_type_derives_the_type_from_the_handler(
        #[case] entry: OpFnData<false>,
        #[case] expected: CodeByteType,
    ) {
        assert_eq!(entry.code_byte_type(), expected);
    }

    #[test]
    fn op_fn_data_get_func_returns_the_handler() {
        let func = interpreter::get_jumptable::<false>()[ADD as usize];
        let entry = OpFnData {
            func,
            data: u256::ZERO,
            code_offset: 0,
        };
        assert!(std::ptr::fn_addr_eq(entry.get_func(), func));
    }

    #[test]
    fn op_fn_data_get_data_returns_the_data() {
        let entry = OpFnData::<false> {
            func: interpreter::get_jumptable()[ADD as usize],
            data: 1u8.into(),
            code_offset: 0,
        };
        assert_eq!(entry.get_data(), 1u8.into());
    }

    #[test]
    fn op_fn_data_get_code_offset_returns_the_code_offset() {
        let entry = OpFnData::<false> {
            func: interpreter::get_jumptable()[ADD as usize],
            data: u256::ZERO,
            code_offset: 2,
        };
        assert_eq!(entry.get_code_offset(), 2);
    }

    #[test]
    fn op_fn_data_debug_prints_the_handler_address_the_data_and_the_code_offset() {
        let func = interpreter::get_jumptable::<false>()[ADD as usize];
        assert_eq!(
            format!("{:?}", OpFnData::<false>::func(ADD, u256::ONE, 2)),
            format!(
                "OpFnData {{ func: {:?}, data: u256(1), code_offset: 2 }}",
                func as *const u8
            )
        );
    }
}
