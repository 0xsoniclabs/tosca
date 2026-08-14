use std::{self, ops::Deref};

use crate::types::{CodeAnalysis, CodeAnalysisCache, CodeByteType, FailStatus, u256};
#[cfg(feature = "fn-ptr-conversion-dispatch")]
use crate::{interpreter::OpFn, types::OpFnData};

#[derive(Debug)]
pub struct CodeReader<'a, const STEPPABLE: bool> {
    code: &'a [u8],
    code_analysis: CodeAnalysis<STEPPABLE>,
    #[cfg(not(feature = "fn-ptr-conversion-dispatch"))]
    pc: usize,
    /// Pointer to the current entry in `code_analysis`. Storing a pointer instead of an index
    /// avoids recomputing the entry address on every dispatch. It always points at a valid entry:
    /// execution cannot advance past the terminator entry and jumps are bounds checked.
    ///
    /// It points into the heap buffer of `code_analysis`, not into this struct, so moving the
    /// reader does not invalidate it and no pinning is needed. It stays valid because
    /// `code_analysis` keeps that buffer alive for as long as the reader exists and because the
    /// buffer is never mutated: it is only reachable through shared references.
    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    pc: *const OpFnData<STEPPABLE>,
}

impl<const STEPPABLE: bool> Deref for CodeReader<'_, STEPPABLE> {
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

impl<'a, const STEPPABLE: bool> CodeReader<'a, STEPPABLE> {
    pub fn new(
        code: &'a [u8],
        code_hash: Option<u256>,
        pc: usize,
        cache: &CodeAnalysisCache<STEPPABLE>,
    ) -> Self {
        let code_analysis = CodeAnalysis::new(code, code_hash, cache);
        #[cfg(feature = "fn-ptr-conversion-dispatch")]
        let pc = {
            let analysis_offset = code_analysis.analysis_offset(pc);
            // SAFETY:
            // analysis_offset only returns offsets of existing analysis entries, so it is in
            // bounds.
            unsafe { code_analysis.as_ptr().add(analysis_offset) }
        };
        Self {
            code,
            code_analysis,
            pc,
        }
    }

    #[cfg(not(feature = "fn-ptr-conversion-dispatch"))]
    pub fn get(&self) -> Result<u8, GetOpcodeError> {
        if let Some(op) = self.code.get(self.pc) {
            let analysis = self.code_analysis[self.pc];
            if analysis == CodeByteType::DataOrInvalid {
                Err(GetOpcodeError::Invalid)
            } else {
                Ok(*op)
            }
        } else {
            Err(GetOpcodeError::OutOfRange)
        }
    }
    /// The analysis ends with a terminator entry that stops execution and the program counter can
    /// never advance past it, so there is always an entry to dispatch to. Invalid opcodes hold the
    /// handler for [crate::types::Opcode::Invalid], hence no error handling is needed either.
    // TODO: technically this method is not safe, because the invariant it relies on can be broken
    // by calling only safe public methods (calling next() until the pc is out of bounds).
    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    pub fn get(&self) -> OpFn<STEPPABLE> {
        // SAFETY:
        // self.pc always points at a valid entry (see field documentation).
        unsafe { (*self.pc).get_func() }
    }

    // TODO: technically speaking, this method is not safe because it can break the invariant that
    // the pc always points to a valid analysis item.
    pub fn next(&mut self) {
        std::cfg_select! {
            feature = "fn-ptr-conversion-dispatch" => {
                // SAFETY:
                // next is only called for entries that do not stop execution, and every such entry
                // is followed by another entry because the analysis ends with a terminator.
                self.pc = unsafe { self.pc.add(1) };
            }
            _ => {
                self.pc += 1;
            }
        }
    }

    pub fn try_jump(&mut self, dest: u256) -> Result<(), FailStatus> {
        let dest = u64::try_from(dest).map_err(|_| FailStatus::BadJumpDestination)? as usize;
        if !self.code_analysis.get(dest).is_some_and(|c| {
            let code_byte_type = std::cfg_select! {
                feature = "fn-ptr-conversion-dispatch" => c.code_byte_type(),
                _ => *c,
            };
            code_byte_type == CodeByteType::JumpDest
        }) {
            std::hint::cold_path();
            return Err(FailStatus::BadJumpDestination);
        }
        std::cfg_select! {
            feature = "fn-ptr-conversion-dispatch" => {
                // SAFETY:
                // The check above ensures that dest is in bounds.
                self.pc = unsafe { self.code_analysis.as_ptr().add(dest) };
            }
            _ => self.pc = dest,
        }

        Ok(())
    }

    #[cfg(not(feature = "fn-ptr-conversion-dispatch"))]
    pub fn get_push_data<const N: usize>(&mut self) -> u256 {
        const { assert!(N > 0 && N <= 32) };

        // N is known at compile time, so copying a whole window of push data compiles to a fixed
        // size copy. Only a push running past the end of the code needs a runtime length copy,
        // which is a call to memcpy.
        let mut data = [0; 32];
        if let Some(window) = self.code.get(self.pc..self.pc + N) {
            data[32 - N..].copy_from_slice(window);
        } else {
            let data_len = self.code.len() - self.pc;
            data[32 - N..32 - N + data_len].copy_from_slice(&self.code[self.pc..]);
        }
        let data = u256::from_be_bytes(data);
        self.pc += N;

        data
    }
    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    pub fn get_push_data(&mut self) -> u256 {
        // SAFETY:
        // self.pc always points at a valid entry (see field documentation).
        let res = unsafe { (*self.pc).get_data() };
        // SAFETY:
        // A push entry is never the last entry because the analysis ends with a terminator.
        self.pc = unsafe { self.pc.add(1) };
        res
    }

    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    pub fn jump_to(&mut self) {
        // SAFETY:
        // self.pc always points at a valid entry (see field documentation).
        let offset = unsafe { (*self.pc).get_data() }.into_u64_saturating();
        // SAFETY:
        // A skip-no-ops entry holds the distance to the following jump dest entry, which is in
        // bounds.
        self.pc = unsafe { self.pc.add(offset as usize) };
    }

    pub fn pc(&self) -> usize {
        std::cfg_select! {
            feature = "fn-ptr-conversion-dispatch" => {
                // SAFETY:
                // self.pc always points at a valid entry (see field documentation).
                unsafe { (*self.pc).get_code_offset() }
            }
            _ => self.pc,
        }
    }
}

#[cfg(test)]
mod tests {
    #[cfg(not(feature = "fn-ptr-conversion-dispatch"))]
    use crate::types::code_reader::GetOpcodeError;
    use crate::types::{CodeAnalysisCache, FailStatus, Opcode, code_reader::CodeReader, u256};

    #[test]
    fn code_reader_internals() {
        let code_analysis_cache = CodeAnalysisCache::default();
        let code = [Opcode::Add as u8, Opcode::Add as u8, 0xc0];
        let pc = 1;
        let code_reader = CodeReader::<false>::new(&code, None, pc, &code_analysis_cache);
        assert_eq!(*code_reader, code);
        assert_eq!(code_reader.len(), code.len());
        assert_eq!(code_reader.pc(), pc);
    }

    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    #[test]
    fn code_reader_pc() {
        let code_analysis_cache = CodeAnalysisCache::default();

        let code = [Opcode::Push1 as u8, Opcode::Add as u8, Opcode::Add as u8];

        let code_reader = CodeReader::<false>::new(&code, None, 0, &code_analysis_cache);
        assert_eq!(code_reader.pc(), 0);

        let mut code_reader = CodeReader::<false>::new(&code, None, 0, &code_analysis_cache);
        code_reader.get_push_data();
        assert_eq!(code_reader.pc(), 2);

        let code_reader = CodeReader::<false>::new(&code, None, 2, &code_analysis_cache);
        assert_eq!(code_reader.pc(), 2);

        let mut code = [Opcode::Add as u8; 23];
        code[0] = Opcode::Push21 as u8;

        let code_reader = CodeReader::<false>::new(&code, None, 0, &code_analysis_cache);
        assert_eq!(code_reader.pc(), 0);

        let mut code_reader = CodeReader::<false>::new(&code, None, 0, &code_analysis_cache);
        code_reader.get_push_data();
        assert_eq!(code_reader.pc(), 22);

        let code_reader = CodeReader::<false>::new(&code, None, 22, &code_analysis_cache);
        assert_eq!(code_reader.pc(), 22);
    }

    #[cfg(not(feature = "fn-ptr-conversion-dispatch"))]
    #[test]
    fn code_reader_get() {
        let code_analysis_cache = CodeAnalysisCache::default();
        let mut code_reader = CodeReader::<false>::new(
            &[Opcode::Add as u8, Opcode::Add as u8, 0xc0],
            None,
            0,
            &code_analysis_cache,
        );
        assert_eq!(code_reader.get(), Ok(Opcode::Add as u8));
        code_reader.next();
        assert_eq!(code_reader.get(), Ok(Opcode::Add as u8));
        code_reader.next();
        assert_eq!(code_reader.get(), Err(GetOpcodeError::Invalid));
        code_reader.next();
        assert_eq!(code_reader.get(), Err(GetOpcodeError::OutOfRange));
    }

    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    #[test]
    fn code_reader_get() {
        let jumptable = crate::interpreter::get_jumptable::<false>();
        let code_analysis_cache = CodeAnalysisCache::default();
        let mut code_reader = CodeReader::<false>::new(
            &[Opcode::Add as u8, Opcode::Add as u8, 0xc0],
            None,
            0,
            &code_analysis_cache,
        );
        assert!(std::ptr::fn_addr_eq(
            code_reader.get(),
            jumptable[Opcode::Add as u8 as usize]
        ));
        code_reader.next();
        assert!(std::ptr::fn_addr_eq(
            code_reader.get(),
            jumptable[Opcode::Add as u8 as usize]
        ));
        code_reader.next();
        assert!(std::ptr::fn_addr_eq(
            code_reader.get(),
            jumptable[Opcode::Invalid as u8 as usize]
        ));
        code_reader.next();
        assert!(std::ptr::fn_addr_eq(
            code_reader.get(),
            jumptable[Opcode::Stop as u8 as usize]
        ));
    }

    #[test]
    fn code_reader_try_jump() {
        let code_analysis_cache = CodeAnalysisCache::default();
        let mut code_reader = CodeReader::<false>::new(
            &[
                Opcode::Push1 as u8,
                Opcode::JumpDest as u8,
                Opcode::JumpDest as u8,
            ],
            None,
            0,
            &code_analysis_cache,
        );
        assert_eq!(
            code_reader.try_jump(1u8.into()),
            Err(FailStatus::BadJumpDestination)
        );
        assert_eq!(code_reader.try_jump(2u8.into()), Ok(()));
        assert_eq!(
            code_reader.try_jump(3u8.into()),
            Err(FailStatus::BadJumpDestination)
        );
        assert_eq!(
            code_reader.try_jump(u256::MAX),
            Err(FailStatus::BadJumpDestination)
        );
    }

    #[cfg(not(feature = "fn-ptr-conversion-dispatch"))]
    #[test]
    fn code_reader_get_push_data() {
        let code_analysis_cache = CodeAnalysisCache::default();
        let mut code_reader = CodeReader::<false>::new(&[0xff; 32], None, 0, &code_analysis_cache);
        assert_eq!(code_reader.get_push_data::<1>(), 0xffu8.into());

        let mut code_reader = CodeReader::<false>::new(&[0xff; 32], None, 0, &code_analysis_cache);
        assert_eq!(code_reader.get_push_data::<32>(), u256::MAX);

        let mut code_reader = CodeReader::<false>::new(&[0xff; 32], None, 31, &code_analysis_cache);
        assert_eq!(
            code_reader.get_push_data::<32>(),
            u256::from(0xffu8) << u256::from(248u8)
        );

        let mut code_reader = CodeReader::<false>::new(&[0xff; 32], None, 32, &code_analysis_cache);
        assert_eq!(code_reader.get_push_data::<32>(), u256::ZERO);
    }
    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    #[test]
    fn code_reader_get_push_data() {
        let code_analysis_cache = CodeAnalysisCache::default();
        // pc on data is non longer possible because there are not data items anymore
        let mut code = [0xff; 33];
        code[0] = Opcode::Push32 as u8;
        let mut code_reader = CodeReader::<false>::new(&code, None, 0, &code_analysis_cache);
        assert_eq!(code_reader.get_push_data(), u256::MAX);
    }
}
