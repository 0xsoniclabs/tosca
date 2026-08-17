#[cfg(feature = "fn-ptr-conversion-dispatch")]
use std::cmp::min;
use std::{ops::Deref, sync::Arc};

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
#[expect(non_camel_case_types)]
#[derive(Debug, PartialEq, Eq)]
struct u256Hash(u256);

#[cfg(feature = "code-analysis-cache")]
impl std::hash::Hash for u256Hash {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u64(self.0.into_u64_with_overflow().0);
    }
}

#[cfg(not(feature = "fn-ptr-conversion-dispatch"))]
pub type AnalysisItem<const STEPPABLE: bool> = CodeByteType;
#[cfg(feature = "fn-ptr-conversion-dispatch")]
pub type AnalysisItem<const STEPPABLE: bool> = OpFnData<STEPPABLE>;

pub struct CodeAnalysisCache<const STEPPABLE: bool>(
    #[cfg(feature = "code-analysis-cache")]
    Cache<u256Hash, CodeAnalysis<STEPPABLE>, BuildNoHashHasher<u64>>,
);

impl<const STEPPABLE: bool> Default for CodeAnalysisCache<STEPPABLE> {
    fn default() -> Self {
        Self::new(Self::DEFAULT_CACHE_SIZE)
    }
}

impl<const STEPPABLE: bool> CodeAnalysisCache<STEPPABLE> {
    #[cfg(not(feature = "fn-ptr-conversion-dispatch"))]
    const DEFAULT_CACHE_SIZE: usize = 1 << 16; // value taken from evmzero
    // 48B for OpFnData instead of 1B for CodeByteType -> reduce size from 2^16 to about
    // 2^16 / 48 to keep roughly the same memory size
    // default to 2^13 nonetheless for better performance
    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    const DEFAULT_CACHE_SIZE: usize = 1 << 13;

    #[allow(unused_variables)]
    pub fn new(size: usize) -> Self {
        std::cfg_select! {
            feature = "code-analysis-cache" => Self(Cache::new(size)),
            _ => Self(),
        }
    }

    #[cfg(test)]
    #[allow(clippy::unused_self)]
    pub fn capacity(&self) -> usize {
        std::cfg_select! {
            feature = "code-analysis-cache" => self.0.capacity(),
            _ => 0,
        }
    }
}

/// The analysis of a code, i.e. the entries that [`crate::types::CodeReader`] uses.
#[derive(Clone, Debug)]
pub struct CodeAnalysis<const STEPPABLE: bool>(
    // Arc for shared ownership between the code reader and (if enabled) the code cache. It also
    // has the benefit of not allowing mutable access to the slice, so a pointer into the Arc is
    // valid for as long as the Arc lives.
    Arc<[AnalysisItem<STEPPABLE>]>,
);

impl<const STEPPABLE: bool> CodeAnalysis<STEPPABLE> {
    #[allow(unused_variables)]
    pub fn new(code: &[u8], code_hash: Option<u256>, cache: &CodeAnalysisCache<STEPPABLE>) -> Self {
        std::cfg_select! {
            feature = "code-analysis-cache" => match code_hash {
                Some(code_hash) if code_hash != u256::ZERO => cache
                    .0
                    .get_or_insert(u256Hash(code_hash), || Self::analyze_code(code)),
                _ => Self::analyze_code(code),
            },
            _ => Self::analyze_code(code),
        }
    }

    /// Converts an offset in the original code to the offset of the corresponding entry in the
    /// analysis.
    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    pub fn analysis_offset(&self, code_offset: usize) -> usize {
        // Entry offsets (see [`OpFnData::get_code_offset`]) never decrease, so the entry to enter
        // at is the last one whose offset does not exceed code_offset. No entry sits at a higher
        // offset in the analysis than in the code, so there is nothing to find beyond code_offset.
        let end = min(code_offset, self.0.len() - 1);
        self.0[..=end]
            .partition_point(|item| item.get_code_offset() <= code_offset)
            .saturating_sub(1)
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

        Self(code_byte_types.into())
    }

    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    fn analyze_code(code: &[u8]) -> Self {
        let mut analysis = Vec::with_capacity(code.len() + 1); // +1 for terminator

        let mut pc = 0;
        let mut no_ops = 0;
        while let Some(op) = code.get(pc).copied() {
            let (code_byte_type, data_len) = code_byte_type(op);

            pc += 1;
            match code_byte_type {
                CodeByteType::JumpDest => {
                    if no_ops > 0 {
                        analysis.extend(OpFnData::skip_no_ops_iter(no_ops, pc - 1));
                    }
                    no_ops = 0;
                    analysis.push(OpFnData::jump_dest(pc - 1));
                }
                CodeByteType::Push => {
                    // Copying a fixed size window of the code to offset `32 - data_len` right
                    // aligns the push data in the first 32 bytes of the buffer. Unlike a copy of
                    // `data_len` bytes this does not compile to a call to memcpy.
                    let mut buf = [0; 64];
                    if let Some(window) = code.get(pc..pc + 32) {
                        buf[32 - data_len..64 - data_len].copy_from_slice(window);
                    } else {
                        let avail = code.len() - pc;
                        buf[32 - data_len..32 - data_len + avail].copy_from_slice(&code[pc..]);
                    }
                    let data = u256::from_be_bytes_words(*buf[..32].as_array().unwrap());
                    analysis.push(OpFnData::func(op, data, pc - 1));

                    no_ops += data_len;
                    pc += data_len;
                }
                CodeByteType::Opcode => {
                    analysis.push(OpFnData::func(op, u256::ZERO, pc - 1));
                }
                CodeByteType::DataOrInvalid => {
                    // This should only be the case if an invalid opcode was not preceded by a push.
                    // In this case we don't care what the data contains.
                    analysis.push(OpFnData::data(u256::ZERO, pc - 1));
                }
            };
        }

        // Let the analysis always end with the terminator so dispatching needs no bounds check.
        analysis.push(OpFnData::terminator(pc));

        Self(analysis.into())
    }
}

impl<const STEPPABLE: bool> Deref for CodeAnalysis<STEPPABLE> {
    type Target = [AnalysisItem<STEPPABLE>];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Opcode;
    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    use crate::types::{OpFnData, u256};

    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    const CODES: &[&[u8]] = {
        const ADD: u8 = Opcode::Add as u8;
        const DEST: u8 = Opcode::JumpDest as u8;
        const PUSH1: u8 = Opcode::Push1 as u8;
        const PUSH2: u8 = Opcode::Push2 as u8;
        const PUSH3: u8 = Opcode::Push3 as u8;
        &[
            &[],
            &[ADD],
            &[PUSH1, 0xff, ADD],
            &[PUSH2, 0xff], // push data runs past the end of the code
            &[0xc0, DEST],
            &[DEST, PUSH1, 0xff, DEST, ADD],
            &[PUSH3, 0xff, 0xff, DEST, DEST, PUSH1, DEST, DEST],
        ]
    };

    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    #[test]
    fn analysis_offset_finds_the_last_entry_with_the_requested_code_offset() {
        for code in CODES {
            let analysis = CodeAnalysis::<false>::analyze_code(code);
            for (index, item) in analysis.iter().enumerate() {
                let offset = analysis.analysis_offset(item.get_code_offset());
                assert_eq!(analysis[offset].get_code_offset(), item.get_code_offset());
                assert!(offset >= index);
            }
        }

        let code = [
            Opcode::Add as u8,
            Opcode::Push1 as u8,
            0xff,
            Opcode::Add as u8,
        ];
        let analysis = CodeAnalysis::<false>::analyze_code(&code);
        assert_eq!(analysis.analysis_offset(2), 1);
    }

    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    #[test]
    fn analysis_offset_clamps_a_code_offset_past_the_end_to_the_terminator() {
        for code in CODES {
            let analysis = CodeAnalysis::<false>::analyze_code(code);
            assert_eq!(
                analysis.analysis_offset(code.len() + 33),
                analysis.len() - 1
            );
        }
    }

    /// [`crate::types::CodeReader::try_jump`] uses the code offset of a jump destination as its
    /// offset in the analysis.
    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    #[test]
    fn jump_dest_entries_sit_at_their_code_offset() {
        for code in CODES {
            let analysis = CodeAnalysis::<false>::analyze_code(code);
            for (index, item) in analysis.iter().enumerate() {
                if item.code_byte_type() == CodeByteType::JumpDest {
                    assert_eq!(index, item.get_code_offset());
                }
            }
        }
    }

    #[cfg(not(feature = "fn-ptr-conversion-dispatch"))]
    #[test]
    fn analyze_code_single_byte() {
        assert_eq!(
            *CodeAnalysis::<false>::analyze_code(&[Opcode::Add as u8]),
            [CodeByteType::Opcode]
        );
        assert_eq!(
            *CodeAnalysis::<false>::analyze_code(&[Opcode::Push2 as u8]),
            [CodeByteType::Opcode]
        );
        assert_eq!(
            *CodeAnalysis::<false>::analyze_code(&[Opcode::JumpDest as u8]),
            [CodeByteType::JumpDest]
        );
        assert_eq!(
            *CodeAnalysis::<false>::analyze_code(&[0xc0]),
            [CodeByteType::DataOrInvalid]
        );
    }

    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    #[test]
    fn analyze_code_single_byte() {
        assert_eq!(
            *CodeAnalysis::<false>::analyze_code(&[Opcode::Add as u8]),
            [
                OpFnData::<false>::func(Opcode::Add as u8, u256::ZERO, 0),
                OpFnData::terminator(1)
            ]
        );
        assert_eq!(
            *CodeAnalysis::<false>::analyze_code(&[Opcode::Push2 as u8]),
            [
                OpFnData::<false>::func(Opcode::Push2 as u8, u256::ZERO, 0),
                OpFnData::terminator(3)
            ]
        );
        assert_eq!(
            *CodeAnalysis::<false>::analyze_code(&[Opcode::JumpDest as u8]),
            [OpFnData::jump_dest(0), OpFnData::terminator(1)]
        );
        assert_eq!(
            *CodeAnalysis::<false>::analyze_code(&[0xc0]),
            [OpFnData::data(u256::ZERO, 0), OpFnData::terminator(1)]
        );
    }
    #[cfg(not(feature = "fn-ptr-conversion-dispatch"))]
    #[test]
    fn analyze_code_jumpdest() {
        assert_eq!(
            *CodeAnalysis::<false>::analyze_code(&[Opcode::JumpDest as u8, Opcode::Add as u8]),
            [CodeByteType::JumpDest, CodeByteType::Opcode]
        );
        assert_eq!(
            *CodeAnalysis::<false>::analyze_code(&[Opcode::JumpDest as u8, 0xc0]),
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
                OpFnData::<false>::func(Opcode::Add as u8, u256::ZERO, 1),
                OpFnData::terminator(2)
            ]
        );
        assert_eq!(
            *CodeAnalysis::<false>::analyze_code(&[Opcode::JumpDest as u8, 0xc0]),
            [
                OpFnData::jump_dest(0),
                OpFnData::data(u256::ZERO, 1),
                OpFnData::terminator(2)
            ]
        );
    }

    #[cfg(not(feature = "fn-ptr-conversion-dispatch"))]
    #[test]
    fn analyze_code_push_with_data() {
        assert_eq!(
            *CodeAnalysis::<false>::analyze_code(&[
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
            *CodeAnalysis::<false>::analyze_code(&[Opcode::Push1 as u8, Opcode::Add as u8, 0xc0]),
            [
                CodeByteType::Opcode,
                CodeByteType::DataOrInvalid,
                CodeByteType::DataOrInvalid,
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
                CodeByteType::Opcode,
                CodeByteType::DataOrInvalid,
                CodeByteType::DataOrInvalid,
                CodeByteType::Opcode,
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
                CodeByteType::Opcode,
                CodeByteType::DataOrInvalid,
                CodeByteType::DataOrInvalid,
                CodeByteType::Opcode,
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
                OpFnData::<false>::func(Opcode::Push1 as u8, (Opcode::Add as u8).into(), 0),
                OpFnData::<false>::func(Opcode::Add as u8, u256::ZERO, 2),
                OpFnData::terminator(3),
            ]
        );
        assert_eq!(
            *CodeAnalysis::<false>::analyze_code(&[Opcode::Push1 as u8, Opcode::Add as u8, 0xc0]),
            [
                OpFnData::<false>::func(Opcode::Push1 as u8, (Opcode::Add as u8).into(), 0),
                OpFnData::data(u256::ZERO, 2),
                OpFnData::terminator(3),
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
                OpFnData::<false>::func(Opcode::Push1 as u8, (Opcode::Add as u8).into(), 0),
                OpFnData::data(u256::ZERO, 2),
                OpFnData::<false>::func(Opcode::Add as u8, u256::ZERO, 3),
                OpFnData::terminator(4),
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
                    (((Opcode::Add as u8 as u64) << 8) + Opcode::Add as u8 as u64).into(),
                    0
                ),
                OpFnData::<false>::func(Opcode::Add as u8, u256::ZERO, 3),
                OpFnData::terminator(4),
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
                    (((Opcode::Add as u8 as u64) << 8) + Opcode::Add as u8 as u64).into(),
                    0
                ),
                OpFnData::data(u256::ZERO, 3),
                OpFnData::terminator(4),
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
                    (u256::ONE << u256::from(8 * 20u8)) + u256::from(2u8),
                    0
                ),
                OpFnData::<false>::func(Opcode::Add as u8, u256::ZERO, 22),
                OpFnData::terminator(23),
            ]
        );

        // push data cut short by the end of the code is padded with zeros
        assert_eq!(
            *CodeAnalysis::<false>::analyze_code(&[Opcode::Push2 as u8, 0xff]),
            [
                OpFnData::<false>::func(Opcode::Push2 as u8, u256::from(0xff00u32), 0),
                OpFnData::terminator(3),
            ]
        );
        assert_eq!(
            *CodeAnalysis::<false>::analyze_code(&[Opcode::Push32 as u8]),
            [
                OpFnData::<false>::func(Opcode::Push32 as u8, u256::ZERO, 0),
                OpFnData::terminator(33),
            ]
        );
    }
}
