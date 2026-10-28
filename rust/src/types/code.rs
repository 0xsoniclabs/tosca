#[cfg(feature = "fn-ptr-conversion-dispatch")]
use std::ptr::NonNull;
use std::{self, marker::PhantomData, ops::Deref};

use crate::types::{CodeAnalysis, CodeByteType, FailStatus, code_analysis::AnalysisItem, u256};
#[cfg(feature = "fn-ptr-conversion-dispatch")]
use crate::{interpreter::OpFn, types::OpFnData};

/// The code of a run and its analysis, which stay fixed while a [`Pc`] moves over them. Both are
/// borrowed, the analysis from the [`CodeAnalysis`] that the caller owns, so a run can hold it by
/// value.
#[derive(Debug)]
pub struct Code<'a, const STEPPABLE: bool> {
    code: &'a [u8],
    code_analysis: &'a [AnalysisItem<STEPPABLE>],
}

impl<const STEPPABLE: bool> Deref for Code<'_, STEPPABLE> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.code
    }
}

/// The error [`Code::get`] returns if there is no opcode at the program counter.
#[cfg(not(feature = "fn-ptr-conversion-dispatch"))]
#[derive(Debug, PartialEq, Eq)]
pub enum GetOpcodeError {
    /// The counter is past the end of the code.
    OutOfRange,
    /// The byte at the counter is push data or an invalid opcode.
    Invalid,
}

impl<'a, const STEPPABLE: bool> Code<'a, STEPPABLE> {
    /// Creates a new [`Code`] from the bytes of a code and their analysis.
    pub fn new(code: &'a [u8], code_analysis: &'a CodeAnalysis<STEPPABLE>) -> Self {
        Self {
            code,
            code_analysis,
        }
    }

    /// Returns the opcode at `pc` or a [`GetOpcodeError`] if there is none.
    #[cfg(not(feature = "fn-ptr-conversion-dispatch"))]
    pub fn get(&self, pc: Pc<'a, STEPPABLE>) -> Result<u8, GetOpcodeError> {
        if let Some(op) = self.code.get(pc.offset) {
            let analysis = self.code_analysis[pc.offset];
            if analysis == CodeByteType::DataOrInvalid {
                Err(GetOpcodeError::Invalid)
            } else {
                Ok(*op)
            }
        } else {
            Err(GetOpcodeError::OutOfRange)
        }
    }

    /// Moves `pc` to `dest`, which has to be a jump destination.
    pub fn try_jump(&self, pc: &mut Pc<'a, STEPPABLE>, dest: u256) -> Result<(), FailStatus> {
        let dest = u64::try_from(dest).map_err(|_| FailStatus::BadJumpDestination)? as usize;

        let Some(analysis_item) = self.code_analysis.get(dest) else {
            std::hint::cold_path();
            return Err(FailStatus::BadJumpDestination);
        };

        let code_byte_type = std::cfg_select! {
            feature = "fn-ptr-conversion-dispatch" => analysis_item.code_byte_type(),
            _ => *analysis_item,
        };
        if code_byte_type != CodeByteType::JumpDest {
            std::hint::cold_path();
            return Err(FailStatus::BadJumpDestination);
        }
        *pc = Pc::at(self.code_analysis, dest);

        Ok(())
    }

    /// Returns the `N` bytes of push data at `pc` and advances `pc` past them.
    #[cfg(not(feature = "fn-ptr-conversion-dispatch"))]
    pub fn get_push_data<const N: usize>(&self, pc: &mut Pc<'a, STEPPABLE>) -> u256 {
        const { assert!(N > 0 && N <= 32) };

        // N is known at compile time, so copying a whole window of push data compiles to a fixed
        // size copy. Only a push running past the end of the code needs a runtime length copy,
        // which is a call to memcpy.
        let mut data = [0; 32];
        if let Some(window) = self.code.get(pc.offset..pc.offset + N) {
            data[32 - N..].copy_from_slice(window);
        } else {
            let data_len = self.code.len() - pc.offset;
            data[32 - N..32 - N + data_len].copy_from_slice(&self.code[pc.offset..]);
        }
        let data = u256::from_be_bytes(data);
        pc.offset += N;

        data
    }
}

/// The program counter of a run: one word that points into a [`Code`], small enough to pass in a
/// register.
#[derive(Clone, Copy, Debug)]
pub struct Pc<'a, const STEPPABLE: bool> {
    /// The current entry of the analysis. It always points at a valid entry: execution cannot
    /// advance past the terminator and jumps are bounds checked. The borrow keeps the analysis
    /// alive, and nothing mutates it.
    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    entry: NonNull<OpFnData<STEPPABLE>>,
    /// The offset into the code, which [`Code`] bounds checks on every access.
    #[cfg(not(feature = "fn-ptr-conversion-dispatch"))]
    offset: usize,
    _code: PhantomData<Code<'a, STEPPABLE>>,
}

impl<'a, const STEPPABLE: bool> Pc<'a, STEPPABLE> {
    /// Creates a new [`Pc`] for the instruction at `code_offset`.
    pub fn new(code_analysis: &'a CodeAnalysis<STEPPABLE>, code_offset: usize) -> Self {
        #[cfg(feature = "fn-ptr-conversion-dispatch")]
        let code_offset = code_analysis.analysis_offset(code_offset);
        Self::at(code_analysis, code_offset)
    }

    /// Creates a new [`Pc`] for the entry at `offset` in `analysis`.
    #[allow(unused_variables)]
    fn at(analysis: &'a [AnalysisItem<STEPPABLE>], offset: usize) -> Self {
        Self {
            // Derived from the tail of the analysis, so its provenance covers the entries ahead.
            #[cfg(feature = "fn-ptr-conversion-dispatch")]
            entry: NonNull::from(&analysis[offset..]).cast(),
            #[cfg(not(feature = "fn-ptr-conversion-dispatch"))]
            offset,
            _code: PhantomData,
        }
    }

    /// Returns the entry at the counter.
    // TODO: technically this method is not safe, because the invariant it relies on can be broken
    // by calling only safe public methods (calling next() until the pc is out of bounds).
    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    fn entry(self) -> &'a OpFnData<STEPPABLE> {
        // SAFETY:
        // The counter always points at a valid entry (see the documentation of Pc).
        unsafe { self.entry.as_ref() }
    }

    /// Returns the handler of the entry at the counter. The analysis ends with a terminator entry
    /// that stops execution and the counter can never advance past it, so there is always an entry
    /// to dispatch to. Invalid opcodes hold the handler for [crate::types::Opcode::Invalid], hence
    /// no error handling is needed either.
    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    pub fn get(self) -> OpFn<STEPPABLE> {
        self.entry().get_func()
    }

    /// Advances the counter to the next entry of the analysis.
    // TODO: technically speaking, this method is not safe because it can break the invariant that
    // the pc always points to a valid analysis item.
    pub fn next(&mut self) {
        std::cfg_select! {
            feature = "fn-ptr-conversion-dispatch" => {
                // SAFETY:
                // next is only called for entries that do not stop execution, and every such entry
                // is followed by another entry because the analysis ends with a terminator.
                self.entry = unsafe { self.entry.add(1) };
            }
            _ => {
                self.offset += 1;
            }
        }
    }

    /// Returns the push data that the current push entry holds.
    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    pub fn get_push_data(self) -> u256 {
        self.entry().get_data()
    }

    /// Advances to the jump destination behind the no-ops, whose distance the entry holds.
    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    pub fn jump_to_jumpdest(&mut self) {
        let offset = self.entry().get_data().into_u64_saturating();
        // SAFETY:
        // A skip-no-ops entry holds the distance to the following jump dest entry, which is in
        // bounds.
        self.entry = unsafe { self.entry.add(offset as usize) };
    }

    /// Returns the code offset of the counter.
    pub fn code_offset(self) -> usize {
        std::cfg_select! {
            feature = "fn-ptr-conversion-dispatch" => self.entry().get_code_offset(),
            _ => self.offset,
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::types::{CodeAnalysisCache, Opcode};

    const ADD: u8 = Opcode::Add as u8;
    const DEST: u8 = Opcode::JumpDest as u8;
    const PUSH1: u8 = Opcode::Push1 as u8;

    #[test]
    fn code_derefs_to_the_code_bytes() {
        let bytes = [PUSH1, 0xff, ADD];
        let cache = CodeAnalysisCache::default();
        let analysis = CodeAnalysis::new(&bytes, None, &cache);
        assert_eq!(*Code::<false>::new(&bytes, &analysis), bytes);
    }

    #[cfg(not(feature = "fn-ptr-conversion-dispatch"))]
    #[rstest]
    #[case::opcode(&[ADD], 0, Ok(ADD))]
    #[case::push(&[PUSH1, 0xff], 0, Ok(PUSH1))]
    #[case::push_data_that_looks_like_an_opcode(&[PUSH1, DEST], 1, Err(GetOpcodeError::Invalid))]
    #[case::invalid(&[0xc0], 0, Err(GetOpcodeError::Invalid))]
    #[case::jump_dest(&[DEST], 0, Ok(DEST))]
    #[case::out_of_range(&[ADD], 1, Err(GetOpcodeError::OutOfRange))]
    fn code_get_returns_the_opcode_or_why_there_is_none(
        #[case] bytes: &[u8],
        #[case] offset: usize,
        #[case] expected: Result<u8, GetOpcodeError>,
    ) {
        let cache = CodeAnalysisCache::default();
        let analysis = CodeAnalysis::new(bytes, None, &cache);
        let code = Code::<false>::new(bytes, &analysis);
        assert_eq!(code.get(Pc::new(&analysis, offset)), expected);
    }

    #[rstest]
    #[case::valid_jump_dest(&[PUSH1, DEST, DEST], 2u8.into(), Ok(()), 2)]
    #[case::push_data_that_looks_like_a_jump_dest(
        &[ADD, PUSH1, DEST, DEST],
        2u8.into(),
        Err(FailStatus::BadJumpDestination),
        0
    )]
    #[case::opcode(&[ADD], 0u8.into(), Err(FailStatus::BadJumpDestination), 0)]
    #[case::end_of_code(&[DEST], 1u8.into(), Err(FailStatus::BadJumpDestination), 0)]
    #[case::past_the_end(&[DEST], 2u8.into(), Err(FailStatus::BadJumpDestination), 0)]
    #[case::above_u64(&[DEST], u256::MAX, Err(FailStatus::BadJumpDestination), 0)]
    fn code_try_jump_moves_the_pc_only_to_jump_dests(
        #[case] bytes: &[u8],
        #[case] dest: u256,
        #[case] expected: Result<(), FailStatus>,
        #[case] code_offset: usize,
    ) {
        let cache = CodeAnalysisCache::default();
        let analysis = CodeAnalysis::new(bytes, None, &cache);
        let code = Code::<false>::new(bytes, &analysis);
        let mut pc = Pc::new(&analysis, 0);
        assert_eq!(code.try_jump(&mut pc, dest), expected);
        assert_eq!(pc.code_offset(), code_offset);
        // The padding in front of a jump dest reports its code offset too, so only the entry after
        // the counter tells them apart.
        pc.next();
        assert_eq!(pc.code_offset(), code_offset + 1);
    }

    #[cfg(not(feature = "fn-ptr-conversion-dispatch"))]
    #[rstest]
    #[case::one_byte(&[1, 2, 3], 0, 1, 1u8.into(), 1)]
    #[case::thirty_two_bytes(&[0xff; 40], 0, 32, u256::MAX, 32)]
    #[case::from_start(&[1, 2, 3, 4], 0, 2, 0x0102u32.into(), 2)]
    #[case::from_middle(&[1, 2, 3, 4], 1, 2, 0x0203u32.into(), 3)]
    #[case::from_end(&[1, 2, 3, 4], 2, 2, 0x0304u32.into(), 4)]
    #[case::past_end_is_zero_padded(&[1, 2, 3, 4], 3, 2, 0x0400u32.into(), 5)]
    #[case::thirty_one_bytes_past_end(&[0xff; 32], 31, 32, u256::from(0xffu8) << u256::from(248u8), 63)]
    #[case::at_end_is_all_zero(&[1, 2, 3, 4], 4, 2, 0x0000u32.into(), 6)]
    fn code_get_push_data_reads_n_bytes_padded_with_zeros_and_advances_the_pc_by_n(
        #[case] bytes: &[u8],
        #[case] offset: usize,
        #[case] n: usize,
        #[case] expected: u256,
        #[case] code_offset: usize,
    ) {
        let cache = CodeAnalysisCache::default();
        let analysis = CodeAnalysis::new(bytes, None, &cache);
        let code = Code::<false>::new(bytes, &analysis);
        let mut pc = Pc::new(&analysis, offset);
        let data = match n {
            1 => code.get_push_data::<1>(&mut pc),
            2 => code.get_push_data::<2>(&mut pc),
            32 => code.get_push_data::<32>(&mut pc),
            _ => unreachable!(),
        };
        assert_eq!(data, expected);
        assert_eq!(pc.code_offset(), code_offset);
    }

    #[rstest]
    #[case::opcode(&[ADD], 0, 0)]
    #[cfg_attr(
        not(feature = "fn-ptr-conversion-dispatch"),
        case::push_data_byte(&[PUSH1, 0xff], 1, 1)
    )]
    #[cfg_attr(
        feature = "fn-ptr-conversion-dispatch",
        case::push_data_maps_to_the_push(&[PUSH1, 0xff], 1, 0)
    )]
    #[case::after_push_data(&[PUSH1, 0xff, ADD], 2, 2)]
    #[case::end_of_code(&[ADD], 1, 1)]
    #[cfg_attr(
        not(feature = "fn-ptr-conversion-dispatch"),
        case::past_the_end(&[ADD], 10, 10)
    )]
    #[cfg_attr(
        feature = "fn-ptr-conversion-dispatch",
        case::past_the_end_maps_to_the_terminator(&[ADD], 10, 1)
    )]
    fn pc_new_points_at_the_code_offset(
        #[case] bytes: &[u8],
        #[case] offset: usize,
        #[case] expected: usize,
    ) {
        let cache = CodeAnalysisCache::default();
        let analysis = CodeAnalysis::<false>::new(bytes, None, &cache);
        assert_eq!(Pc::new(&analysis, offset).code_offset(), expected);
    }

    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    #[rstest]
    #[case::opcode(&[ADD], 0, Opcode::Add)]
    #[case::push(&[PUSH1, 0xff], 0, Opcode::Push1)]
    #[case::jump_dest(&[DEST], 0, Opcode::JumpDest)]
    #[case::invalid(&[0xc0], 0, Opcode::Invalid)]
    #[case::terminator(&[ADD], 1, Opcode::Stop)]
    fn pc_get_returns_the_handler_of_the_entry(
        #[case] bytes: &[u8],
        #[case] offset: usize,
        #[case] expected: Opcode,
    ) {
        let cache = CodeAnalysisCache::default();
        let analysis = CodeAnalysis::new(bytes, None, &cache);
        assert!(std::ptr::fn_addr_eq(
            Pc::<false>::new(&analysis, offset).get(),
            crate::interpreter::get_jumptable::<false>()[expected as usize]
        ));
    }

    #[rstest]
    #[case::opcode(&[ADD, ADD], 1)]
    #[cfg_attr(
        not(feature = "fn-ptr-conversion-dispatch"),
        case::push(&[PUSH1, 0xff, ADD], 1)
    )]
    #[cfg_attr(
        feature = "fn-ptr-conversion-dispatch",
        case::push_skips_its_data(&[PUSH1, 0xff, ADD], 2)
    )]
    fn pc_next_advances_to_the_next_entry(#[case] bytes: &[u8], #[case] expected: usize) {
        let cache = CodeAnalysisCache::default();
        let analysis = CodeAnalysis::<false>::new(bytes, None, &cache);
        let mut pc = Pc::new(&analysis, 0);
        pc.next();
        assert_eq!(pc.code_offset(), expected);
    }

    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    #[rstest]
    #[case::full(&[Opcode::Push2 as u8, 1, 2, ADD], 0x0102u32.into())]
    #[case::thirty_two_bytes(&[[Opcode::Push32 as u8].as_slice(), &[0xff; 32]].concat(), u256::MAX)]
    #[case::past_end_is_zero_padded(&[Opcode::Push2 as u8, 1], 0x0100u32.into())]
    fn pc_get_push_data_returns_the_data_of_the_entry(
        #[case] bytes: &[u8],
        #[case] expected: u256,
    ) {
        let cache = CodeAnalysisCache::default();
        let analysis = CodeAnalysis::new(bytes, None, &cache);
        assert_eq!(Pc::<false>::new(&analysis, 0).get_push_data(), expected);
    }

    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    #[test]
    fn pc_jump_to_jumpdest_skips_the_no_ops() {
        let bytes = [Opcode::Push3 as u8, 0xff, 0xff, 0xff, DEST, ADD];
        // Analysis entries: Push3, SkipNoOps(3), NoOp, NoOp, JumpDest, Add, Stop
        let cache = CodeAnalysisCache::default();
        let analysis = CodeAnalysis::new(&bytes, None, &cache);
        let mut pc = Pc::<false>::new(&analysis, 0);
        pc.next();
        pc.jump_to_jumpdest();
        // The no-ops report the code offset of the jump dest too, so only the handler tells them
        // apart.
        assert!(std::ptr::fn_addr_eq(
            pc.get(),
            crate::interpreter::get_jumptable::<false>()[DEST as usize]
        ));
        pc.next();
        assert_eq!(pc.code_offset(), 5);
    }

    #[rstest]
    #[cfg_attr(
        not(feature = "fn-ptr-conversion-dispatch"),
        case::one_entry_per_byte(&[PUSH1, 0xff, ADD], &[0, 1, 2, 3])
    )]
    #[cfg_attr(
        feature = "fn-ptr-conversion-dispatch",
        case::push_data_has_no_entry(&[PUSH1, 0xff, ADD], &[0, 2, 3])
    )]
    #[cfg_attr(
        feature = "fn-ptr-conversion-dispatch",
        case::padding_reports_the_jump_dest(&[PUSH1, 0xff, DEST], &[0, 2, 2, 3])
    )]
    #[cfg_attr(
        feature = "fn-ptr-conversion-dispatch",
        case::terminator_after_zero_padded_push_data(&[Opcode::Push2 as u8, 1], &[0, 3])
    )]
    fn pc_code_offset_returns_the_code_offset_of_each_entry(
        #[case] bytes: &[u8],
        #[case] expected: &[usize],
    ) {
        let cache = CodeAnalysisCache::default();
        let analysis = CodeAnalysis::<false>::new(bytes, None, &cache);
        let mut pc = Pc::new(&analysis, 0);
        let mut offsets = vec![pc.code_offset()];
        for _ in 1..expected.len() {
            pc.next();
            offsets.push(pc.code_offset());
        }
        assert_eq!(offsets, expected);
    }
}
