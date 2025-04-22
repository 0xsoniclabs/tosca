#[cfg(not(feature = "fn-ptr-conversion-dispatch"))]
use std::cmp::min;
use std::{self, ops::Deref};

#[cfg(feature = "fn-ptr-conversion-dispatch")]
use crate::interpreter::OpFn;
use crate::types::{
    AnalysisContainer, CodeAnalysis, CodeAnalysisCache, CodeByteType, FailStatus, u256,
};

#[cfg(not(feature = "fn-ptr-conversion-dispatch"))]
struct PushDataLen<const N: usize>;

#[cfg(not(feature = "fn-ptr-conversion-dispatch"))]
impl<const N: usize> PushDataLen<N> {
    const VALID: () = assert!(N > 0 && N <= 32);
}

#[derive(Debug)]
pub struct CodeReader<'a, const STEPPABLE: bool> {
    code: &'a [u8],
    code_analysis: AnalysisContainer<CodeAnalysis<STEPPABLE>>,
    pc: usize,
}

impl<const STEPPABLE: bool> Deref for CodeReader<'_, STEPPABLE> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.code
    }
}

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
        let pc = code_analysis.pc_map.to_converted(pc);
        Self {
            code,
            code_analysis,
            pc,
        }
    }

    #[cfg(not(feature = "fn-ptr-conversion-dispatch"))]
    pub fn get(&self) -> Result<u8, GetOpcodeError> {
        if let Some(op) = self.code.get(self.pc) {
            let analysis = self.code_analysis.analysis[self.pc];
            if analysis == CodeByteType::DataOrInvalid {
                Err(GetOpcodeError::Invalid)
            } else {
                Ok(*op)
            }
        } else {
            Err(GetOpcodeError::OutOfRange)
        }
    }
    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    pub fn get(&self) -> Result<OpFn<STEPPABLE>, GetOpcodeError> {
        self.code_analysis
            .analysis
            .get(self.pc)
            .ok_or(GetOpcodeError::OutOfRange)
            .and_then(|analysis| analysis.get_func().ok_or(GetOpcodeError::Invalid))
    }

    pub fn next(&mut self) {
        self.pc += 1;
    }

    pub fn try_jump(&mut self, dest: u256) -> Result<(), FailStatus> {
        let dest = u64::try_from(dest).map_err(|_| FailStatus::BadJumpDestination)? as usize;
        if !self.code_analysis.analysis.get(dest).is_some_and(|c| {
            #[cfg(not(feature = "fn-ptr-conversion-dispatch"))]
            return *c == CodeByteType::JumpDest;

            #[cfg(feature = "fn-ptr-conversion-dispatch")]
            return c.code_byte_type() == CodeByteType::JumpDest;
        }) {
            return Err(FailStatus::BadJumpDestination);
        }
        self.pc = dest;

        Ok(())
    }

    #[cfg(not(feature = "fn-ptr-conversion-dispatch"))]
    pub fn get_push_data<const N: usize>(&mut self) -> u256 {
        let () = const { PushDataLen::<N>::VALID };

        let data_len = min(N, self.code.len().saturating_sub(self.pc));
        let mut data = [0; 32];
        data[32 - N..32 - N + data_len].copy_from_slice(&self.code[self.pc..self.pc + data_len]);
        let data = u256::from_be_bytes(data);
        self.pc += N;

        data
    }
    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    pub fn get_push_data(&mut self) -> u256 {
        // SAFETY:
        // This assertion assumes that the program counter (self.pc) was not modified after calling
        // [`CodeReader::get`]. While this can not be guaranteed here, marking the function
        // as unsafe would propagate all the way to the function dispatch function and also require
        // that all opcode functions are unsafe. In practice, the only way to modify the
        // program counter, is through one of the functions of CodeReader that take it by mutable
        // reference. Those are next, try_jump, jump_to and get_push_data itself.
        // Calling those and then calling get_push_data makes semantically no sense.
        #[cfg(feature = "unsafe-hints")]
        unsafe {
            std::hint::assert_unchecked(self.pc < self.code_analysis.analysis.len());
        }
        let res = self.code_analysis.analysis[self.pc].get_data();
        self.pc += 1;
        res
    }

    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    pub fn jump_to(&mut self) {
        #[cfg(feature = "fn-ptr-conversion-dispatch")]
        let offset = self.code_analysis.analysis[self.pc]
            .get_data()
            .into_u64_saturating();
        #[cfg(not(feature = "fn-ptr-conversion-dispatch"))]
        let offset = u32::from_ne_bytes(self.code_analysis.analysis[self.pc].get_data());
        self.pc += offset as usize;
    }

    pub fn pc(&self) -> usize {
        #[cfg(not(feature = "fn-ptr-conversion-dispatch"))]
        return self.pc;
        #[cfg(feature = "fn-ptr-conversion-dispatch")]
        return self.code_analysis.pc_map.to_ct(self.pc);
    }
}

#[cfg(test)]
mod tests {
    use crate::types::{
        CodeAnalysisCache, FailStatus, Opcode,
        code_reader::{CodeReader, GetOpcodeError},
        u256,
    };

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
        assert_eq!(code_reader.pc, 0);
        assert_eq!(code_reader.pc(), 0);

        let mut code_reader = CodeReader::<false>::new(&code, None, 0, &code_analysis_cache);
        assert_eq!(code_reader.pc, 0);
        code_reader.get_push_data();
        assert_eq!(code_reader.pc, 1);
        assert_eq!(code_reader.pc(), 2);

        let code_reader = CodeReader::<false>::new(&code, None, 2, &code_analysis_cache);
        assert_eq!(code_reader.pc, 1);
        assert_eq!(code_reader.pc(), 2);

        let mut code = [Opcode::Add as u8; 23];
        code[0] = Opcode::Push21 as u8;

        let code_reader = CodeReader::<false>::new(&code, None, 0, &code_analysis_cache);
        assert_eq!(code_reader.pc, 0);
        assert_eq!(code_reader.pc(), 0);

        let mut code_reader = CodeReader::<false>::new(&code, None, 0, &code_analysis_cache);
        assert_eq!(code_reader.pc, 0);
        code_reader.get_push_data();
        assert_eq!(code_reader.pc, 1);
        assert_eq!(code_reader.pc(), 22);

        let code_reader = CodeReader::<false>::new(&code, None, 22, &code_analysis_cache);
        assert_eq!(code_reader.pc, 1);
        assert_eq!(code_reader.pc(), 22);
    }

    #[test]
    fn code_reader_get() {
        let code_analysis_cache = CodeAnalysisCache::default();
        let mut code_reader = CodeReader::<false>::new(
            &[Opcode::Add as u8, Opcode::Add as u8, 0xc0],
            None,
            0,
            &code_analysis_cache,
        );
        #[cfg(not(feature = "fn-ptr-conversion-dispatch"))]
        assert_eq!(code_reader.get(), Ok(Opcode::Add as u8));
        #[cfg(feature = "fn-ptr-conversion-dispatch")]
        assert!(code_reader.get().is_ok(),);
        code_reader.next();
        #[cfg(not(feature = "fn-ptr-conversion-dispatch"))]
        assert_eq!(code_reader.get(), Ok(Opcode::Add as u8));
        #[cfg(feature = "fn-ptr-conversion-dispatch")]
        assert!(code_reader.get().is_ok(),);
        code_reader.next();
        assert_eq!(code_reader.get(), Err(GetOpcodeError::Invalid));
        code_reader.next();
        assert_eq!(code_reader.get(), Err(GetOpcodeError::OutOfRange));
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
