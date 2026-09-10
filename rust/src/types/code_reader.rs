#[cfg(feature = "fn-ptr-conversion-dispatch")]
use std::ptr::NonNull;
use std::{self, marker::PhantomData, ops::Deref};

use crate::types::{CodeAnalysis, CodeAnalysisCache, CodeByteType, FailStatus, u256};
#[cfg(feature = "fn-ptr-conversion-dispatch")]
use crate::{interpreter::OpFn, types::OpFnData};

/// The code a run executes together with its analysis. Both are fixed for the whole run, so they
/// stay in memory while a [`Pc`] moves over them.
#[derive(Debug)]
pub struct Code<'a, const STEPPABLE: bool> {
    code: &'a [u8],
    analysis: CodeAnalysis<STEPPABLE>,
}

impl<const STEPPABLE: bool> Deref for Code<'_, STEPPABLE> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.code
    }
}

#[cfg(not(feature = "fn-ptr-conversion-dispatch"))]
#[derive(Debug, PartialEq, Eq)]
pub enum GetOpcodeError {
    OutOfRange,
    Invalid,
}

impl<'a, const STEPPABLE: bool> Code<'a, STEPPABLE> {
    pub fn new(
        code: &'a [u8],
        code_hash: Option<u256>,
        cache: &CodeAnalysisCache<STEPPABLE>,
    ) -> Self {
        Self {
            code,
            analysis: CodeAnalysis::new(code, code_hash, cache),
        }
    }

    /// The program counter for the instruction at `code_offset`.
    pub fn pc(&self, code_offset: usize) -> Pc<'_, STEPPABLE> {
        std::cfg_select! {
            feature = "fn-ptr-conversion-dispatch" => {
                Pc::at(self, self.analysis.analysis_offset(code_offset))
            }
            _ => Pc::at(self, code_offset),
        }
    }

    #[cfg(not(feature = "fn-ptr-conversion-dispatch"))]
    pub fn get_at(&self, pc: Pc<STEPPABLE>) -> Result<u8, GetOpcodeError> {
        if let Some(op) = self.code.get(pc.offset) {
            let analysis = self.analysis[pc.offset];
            if analysis == CodeByteType::DataOrInvalid {
                Err(GetOpcodeError::Invalid)
            } else {
                Ok(*op)
            }
        } else {
            Err(GetOpcodeError::OutOfRange)
        }
    }

    /// Moves `pc` to the jump destination `dest`, which has to be a jump destination in this code.
    pub fn try_jump<'c>(
        &'c self,
        pc: &mut Pc<'c, STEPPABLE>,
        dest: u256,
    ) -> Result<(), FailStatus> {
        let dest = u64::try_from(dest).map_err(|_| FailStatus::BadJumpDestination)? as usize;

        let Some(analysis_item) = self.analysis.get(dest) else {
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
        *pc = Pc::at(self, dest);

        Ok(())
    }

    #[cfg(not(feature = "fn-ptr-conversion-dispatch"))]
    pub fn get_push_data<const N: usize>(&self, pc: &mut Pc<STEPPABLE>) -> u256 {
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

/// The program counter of a run. It borrows the [`Code`] it was obtained from and is one word, so
/// the handler chain passes it by value and it stays in a register instead of being loaded from
/// and stored back into memory on every dispatch.
#[derive(Clone, Copy, Debug)]
pub struct Pc<'c, const STEPPABLE: bool> {
    /// The current entry of the analysis. A pointer instead of an index avoids recomputing the
    /// entry address on every dispatch. It always points at a valid entry: execution cannot
    /// advance past the terminator entry and jumps are bounds checked.
    ///
    /// It points into the heap buffer of the analysis, which the borrowed code keeps alive for as
    /// long as the counter exists and which is never mutated: it is only reachable through shared
    /// references.
    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    entry: NonNull<OpFnData<STEPPABLE>>,
    /// The offset into the code, which [`Code`] bounds checks on every access.
    #[cfg(not(feature = "fn-ptr-conversion-dispatch"))]
    offset: usize,
    _code: PhantomData<&'c Code<'c, STEPPABLE>>,
}

impl<'c, const STEPPABLE: bool> Pc<'c, STEPPABLE> {
    /// The counter for the entry at `offset` in the analysis of `code`.
    #[allow(unused_variables)]
    fn at(code: &'c Code<'c, STEPPABLE>, offset: usize) -> Self {
        Self {
            // The pointer is derived from the part of the analysis behind `offset`, so it covers
            // every entry the counter can be advanced to.
            #[cfg(feature = "fn-ptr-conversion-dispatch")]
            entry: NonNull::from(&code.analysis[offset..]).cast(),
            #[cfg(not(feature = "fn-ptr-conversion-dispatch"))]
            offset,
            _code: PhantomData,
        }
    }

    /// The handler of the current entry. The analysis ends with a terminator entry that stops
    /// execution and the counter can never advance past it, so there is always an entry to dispatch
    /// to. Invalid opcodes hold the handler for [crate::types::Opcode::Invalid], hence no error
    /// handling is needed either.
    // TODO: technically this method is not safe, because the invariant it relies on can be broken
    // by calling only safe public methods (calling advance() until the counter is out of bounds).
    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    pub fn op_fn(self) -> OpFn<STEPPABLE> {
        // SAFETY:
        // The counter always points at a valid entry (see field documentation).
        unsafe { self.entry.as_ref() }.get_func()
    }

    /// Advances the counter to the following instruction.
    // TODO: technically speaking, this method is not safe because it can break the invariant that
    // the counter always points to a valid analysis item.
    pub fn advance(&mut self) {
        std::cfg_select! {
            feature = "fn-ptr-conversion-dispatch" => {
                // SAFETY:
                // advance is only called for entries that do not stop execution, and every such
                // entry is followed by another entry because the analysis ends with a terminator.
                self.entry = unsafe { self.entry.add(1) };
            }
            _ => {
                self.offset += 1;
            }
        }
    }

    /// The push data of the current entry, advancing to the following entry.
    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    pub fn get_push_data(&mut self) -> u256 {
        // SAFETY:
        // The counter always points at a valid entry (see field documentation).
        let data = unsafe { self.entry.as_ref() }.get_data();
        // SAFETY:
        // A push entry is never the last entry because the analysis ends with a terminator.
        self.entry = unsafe { self.entry.add(1) };
        data
    }

    /// Advances to the jump destination entry behind a run of no-ops, whose distance the current
    /// entry holds.
    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    pub fn skip_no_ops(&mut self) {
        // SAFETY:
        // The counter always points at a valid entry (see field documentation).
        let offset = unsafe { self.entry.as_ref() }
            .get_data()
            .into_u64_saturating();
        // SAFETY:
        // A skip-no-ops entry holds the distance to the following jump dest entry, which is in
        // bounds.
        self.entry = unsafe { self.entry.add(offset as usize) };
    }

    /// The code offset the counter refers to, which is what the steppable interface reports.
    pub fn code_offset(self) -> usize {
        std::cfg_select! {
            feature = "fn-ptr-conversion-dispatch" => {
                // SAFETY:
                // The counter always points at a valid entry (see field documentation).
                unsafe { self.entry.as_ref() }.get_code_offset()
            }
            _ => self.offset,
        }
    }
}

#[cfg(test)]
mod tests {
    #[cfg(not(feature = "fn-ptr-conversion-dispatch"))]
    use crate::types::code_reader::GetOpcodeError;
    use crate::types::{CodeAnalysisCache, FailStatus, Opcode, code_reader::Code, u256};

    #[test]
    fn code_internals() {
        let code_analysis_cache = CodeAnalysisCache::default();
        let bytes = [Opcode::Add as u8, Opcode::Add as u8, 0xc0];
        let code = Code::<false>::new(&bytes, None, &code_analysis_cache);
        assert_eq!(*code, bytes);
        assert_eq!(code.len(), bytes.len());
        assert_eq!(code.pc(1).code_offset(), 1);
    }

    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    #[test]
    fn pc() {
        let code_analysis_cache = CodeAnalysisCache::default();

        let code = [Opcode::Push1 as u8, Opcode::Add as u8, Opcode::Add as u8];
        let code = Code::<false>::new(&code, None, &code_analysis_cache);

        assert_eq!(code.pc(0).code_offset(), 0);

        let mut pc = code.pc(0);
        pc.get_push_data();
        assert_eq!(pc.code_offset(), 2);

        assert_eq!(code.pc(2).code_offset(), 2);

        let mut code = [Opcode::Add as u8; 23];
        code[0] = Opcode::Push21 as u8;
        let code = Code::<false>::new(&code, None, &code_analysis_cache);

        assert_eq!(code.pc(0).code_offset(), 0);

        let mut pc = code.pc(0);
        pc.get_push_data();
        assert_eq!(pc.code_offset(), 22);

        assert_eq!(code.pc(22).code_offset(), 22);
    }

    #[cfg(not(feature = "fn-ptr-conversion-dispatch"))]
    #[test]
    fn get_at() {
        let code_analysis_cache = CodeAnalysisCache::default();
        let code = Code::<false>::new(
            &[Opcode::Add as u8, Opcode::Add as u8, 0xc0],
            None,
            &code_analysis_cache,
        );
        let mut pc = code.pc(0);
        assert_eq!(code.get_at(pc), Ok(Opcode::Add as u8));
        pc.advance();
        assert_eq!(code.get_at(pc), Ok(Opcode::Add as u8));
        pc.advance();
        assert_eq!(code.get_at(pc), Err(GetOpcodeError::Invalid));
        pc.advance();
        assert_eq!(code.get_at(pc), Err(GetOpcodeError::OutOfRange));
    }

    #[test]
    fn try_jump() {
        let code_analysis_cache = CodeAnalysisCache::default();
        let code = Code::<false>::new(
            &[
                Opcode::Push1 as u8,
                Opcode::JumpDest as u8,
                Opcode::JumpDest as u8,
            ],
            None,
            &code_analysis_cache,
        );
        let mut pc = code.pc(0);
        assert_eq!(
            code.try_jump(&mut pc, 1u8.into()),
            Err(FailStatus::BadJumpDestination)
        );
        assert_eq!(code.try_jump(&mut pc, 2u8.into()), Ok(()));
        assert_eq!(pc.code_offset(), 2);
        assert_eq!(
            code.try_jump(&mut pc, 3u8.into()),
            Err(FailStatus::BadJumpDestination)
        );
        assert_eq!(
            code.try_jump(&mut pc, u256::MAX),
            Err(FailStatus::BadJumpDestination)
        );
    }

    #[cfg(not(feature = "fn-ptr-conversion-dispatch"))]
    #[test]
    fn get_push_data() {
        let code_analysis_cache = CodeAnalysisCache::default();
        let code = Code::<false>::new(&[0xff; 32], None, &code_analysis_cache);

        assert_eq!(code.get_push_data::<1>(&mut code.pc(0)), 0xffu8.into());
        assert_eq!(code.get_push_data::<32>(&mut code.pc(0)), u256::MAX);
        assert_eq!(
            code.get_push_data::<32>(&mut code.pc(31)),
            u256::from(0xffu8) << u256::from(248u8)
        );
        assert_eq!(code.get_push_data::<32>(&mut code.pc(32)), u256::ZERO);
    }

    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    #[test]
    fn get_push_data() {
        let code_analysis_cache = CodeAnalysisCache::default();
        // pc on data is non longer possible because there are not data items anymore
        let mut code = [0xff; 33];
        code[0] = Opcode::Push32 as u8;
        let code = Code::<false>::new(&code, None, &code_analysis_cache);
        assert_eq!(code.pc(0).get_push_data(), u256::MAX);
    }

    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    #[test]
    fn op_fn() {
        let jumptable = crate::interpreter::get_jumptable::<false>();
        let code_analysis_cache = CodeAnalysisCache::default();
        let code = Code::<false>::new(
            &[Opcode::Add as u8, Opcode::Add as u8, 0xc0],
            None,
            &code_analysis_cache,
        );
        let mut pc = code.pc(0);
        assert!(std::ptr::fn_addr_eq(
            pc.op_fn(),
            jumptable[Opcode::Add as u8 as usize]
        ));
        pc.advance();
        assert!(std::ptr::fn_addr_eq(
            pc.op_fn(),
            jumptable[Opcode::Add as u8 as usize]
        ));
        pc.advance();
        assert!(std::ptr::fn_addr_eq(
            pc.op_fn(),
            jumptable[Opcode::Invalid as u8 as usize]
        ));
        pc.advance();
        assert!(std::ptr::fn_addr_eq(
            pc.op_fn(),
            jumptable[Opcode::Stop as u8 as usize]
        ));
    }
}
