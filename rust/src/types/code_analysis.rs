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

/// The analysis of a code, i.e. the entries that [`crate::types::Pc`] moves over.
#[derive(Clone, Debug)]
pub struct CodeAnalysis<const STEPPABLE: bool>(
    // Arc for shared ownership between a run and (if enabled) the code cache. It also
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
        // Every code byte has the entry at its own offset, so a jump destination sits at its code
        // offset and a push moves past a fixed number of entries. A push whose data runs past the
        // end of the code moves up to 32 entries past the end, so the 33 entries from the end of
        // the code on are terminators.
        let mut analysis = Arc::new_uninit_slice(code.len() + 32 + 1);
        let entries = Arc::get_mut(&mut analysis).unwrap();

        let mut pc = 0;
        while let Some(op) = code.get(pc).copied() {
            let (code_byte_type, data_len) = code_byte_type(op);

            pc += 1;
            match code_byte_type {
                CodeByteType::JumpDest => {
                    entries[pc - 1].write(OpFnData::jump_dest(pc - 1));
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
                    entries[pc - 1].write(OpFnData::func(op, data, pc - 1));

                    for (entry, offset) in entries[pc..pc + data_len].iter_mut().zip(pc..) {
                        entry.write(OpFnData::invalid(offset));
                    }
                    pc += data_len;
                }
                CodeByteType::Opcode => {
                    entries[pc - 1].write(OpFnData::func(op, u256::ZERO, pc - 1));
                }
                CodeByteType::DataOrInvalid => {
                    entries[pc - 1].write(OpFnData::invalid(pc - 1));
                }
            };
        }

        // The walk writes the entries of the code bytes, and of push data running past the end,
        // which become terminators here too. So the analysis always ends with the terminator and
        // dispatching needs no bounds check. Filling all entries past the code initializes the
        // whole allocation.
        for entry in &mut entries[code.len()..] {
            entry.write(OpFnData::terminator(pc));
        }

        // SAFETY: the loop above initialized every entry the walk did not write.
        Self(unsafe { analysis.assume_init() })
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

    /// Returns `entries` followed by the 33 terminators at `code_offset` that end every analysis.
    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    fn terminated(entries: &[OpFnData<false>], code_offset: usize) -> Vec<OpFnData<false>> {
        [
            entries.to_vec(),
            vec![OpFnData::terminator(code_offset); 33],
        ]
        .concat()
    }

    /// [`crate::types::Pc::new`] and [`crate::types::Code::try_jump`] use the code offset as the
    /// offset in the analysis.
    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    #[test]
    fn analyze_code_puts_the_entry_of_each_code_byte_at_its_code_offset() {
        for code in CODES {
            let analysis = CodeAnalysis::<false>::analyze_code(code);
            for offset in 0..code.len() {
                assert_eq!(analysis[offset].get_code_offset(), offset);
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
            terminated(&[OpFnData::func(Opcode::Add as u8, u256::ZERO, 0)], 1)
        );
        assert_eq!(
            *CodeAnalysis::<false>::analyze_code(&[Opcode::Push2 as u8]),
            terminated(&[OpFnData::func(Opcode::Push2 as u8, u256::ZERO, 0)], 3)
        );
        assert_eq!(
            *CodeAnalysis::<false>::analyze_code(&[Opcode::JumpDest as u8]),
            terminated(&[OpFnData::jump_dest(0)], 1)
        );
        assert_eq!(
            *CodeAnalysis::<false>::analyze_code(&[0xc0]),
            terminated(&[OpFnData::invalid(0)], 1)
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
            terminated(
                &[
                    OpFnData::jump_dest(0),
                    OpFnData::func(Opcode::Add as u8, u256::ZERO, 1)
                ],
                2
            )
        );
        assert_eq!(
            *CodeAnalysis::<false>::analyze_code(&[Opcode::JumpDest as u8, 0xc0]),
            terminated(&[OpFnData::jump_dest(0), OpFnData::invalid(1)], 2)
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
            terminated(
                &[
                    OpFnData::func(Opcode::Push1 as u8, (Opcode::Add as u8).into(), 0),
                    OpFnData::invalid(1),
                    OpFnData::func(Opcode::Add as u8, u256::ZERO, 2),
                ],
                3
            )
        );
        assert_eq!(
            *CodeAnalysis::<false>::analyze_code(&[Opcode::Push1 as u8, Opcode::Add as u8, 0xc0]),
            terminated(
                &[
                    OpFnData::func(Opcode::Push1 as u8, (Opcode::Add as u8).into(), 0),
                    OpFnData::invalid(1),
                    OpFnData::invalid(2),
                ],
                3
            )
        );
        assert_eq!(
            *CodeAnalysis::<false>::analyze_code(&[
                Opcode::Push1 as u8,
                Opcode::Add as u8,
                0xc0,
                Opcode::Add as u8
            ]),
            terminated(
                &[
                    OpFnData::func(Opcode::Push1 as u8, (Opcode::Add as u8).into(), 0),
                    OpFnData::invalid(1),
                    OpFnData::invalid(2),
                    OpFnData::func(Opcode::Add as u8, u256::ZERO, 3),
                ],
                4
            )
        );
        assert_eq!(
            *CodeAnalysis::<false>::analyze_code(&[
                Opcode::Push2 as u8,
                Opcode::Add as u8,
                Opcode::Add as u8,
                Opcode::Add as u8,
            ]),
            terminated(
                &[
                    OpFnData::func(
                        Opcode::Push2 as u8,
                        (((Opcode::Add as u8 as u64) << 8) + Opcode::Add as u8 as u64).into(),
                        0
                    ),
                    OpFnData::invalid(1),
                    OpFnData::invalid(2),
                    OpFnData::func(Opcode::Add as u8, u256::ZERO, 3),
                ],
                4
            )
        );
        assert_eq!(
            *CodeAnalysis::<false>::analyze_code(&[
                Opcode::Push2 as u8,
                Opcode::Add as u8,
                Opcode::Add as u8,
                0xc0
            ]),
            terminated(
                &[
                    OpFnData::func(
                        Opcode::Push2 as u8,
                        (((Opcode::Add as u8 as u64) << 8) + Opcode::Add as u8 as u64).into(),
                        0
                    ),
                    OpFnData::invalid(1),
                    OpFnData::invalid(2),
                    OpFnData::invalid(3),
                ],
                4
            )
        );
        let mut code = [0; 23];
        code[0] = Opcode::Push21 as u8;
        code[1] = 1;
        code[21] = 2;
        code[22] = Opcode::Add as u8;
        assert_eq!(
            *CodeAnalysis::<false>::analyze_code(&code),
            terminated(
                &[
                    vec![OpFnData::func(
                        Opcode::Push21 as u8,
                        (u256::ONE << u256::from(8 * 20u8)) + u256::from(2u8),
                        0
                    )],
                    (1..22).map(OpFnData::invalid).collect(),
                    vec![OpFnData::func(Opcode::Add as u8, u256::ZERO, 22)],
                ]
                .concat(),
                23
            )
        );

        // push data cut short by the end of the code is padded with zeros
        assert_eq!(
            *CodeAnalysis::<false>::analyze_code(&[Opcode::Push2 as u8, 0xff]),
            terminated(
                &[
                    OpFnData::func(Opcode::Push2 as u8, u256::from(0xff00u32), 0),
                    OpFnData::invalid(1),
                ],
                3
            )
        );
        assert_eq!(
            *CodeAnalysis::<false>::analyze_code(&[Opcode::Push32 as u8]),
            terminated(&[OpFnData::func(Opcode::Push32 as u8, u256::ZERO, 0)], 33)
        );
    }
}
