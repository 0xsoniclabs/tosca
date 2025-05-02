#[cfg(feature = "fn-ptr-conversion-dispatch")]
use std::cmp::min;
use std::ops::Deref;
#[cfg(feature = "code-analysis-cache")]
use std::sync::Arc;

#[cfg(feature = "code-analysis-cache")]
use nohash_hasher::BuildNoHashHasher;

#[cfg(feature = "code-analysis-cache")]
use crate::types::Cache;
#[cfg(feature = "fn-ptr-conversion-dispatch")]
use crate::types::OpFnData;
use crate::types::{CodeByteType, code_byte_type, u256};

/// This type represents a hash value in form of a u256.
/// Because it is already a hash value there is no need to hash it again when implementing Hash.
#[cfg(feature = "code-analysis-cache")]
#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq, Eq)]
struct u256Hash(u256);

#[cfg(feature = "code-analysis-cache")]
impl std::hash::Hash for u256Hash {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u64(self.0.into_u64_with_overflow().0);
    }
}

#[cfg(not(feature = "fn-ptr-conversion-dispatch"))]
pub type AnalysisItem<const STEPPABLE: bool, const TAILCALL: bool> = CodeByteType;
#[cfg(feature = "fn-ptr-conversion-dispatch")]
pub type AnalysisItem<const STEPPABLE: bool, const TAILCALL: bool> = OpFnData<STEPPABLE, TAILCALL>;

pub struct CodeAnalysisCache<const STEPPABLE: bool, const TAILCALL: bool>(
    #[cfg(feature = "code-analysis-cache")]
    Cache<u256Hash, CodeAnalysis<STEPPABLE, TAILCALL>, BuildNoHashHasher<u64>>,
);

impl<const STEPPABLE: bool, const TAILCALL: bool> Default
    for CodeAnalysisCache<STEPPABLE, TAILCALL>
{
    fn default() -> Self {
        Self::new(Self::DEFAULT_CACHE_SIZE)
    }
}

impl<const STEPPABLE: bool, const TAILCALL: bool> CodeAnalysisCache<STEPPABLE, TAILCALL> {
    #[cfg(not(feature = "fn-ptr-conversion-dispatch"))]
    const DEFAULT_CACHE_SIZE: usize = 1 << 16; // value taken from evmzero
    // 48B for OpFnData + 2*8B for entries in PcMap = 64B -> reduce size from 2^16 to 2^16 / 64 =
    // 2^10 to keep roughly the same memory size
    // default to 2^13 nonetheless for better performance
    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    const DEFAULT_CACHE_SIZE: usize = 1 << 13;

    #[allow(unused_variables)]
    pub fn new(size: usize) -> Self {
        #[cfg(feature = "code-analysis-cache")]
        return Self(Cache::new(size));
        #[cfg(not(feature = "code-analysis-cache"))]
        return Self();
    }

    #[cfg(test)]
    #[allow(clippy::unused_self)]
    pub fn capacity(&self) -> usize {
        #[cfg(feature = "code-analysis-cache")]
        return self.0.capacity();
        #[cfg(not(feature = "code-analysis-cache"))]
        return 0;
    }
}

#[derive(Debug, Clone)]
pub struct CodeAnalysis<const STEPPABLE: bool, const TAILCALL: bool>(
    #[cfg(feature = "code-analysis-cache")] Arc<[AnalysisItem<STEPPABLE, TAILCALL>]>,
    #[cfg(not(feature = "code-analysis-cache"))] Vec<AnalysisItem<STEPPABLE, TAILCALL>>,
);

impl<const STEPPABLE: bool, const TAILCALL: bool> CodeAnalysis<STEPPABLE, TAILCALL> {
    #[allow(unused_variables)]
    pub fn new(
        code: &[u8],
        code_hash: Option<u256>,
        cache: &CodeAnalysisCache<STEPPABLE, TAILCALL>,
    ) -> Self {
        #[cfg(feature = "code-analysis-cache")]
        match code_hash {
            Some(code_hash) if code_hash != u256::ZERO => cache
                .0
                .get_or_insert(u256Hash(code_hash), || Self::analyze_code(code)),
            _ => Self::analyze_code(code),
        }
        #[cfg(not(feature = "code-analysis-cache"))]
        Self::analyze_code(code)
    }

    #[cfg(not(feature = "fn-ptr-conversion-dispatch"))]
    fn analyze_code(code: &[u8]) -> Self {
        let mut code_byte_types = vec![CodeByteType::DataOrInvalid; code.len()];

        let mut pc = 0;
        while let Some(op) = code.get(pc).copied() {
            let (code_byte_type, data) = code_byte_type(op);
            code_byte_types[pc] = code_byte_type;
            pc += 1 + data;
        }

        CodeAnalysis(code_byte_types)
    }
    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    fn analyze_code(code: &[u8]) -> Self {
        use std::mem::MaybeUninit;

        use crate::Opcode;

        let mut analysis_arc = Arc::new_uninit_slice(code.len());
        let analysis = Arc::get_mut(&mut analysis_arc).unwrap();

        let mut analysis_init = 0;
        let mut pc = 0;
        let mut no_ops = 0;
        while let Some(op) = code.get(pc).copied() {
            let (code_byte_type, data_len) = code_byte_type(op);

            pc += 1;
            match code_byte_type {
                CodeByteType::JumpDest => {
                    if no_ops > 0 {
                        for op_fn in OpFnData::skip_no_ops_iter(no_ops, pc - 1) {
                            analysis[analysis_init] = MaybeUninit::new(op_fn);
                            analysis_init += 1;
                        }
                    }
                    no_ops = 0;
                    analysis[analysis_init] = MaybeUninit::new(OpFnData::jump_dest(pc - 1));
                    analysis_init += 1;
                }
                CodeByteType::Push => {
                    let mut data = [0; 32];
                    let avail = min(data_len, code.len() - pc);
                    data[32 - data_len..32 - data_len + avail]
                        .copy_from_slice(&code[pc..pc + avail]);
                    let data = u256::from_be_bytes(data);
                    analysis[analysis_init] = MaybeUninit::new(OpFnData::func(op, pc - 1, data));
                    analysis_init += 1;

                    no_ops += data_len;
                    pc += data_len;
                }
                CodeByteType::Opcode => {
                    analysis[analysis_init] =
                        MaybeUninit::new(OpFnData::func(op, pc - 1, u256::ZERO));
                    analysis_init += 1;
                }
                CodeByteType::DataOrInvalid => {
                    // This should only be the case if an invalid opcode was not preceded by a push.
                    // In this case we don't care what the data contains.
                    analysis[analysis_init] = MaybeUninit::new(OpFnData::data(u256::ZERO));
                    analysis_init += 1;
                }
            };
        }

        while analysis_init < analysis.len() {
            analysis[analysis_init] = MaybeUninit::new(OpFnData::func(
                Opcode::Stop as u8,
                analysis.len(),
                u256::ZERO,
            ));
            analysis_init += 1;
        }

        // SAFETY:
        // All elements have been initialized.
        let analysis = unsafe { analysis_arc.assume_init() };
        CodeAnalysis(analysis)
    }
}

impl<const STEPPABLE: bool, const TAILCALL: bool> Deref for CodeAnalysis<STEPPABLE, TAILCALL> {
    type Target = [AnalysisItem<STEPPABLE, TAILCALL>];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    #[cfg(not(feature = "fn-ptr-conversion-dispatch"))]
    use crate::types::CodeByteType;
    use crate::types::{CodeAnalysis, Opcode};
    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    use crate::types::{OpFnData, u256};

    #[cfg(not(feature = "fn-ptr-conversion-dispatch"))]
    #[test]
    fn analyze_code_single_byte() {
        assert_eq!(
            CodeAnalysis::<false>::analyze_code(&[Opcode::Add as u8]),
            [CodeByteType::Opcode]
        );
        assert_eq!(
            CodeAnalysis::<false>::analyze_code(&[Opcode::Push2 as u8]),
            [CodeByteType::Opcode]
        );
        assert_eq!(
            CodeAnalysis::<false>::analyze_code(&[Opcode::JumpDest as u8]),
            [CodeByteType::JumpDest]
        );
        assert_eq!(
            CodeAnalysis::<false>::analyze_code(&[0xc0]),
            [CodeByteType::DataOrInvalid]
        );
    }

    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    #[test]
    fn analyze_code_single_byte() {
        assert_eq!(
            *CodeAnalysis::<false>::analyze_code(&[Opcode::Add as u8]),
            [OpFnData::<false>::func(Opcode::Add as u8, 0, u256::ZERO)]
        );
        assert_eq!(
            *CodeAnalysis::<false>::analyze_code(&[Opcode::Push2 as u8]),
            [OpFnData::<false>::func(Opcode::Push2 as u8, 0, u256::ZERO)]
        );
        assert_eq!(
            *CodeAnalysis::<false>::analyze_code(&[Opcode::JumpDest as u8]),
            [OpFnData::jump_dest(0)]
        );
        assert_eq!(
            *CodeAnalysis::<false>::analyze_code(&[0xc0]),
            [OpFnData::data(u256::ZERO)]
        );
    }
    #[cfg(not(feature = "fn-ptr-conversion-dispatch"))]
    #[test]
    fn analyze_code_jumpdest() {
        assert_eq!(
            CodeAnalysis::<false>::analyze_code(&[Opcode::JumpDest as u8, Opcode::Add as u8]),
            [CodeByteType::JumpDest, CodeByteType::Opcode]
        );
        assert_eq!(
            CodeAnalysis::<false>::analyze_code(&[Opcode::JumpDest as u8, 0xc0]),
            [CodeByteType::JumpDest, CodeByteType::DataOrInvalid]
        );
    }

    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    #[test]
    fn analyze_code_jumpdest() {
        use crate::u256;

        assert_eq!(
            *CodeAnalysis::<false>::analyze_code(&[Opcode::JumpDest as u8, Opcode::Add as u8]),
            [
                OpFnData::jump_dest(0),
                OpFnData::<false>::func(Opcode::Add as u8, 1, u256::ZERO)
            ]
        );
        assert_eq!(
            *CodeAnalysis::<false>::analyze_code(&[Opcode::JumpDest as u8, 0xc0]),
            [OpFnData::jump_dest(0), OpFnData::data(u256::ZERO)]
        );
    }

    #[cfg(not(feature = "fn-ptr-conversion-dispatch"))]
    #[test]
    fn analyze_code_push_with_data() {
        assert_eq!(
            CodeAnalysis::<false>::analyze_code(&[
                Opcode::Push1 as u8,
                Opcode::Add as u8,
                Opcode::Add as u8
            ]),
            [
                CodeByteType::Opcode,
                CodeByteType::DataOrInvalid,
                CodeByteType::Opcode,
            ]
        );
        assert_eq!(
            CodeAnalysis::<false>::analyze_code(&[Opcode::Push1 as u8, Opcode::Add as u8, 0xc0]),
            [
                CodeByteType::Opcode,
                CodeByteType::DataOrInvalid,
                CodeByteType::DataOrInvalid,
            ]
        );
        assert_eq!(
            CodeAnalysis::<false>::analyze_code(&[
                Opcode::Push1 as u8,
                Opcode::Add as u8,
                0xc0,
                Opcode::Add as u8
            ]),
            [
                CodeByteType::Opcode,
                CodeByteType::DataOrInvalid,
                CodeByteType::DataOrInvalid,
                CodeByteType::Opcode,
            ]
        );
        assert_eq!(
            CodeAnalysis::<false>::analyze_code(&[
                Opcode::Push2 as u8,
                Opcode::Add as u8,
                Opcode::Add as u8,
                Opcode::Add as u8,
            ]),
            [
                CodeByteType::Opcode,
                CodeByteType::DataOrInvalid,
                CodeByteType::DataOrInvalid,
                CodeByteType::Opcode,
            ]
        );
        assert_eq!(
            CodeAnalysis::<false>::analyze_code(&[
                Opcode::Push2 as u8,
                Opcode::Add as u8,
                Opcode::Add as u8,
                0xc0
            ]),
            [
                CodeByteType::Opcode,
                CodeByteType::DataOrInvalid,
                CodeByteType::DataOrInvalid,
                CodeByteType::DataOrInvalid,
            ]
        );
    }
    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    #[test]
    fn analyze_code_push_with_data() {
        use crate::u256;

        assert_eq!(
            *CodeAnalysis::<false>::analyze_code(&[
                Opcode::Push1 as u8,
                Opcode::Add as u8,
                Opcode::Add as u8
            ]),
            [
                OpFnData::<false>::func(Opcode::Push1 as u8, 0, (Opcode::Add as u8).into()),
                OpFnData::<false>::func(Opcode::Add as u8, 2, u256::ZERO),
                OpFnData::<false>::func(Opcode::Stop as u8, 3, u256::ZERO),
            ]
        );
        assert_eq!(
            *CodeAnalysis::<false>::analyze_code(&[Opcode::Push1 as u8, Opcode::Add as u8, 0xc0]),
            [
                OpFnData::<false>::func(Opcode::Push1 as u8, 0, (Opcode::Add as u8).into()),
                OpFnData::data(u256::ZERO),
                OpFnData::<false>::func(Opcode::Stop as u8, 3, u256::ZERO),
            ]
        );
        assert_eq!(
            *CodeAnalysis::<false>::analyze_code(&[
                Opcode::Push1 as u8,
                Opcode::Add as u8,
                0xc0,
                Opcode::Add as u8
            ]),
            [
                OpFnData::<false>::func(Opcode::Push1 as u8, 0, (Opcode::Add as u8).into()),
                OpFnData::data(u256::ZERO),
                OpFnData::<false>::func(Opcode::Add as u8, 3, u256::ZERO),
                OpFnData::<false>::func(Opcode::Stop as u8, 4, u256::ZERO),
            ]
        );
        assert_eq!(
            *CodeAnalysis::<false>::analyze_code(&[
                Opcode::Push2 as u8,
                Opcode::Add as u8,
                Opcode::Add as u8,
                Opcode::Add as u8,
            ]),
            [
                OpFnData::<false>::func(
                    Opcode::Push2 as u8,
                    0,
                    (((Opcode::Add as u8 as u64) << 8) + Opcode::Add as u8 as u64).into()
                ),
                OpFnData::<false>::func(Opcode::Add as u8, 3, u256::ZERO),
                OpFnData::<false>::func(Opcode::Stop as u8, 4, u256::ZERO),
                OpFnData::<false>::func(Opcode::Stop as u8, 4, u256::ZERO),
            ]
        );
        assert_eq!(
            *CodeAnalysis::<false>::analyze_code(&[
                Opcode::Push2 as u8,
                Opcode::Add as u8,
                Opcode::Add as u8,
                0xc0
            ]),
            [
                OpFnData::<false>::func(
                    Opcode::Push2 as u8,
                    0,
                    (((Opcode::Add as u8 as u64) << 8) + Opcode::Add as u8 as u64).into()
                ),
                OpFnData::data(u256::ZERO),
                OpFnData::<false>::func(Opcode::Stop as u8, 4, u256::ZERO),
                OpFnData::<false>::func(Opcode::Stop as u8, 4, u256::ZERO),
            ]
        );
        let mut code = [0; 23];
        code[0] = Opcode::Push21 as u8;
        code[1] = 1;
        code[21] = 2;
        code[22] = Opcode::Add as u8;
        assert_eq!(
            *CodeAnalysis::<false>::analyze_code(&code),
            [
                OpFnData::<false>::func(
                    Opcode::Push21 as u8,
                    0,
                    (u256::ONE << u256::from(8 * 20u8)) + u256::from(2u8)
                ),
                OpFnData::<false>::func(Opcode::Add as u8, 22, u256::ZERO),
                OpFnData::<false>::func(Opcode::Stop as u8, 23, u256::ZERO),
                OpFnData::<false>::func(Opcode::Stop as u8, 23, u256::ZERO),
                OpFnData::<false>::func(Opcode::Stop as u8, 23, u256::ZERO),
                OpFnData::<false>::func(Opcode::Stop as u8, 23, u256::ZERO),
                OpFnData::<false>::func(Opcode::Stop as u8, 23, u256::ZERO),
                OpFnData::<false>::func(Opcode::Stop as u8, 23, u256::ZERO),
                OpFnData::<false>::func(Opcode::Stop as u8, 23, u256::ZERO),
                OpFnData::<false>::func(Opcode::Stop as u8, 23, u256::ZERO),
                OpFnData::<false>::func(Opcode::Stop as u8, 23, u256::ZERO),
                OpFnData::<false>::func(Opcode::Stop as u8, 23, u256::ZERO),
                OpFnData::<false>::func(Opcode::Stop as u8, 23, u256::ZERO),
                OpFnData::<false>::func(Opcode::Stop as u8, 23, u256::ZERO),
                OpFnData::<false>::func(Opcode::Stop as u8, 23, u256::ZERO),
                OpFnData::<false>::func(Opcode::Stop as u8, 23, u256::ZERO),
                OpFnData::<false>::func(Opcode::Stop as u8, 23, u256::ZERO),
                OpFnData::<false>::func(Opcode::Stop as u8, 23, u256::ZERO),
                OpFnData::<false>::func(Opcode::Stop as u8, 23, u256::ZERO),
                OpFnData::<false>::func(Opcode::Stop as u8, 23, u256::ZERO),
                OpFnData::<false>::func(Opcode::Stop as u8, 23, u256::ZERO),
                OpFnData::<false>::func(Opcode::Stop as u8, 23, u256::ZERO),
                OpFnData::<false>::func(Opcode::Stop as u8, 23, u256::ZERO),
            ]
        );
    }
}
