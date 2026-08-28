#[cfg(feature = "fn-ptr-conversion-dispatch")]
use std::cmp::min;
use std::{ops::Deref, sync::Arc};

#[cfg(feature = "code-analysis-cache")]
use nohash_hasher::BuildNoHashHasher;

#[cfg(feature = "code-analysis-cache")]
use crate::types::Cache;
#[cfg(feature = "fn-ptr-conversion-dispatch")]
use crate::types::OpFnData;
#[cfg(feature = "fn-ptr-conversion-dispatch")]
use crate::types::{BlockEnd, block_end, static_gas};
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

/// Marks a code offset that is not a jump destination in [`CodeAnalysis::jump_dests`].
#[cfg(feature = "fn-ptr-conversion-dispatch")]
const NO_JUMP_DEST: u32 = u32::MAX;

/// The analysis of a code, i.e. the entries that [`crate::types::CodeReader`] uses.
#[derive(Clone, Debug)]
pub struct CodeAnalysis<const STEPPABLE: bool> {
    // Arc for shared ownership between the code reader and (if enabled) the code cache. It also
    // has the benefit of not allowing mutable access to the slice, so a pointer into the Arc is
    // valid for as long as the Arc lives.
    items: Arc<[AnalysisItem<STEPPABLE>]>,
    /// For every code offset the index of its entry in `items`, or [`NO_JUMP_DEST`] if the offset
    /// is not a jump destination. Entries do not sit at the offset of the code byte they were
    /// created from, so jumping needs this to translate the two.
    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    jump_dests: Arc<[u32]>,
    /// Gas of the basic block that starts at the first entry. Every other block is charged from
    /// the entry that precedes it (see [`analyze_code`](Self::analyze_code)).
    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    first_block_gas: u64,
}

/// Where the gas of the basic block currently being analyzed has to be written to.
#[cfg(feature = "fn-ptr-conversion-dispatch")]
#[derive(Clone, Copy)]
enum BlockGasSink {
    FirstBlock,
    Entry(usize),
    /// The block cannot be entered by falling through, so its gas is charged by the jump
    /// destination it starts with.
    Unreachable,
}

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
        let end = min(code_offset, self.items.len() - 1);
        self.items[..=end]
            .partition_point(|item| item.get_code_offset() <= code_offset)
            .saturating_sub(1)
    }

    /// The index of the entry a jump to `code_offset` continues at, if that offset is a jump
    /// destination.
    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    pub fn jump_dest(&self, code_offset: usize) -> Option<usize> {
        match self.jump_dests.get(code_offset) {
            Some(&index) if index != NO_JUMP_DEST => Some(index as usize),
            _ => None,
        }
    }

    /// See [`CodeAnalysis::first_block_gas`].
    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    pub fn first_block_gas(&self) -> u64 {
        self.first_block_gas
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

        Self {
            items: code_byte_types.into(),
        }
    }

    /// Splits the code into basic blocks and records the gas of each one in the entry that
    /// precedes it, so that it can be charged once per block instead of once per instruction. A
    /// block is charged by the jump destination it starts with, by the instruction that falls
    /// through into it, or, for the block at the start of the code, by
    /// [`CodeAnalysis::first_block_gas`].
    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    fn analyze_code(code: &[u8]) -> Self {
        fn flush<const STEPPABLE: bool>(
            analysis: &mut [OpFnData<STEPPABLE>],
            first_block_gas: &mut u64,
            sink: BlockGasSink,
            block_gas: u64,
        ) {
            match sink {
                BlockGasSink::FirstBlock => *first_block_gas = block_gas,
                BlockGasSink::Entry(index) => analysis[index].set_data(block_gas.into()),
                BlockGasSink::Unreachable => (),
            }
        }

        let mut analysis = Vec::with_capacity(code.len() + 1); // +1 for terminator
        let mut jump_dests = vec![NO_JUMP_DEST; code.len()];
        let mut first_block_gas = 0;
        let mut sink = BlockGasSink::FirstBlock;
        let mut block_gas = 0;

        let mut pc = 0;
        while let Some(op) = code.get(pc).copied() {
            let (code_byte_type, data_len) = code_byte_type(op);

            pc += 1;
            match code_byte_type {
                CodeByteType::JumpDest => {
                    jump_dests[pc - 1] = analysis.len() as u32;
                    flush(&mut analysis, &mut first_block_gas, sink, block_gas);
                    sink = BlockGasSink::Entry(analysis.len());
                    block_gas = 0;
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
                    let data = u256::from_be_bytes(*buf[..32].as_array().unwrap());
                    analysis.push(OpFnData::func(op, data, pc - 1));

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

            block_gas += static_gas(op);
            // An invalid opcode always fails, so nothing after it can be reached by falling
            // through.
            let block_end = if code_byte_type == CodeByteType::DataOrInvalid {
                BlockEnd::Terminator
            } else {
                block_end(op)
            };
            match block_end {
                BlockEnd::No => (),
                BlockEnd::FallThrough => {
                    flush(&mut analysis, &mut first_block_gas, sink, block_gas);
                    sink = BlockGasSink::Entry(analysis.len() - 1);
                    block_gas = 0;
                }
                BlockEnd::Terminator => {
                    flush(&mut analysis, &mut first_block_gas, sink, block_gas);
                    sink = BlockGasSink::Unreachable;
                    block_gas = 0;
                }
            }
        }

        // Let the analysis always end with the terminator so dispatching needs no bounds check.
        analysis.push(OpFnData::terminator(pc));
        flush(&mut analysis, &mut first_block_gas, sink, block_gas);

        Self {
            items: analysis.into(),
            jump_dests: jump_dests.into(),
            first_block_gas,
        }
    }
}

impl<const STEPPABLE: bool> Deref for CodeAnalysis<STEPPABLE> {
    type Target = [AnalysisItem<STEPPABLE>];

    fn deref(&self) -> &Self::Target {
        &self.items
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Opcode;
    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    use crate::types::{OpFnData, u256};

    /// A jump destination entry as [`CodeAnalysis::analyze_code`] emits it, i.e. carrying the gas
    /// of the block it starts.
    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    fn jump_dest(code_offset: usize, block_gas: u64) -> OpFnData<false> {
        let mut jump_dest = OpFnData::jump_dest(code_offset);
        jump_dest.set_data(block_gas.into());
        jump_dest
    }

    /// A code together with the gas of its first basic block and the block gas recorded in the
    /// analysis entry at each of the listed indices.
    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    type BlockGasCase = (&'static [u8], u64, &'static [(usize, u64)]);

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

    /// [`crate::types::CodeReader::try_jump`] translates the code offset of a jump destination
    /// into the offset of its entry in the analysis.
    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    #[test]
    fn jump_dest_maps_a_code_offset_to_the_entry_at_that_offset() {
        for code in CODES {
            let analysis = CodeAnalysis::<false>::analyze_code(code);
            let mut jump_dests = 0;
            for (index, item) in analysis.iter().enumerate() {
                let code_offset = item.get_code_offset();
                if code.get(code_offset) == Some(&(Opcode::JumpDest as u8)) {
                    jump_dests += 1;
                    assert_eq!(Some(index), analysis.jump_dest(code_offset));
                }
            }
            // A jump destination byte that is push data does not become an entry, so it must not
            // be mapped either.
            let mapped = (0..code.len()).filter(|o| analysis.jump_dest(*o).is_some()).count();
            assert_eq!(jump_dests, mapped);
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
            [jump_dest(0, 1), OpFnData::terminator(1)]
        );
        assert_eq!(
            *CodeAnalysis::<false>::analyze_code(&[0xc0]),
            [OpFnData::data(u256::ZERO, 0), OpFnData::terminator(1)]
        );
    }
    #[cfg(feature = "fn-ptr-conversion-dispatch")]
    #[test]
    fn analyze_code_records_the_gas_of_every_basic_block() {
        const ADD: u8 = Opcode::Add as u8; // 3
        const DEST: u8 = Opcode::JumpDest as u8; // 1
        const GAS: u8 = Opcode::Gas as u8; // 2
        const JUMP: u8 = Opcode::Jump as u8; // 8
        const STOP: u8 = Opcode::Stop as u8; // 0

        let cases: &[BlockGasCase] = &[
            (&[], 0, &[]),
            (&[ADD, ADD], 3 + 3, &[]),
            // a jump destination charges the block it starts, its own gas included
            (&[ADD, DEST, ADD], 3, &[(1, 1 + 3)]),
            // ... which leaves no first block to charge if the code starts with one
            (&[DEST, ADD], 0, &[(0, 1 + 3)]),
            // GAS reads the gas left, so it ends its block and charges the following one
            (&[GAS, ADD], 2, &[(0, 3)]),
            // ... unless a jump destination follows, which charges its own block
            (&[GAS, DEST], 2, &[(0, 0), (1, 1)]),
            // code behind a terminator is only reachable through a jump destination
            (&[JUMP, ADD, DEST, ADD], 8, &[(2, 1 + 3)]),
            (&[STOP, ADD], 0, &[]),
        ];

        for (code, first_block_gas, entries) in cases {
            let analysis = CodeAnalysis::<false>::analyze_code(code);
            assert_eq!(*first_block_gas, analysis.first_block_gas());
            for (index, block_gas) in *entries {
                assert_eq!(u256::from(*block_gas), analysis[*index].get_data());
            }
        }
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
                jump_dest(0, 1 + 3),
                OpFnData::<false>::func(Opcode::Add as u8, u256::ZERO, 1),
                OpFnData::terminator(2)
            ]
        );
        assert_eq!(
            *CodeAnalysis::<false>::analyze_code(&[Opcode::JumpDest as u8, 0xc0]),
            [
                jump_dest(0, 1),
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
