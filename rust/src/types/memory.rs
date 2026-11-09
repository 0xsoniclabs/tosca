use std::cmp::max;

use crate::{
    types::{FailStatus, u256},
    utils::{Gas, word_size},
};

/// The interpreter memory. All operations that access it expand it as needed, rounding the length
/// up to whole words, and consume the expansion cost.
#[derive(Debug)]
pub struct Memory(Vec<u8>);

impl Memory {
    /// Creates a new empty [`Memory`].
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// Replaces the content of the memory with `memory`.
    pub fn reset_to(&mut self, memory: &[u8]) {
        self.0.clear();
        self.0.extend_from_slice(memory);
    }

    /// Returns the content of the memory.
    pub fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }

    /// Returns the length of the memory in bytes.
    pub fn len(&self) -> u64 {
        self.0.len() as u64
    }

    /// Expands the memory with zeros to `new_len_bytes` rounded up to whole words and consumes the
    /// expansion cost. Does nothing if the memory is at least that long. Fails with
    /// [`FailStatus::OutOfGas`] if not enough gas is left.
    fn expand(&mut self, new_len_bytes: u64, gas_left: &mut Gas) -> Result<(), FailStatus> {
        #[cold]
        fn expand_raw(m: &mut Memory, new_len: u64, gas_left: &mut Gas) -> Result<(), FailStatus> {
            m.consume_expansion_cost(new_len, gas_left)?;
            m.0.resize(new_len as usize, 0);
            Ok(())
        }

        let current_len = self.0.len() as u64;
        let new_len = word_size(new_len_bytes)? * 32; // word_size is at most u64::MAX / 32 so * will not overflow
        if new_len > current_len {
            expand_raw(self, new_len, gas_left)?;
        }
        #[cfg(feature = "unsafe-hints")]
        // SAFETY:
        // new_len is new_len_bytes rounded up to whole words and the memory was expanded to at
        // least that length above.
        unsafe {
            std::hint::assert_unchecked(new_len_bytes <= self.0.len() as u64);
        }
        Ok(())
    }

    /// Consumes the cost of expanding the memory to `new_len` bytes, which is the difference
    /// between the memory cost of the new and the current length. Fails with
    /// [`FailStatus::OutOfGas`] if not enough gas is left.
    fn consume_expansion_cost(&self, new_len: u64, gas_left: &mut Gas) -> Result<(), FailStatus> {
        /// Returns the cost of a memory of `size` bytes, which is `words^2 / 512 + 3 * words` for
        /// the number of started words. Fails with [`FailStatus::OutOfGas`] on overflow.
        fn memory_cost(size: u64) -> Result<u64, FailStatus> {
            let word_size = word_size(size)?;
            let (pow2, pow2_overflow) = word_size.overflowing_pow(2);
            if pow2_overflow {
                std::hint::cold_path();
                return Err(FailStatus::OutOfGas);
            };
            let word_size_3 = word_size * 3; // word_size is at most 2^64 / 32 so *3 will not overflow
            Ok(pow2 / 512 + word_size_3) // pow2 / 512 < 2^55 and word_size_3 < 2^62 so + will not overflow
        }

        let current_len = self.0.len() as u64;

        if new_len > current_len {
            let memory_expansion_cost = memory_cost(new_len)? - memory_cost(current_len)?;
            gas_left.consume(memory_expansion_cost)?;
        }
        Ok(())
    }

    /// Returns the `len` bytes at `offset`, expanding the memory if needed. A zero length is free
    /// at any offset. Fails with [`FailStatus::OutOfGas`] if the range exceeds `u64` or not enough
    /// gas is left for the expansion.
    pub fn get_mut_slice(
        &mut self,
        offset: u256,
        len: u64,
        gas_left: &mut Gas,
    ) -> Result<&mut [u8], FailStatus> {
        if len == 0 {
            return Ok(&mut []);
        }
        let (offset, offset_overflow) = offset.into_u64_with_overflow();
        let (end, end_overflow) = offset.overflowing_add(len);
        if offset_overflow || end_overflow {
            std::hint::cold_path();
            return Err(FailStatus::OutOfGas);
        }
        self.expand(end, gas_left)?;

        let offset = offset as usize;
        let end = end as usize;
        Ok(&mut self.0[offset..end])
    }

    /// Returns the `N` bytes at `offset` as an array, like [`Self::get_mut_slice`].
    pub fn get_mut_array<const N: usize>(
        &mut self,
        offset: u256,
        gas_left: &mut Gas,
    ) -> Result<&mut [u8; N], FailStatus> {
        let slice = self.get_mut_slice(offset, N as u64, gas_left)?;
        Ok(slice.as_mut_array().unwrap())
    }

    /// Returns the 32 bytes at `offset` as a big endian [`u256`], like [`Self::get_mut_slice`].
    pub fn get_word(&mut self, offset: u256, gas_left: &mut Gas) -> Result<u256, FailStatus> {
        Ok(u256::from_be_bytes(*self.get_mut_array(offset, gas_left)?))
    }

    /// Copies `len` bytes from `src_offset` to `dest_offset`; the ranges may overlap. Consumes the
    /// copy cost and expands the memory to cover both ranges. A zero length is free at any offset.
    /// Fails with [`FailStatus::OutOfGas`] if a range exceeds `u64` or not enough gas is left.
    #[inline(always)]
    pub fn copy_within(
        &mut self,
        src_offset: u256,
        dest_offset: u256,
        len: u256,
        gas_left: &mut Gas,
    ) -> Result<(), FailStatus> {
        if len == u256::ZERO {
            return Ok(());
        }
        let (src_offset, src_overflow) = src_offset.into_u64_with_overflow();
        let (dest_offset, dest_overflow) = dest_offset.into_u64_with_overflow();
        let (len, len_overflow) = len.into_u64_with_overflow();
        let (end, end_overflow) = max(src_offset, dest_offset).overflowing_add(len);
        if src_overflow || dest_overflow || len_overflow || end_overflow {
            std::hint::cold_path();
            return Err(FailStatus::OutOfGas);
        }
        gas_left.consume_copy_cost(len)?;
        self.expand(end, gas_left)?;
        let src_offset = src_offset as usize;
        let dest_offset = dest_offset as usize;
        let len = len as usize;
        self.0
            .copy_within(src_offset..src_offset + len, dest_offset); // + does not overflow
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use FailStatus::OutOfGas;
    use rstest::rstest;

    use super::*;

    fn two_pow_64() -> u256 {
        u256::ONE << u256::from(64u8)
    }

    // Concatenates multiple arrays into a single vector.
    macro_rules! cat {
        ($($array:expr),+) => {
            [$(&$array[..]),+].concat()
        };
    }

    #[test]
    fn memory_new_creates_an_empty_memory() {
        assert_eq!(Memory::new().as_slice(), []);
    }

    #[rstest]
    #[case::empty_to_content(vec![], &[1, 2], vec![1, 2])]
    #[case::longer_to_shorter(vec![1; 64], &[2], vec![2])]
    #[case::content_to_empty(vec![1; 32], &[], vec![])]
    fn memory_reset_to_replaces_the_content(
        #[case] memory: Vec<u8>,
        #[case] content: &[u8],
        #[case] expected: Vec<u8>,
    ) {
        let mut memory = Memory(memory);
        memory.reset_to(content);
        assert_eq!(memory.0, expected);
    }

    #[test]
    fn memory_as_slice_returns_the_content() {
        assert_eq!(Memory(vec![1, 2]).as_slice(), [1, 2]);
    }

    #[test]
    fn memory_len_returns_the_length_in_bytes() {
        assert_eq!(Memory(vec![0; 3]).len(), 3);
    }

    #[rstest]
    #[case::zero_len(vec![], 0, 0, Ok(()), vec![], 0)]
    #[case::rounds_up_to_a_word(vec![], 1, 3, Ok(()), vec![0; 32], 0)]
    #[case::exact_word(vec![], 32, 3, Ok(()), vec![0; 32], 0)]
    #[case::past_the_content(vec![1; 32], 64, 3, Ok(()), cat!([1; 32], [0; 32]), 0)]
    #[case::already_large_enough(vec![1; 64], 32, 1, Ok(()), vec![1; 64], 1)]
    #[case::unaligned_content_to_its_word(vec![1], 2, 0, Ok(()), cat!([1], [0; 31]), 0)]
    #[case::out_of_gas(vec![], 1, 2, Err(OutOfGas), vec![], 2)]
    #[case::rounding_up_overflows(vec![], u64::MAX, 1_000, Err(OutOfGas), vec![], 1_000)]
    fn memory_expand_zero_fills_to_whole_words_and_fails_if_out_of_gas(
        #[case] memory: Vec<u8>,
        #[case] new_len: u64,
        #[case] gas: i64,
        #[case] expected: Result<(), FailStatus>,
        #[case] expected_memory: Vec<u8>,
        #[case] gas_left: u64,
    ) {
        let mut memory = Memory(memory);
        let mut gas = Gas::new(gas).unwrap();
        assert_eq!(memory.expand(new_len, &mut gas), expected);
        assert_eq!(memory.0, expected_memory);
        assert_eq!(gas, gas_left);
    }

    #[rstest]
    #[case::zero_len(0, 0, 0, Ok(()), 0)]
    #[case::rounds_up_to_a_word(0, 1, 3, Ok(()), 0)]
    #[case::exact_word(0, 32, 3, Ok(()), 0)]
    #[case::past_the_content(32, 64, 3, Ok(()), 0)]
    #[case::already_large_enough(64, 32, 1, Ok(()), 1)]
    #[case::quadratic(0, 1024 * 32, 5_120, Ok(()), 0)]
    #[case::quadratic_difference(1024 * 32, 2048 * 32, 9_216, Ok(()), 0)]
    #[case::out_of_gas(0, 32, 2, Err(OutOfGas), 2)]
    #[case::rounding_up_overflows(0, u64::MAX, 1_000, Err(OutOfGas), 1_000)]
    #[case::cost_overflows(0, (1 << 32) * 32, Gas::MAX.cast_signed(), Err(OutOfGas), Gas::MAX)]
    fn memory_consume_expansion_cost_charges_the_cost_difference_and_fails_if_out_of_gas(
        #[case] len: usize,
        #[case] new_len: u64,
        #[case] gas: i64,
        #[case] expected: Result<(), FailStatus>,
        #[case] gas_left: u64,
    ) {
        let memory = Memory(vec![0; len]);
        let mut gas = Gas::new(gas).unwrap();
        assert_eq!(memory.consume_expansion_cost(new_len, &mut gas), expected);
        assert_eq!(gas, gas_left);
    }

    #[rstest]
    #[case::zero_len_is_free_at_any_offset(vec![], u256::MAX, 0, 0, Ok(vec![]), vec![], 0)]
    #[case::rounds_up_to_a_word(vec![], u256::ZERO, 1, 3, Ok(vec![0]), vec![0; 32], 0)]
    #[case::exact_word(vec![], u256::ZERO, 32, 3, Ok(vec![0; 32]), vec![0; 32], 0)]
    #[case::rounds_up_to_two_words(vec![], u256::ZERO, 33, 6, Ok(vec![0; 33]), vec![0; 64], 0)]
    #[case::within_memory(vec![1; 32], u256::ZERO, 1, 0, Ok(vec![1]), vec![1; 32], 0)]
    #[case::whole_memory(vec![1; 32], u256::ZERO, 32, 0, Ok(vec![1; 32]), vec![1; 32], 0)]
    #[case::past_the_content(
        vec![1; 32],
        u256::ZERO,
        33,
        3,
        Ok(cat!([1; 32], [0])),
        cat!([1; 32], [0; 32]),
        0
    )]
    #[case::at_offset(
        vec![1; 64],
        32u8.into(),
        64,
        3,
        Ok(cat!([1; 32], [0; 32])),
        cat!([1; 64], [0; 32]),
        0
    )]
    #[case::out_of_gas(vec![], u256::ZERO, 1, 2, Err(OutOfGas), vec![], 2)]
    #[case::offset_exceeds_u64(vec![], two_pow_64(), 1, 1_000, Err(OutOfGas), vec![], 1_000)]
    #[case::end_exceeds_u64(vec![], u64::MAX.into(), 1, 1_000, Err(OutOfGas), vec![], 1_000)]
    fn memory_get_mut_slice_expands_the_memory_and_fails_if_out_of_gas(
        #[case] memory: Vec<u8>,
        #[case] offset: u256,
        #[case] len: u64,
        #[case] gas: i64,
        #[case] expected: Result<Vec<u8>, FailStatus>,
        #[case] expected_memory: Vec<u8>,
        #[case] gas_left: u64,
    ) {
        let mut memory = Memory(memory);
        let mut gas = Gas::new(gas).unwrap();
        assert_eq!(
            memory
                .get_mut_slice(offset, len, &mut gas)
                .map(|slice| slice.to_vec()),
            expected
        );
        assert_eq!(memory.0, expected_memory);
        assert_eq!(gas, gas_left);
    }

    #[test]
    fn memory_get_mut_slice_returns_a_mutable_reference_into_the_memory() {
        let mut memory = Memory(vec![0; 32]);
        let mut gas = Gas::new(0).unwrap();
        memory
            .get_mut_slice(u256::ONE, 2, &mut gas)
            .unwrap()
            .copy_from_slice(&[1, 2]);
        assert_eq!(memory.0, cat!([0, 1, 2], [0; 29]));
    }

    #[test]
    fn memory_get_mut_array_returns_n_bytes() {
        let content = cat!([1, 2, 3], [0; 29]);
        let mut memory = Memory(content.clone());
        let mut gas = Gas::new(0).unwrap();
        assert_eq!(memory.get_mut_array(u256::ONE, &mut gas), Ok(&mut [2, 3]));
        assert_eq!(memory.0, content);
    }

    #[test]
    fn memory_get_word_reads_a_big_endian_word() {
        let mut memory = Memory(cat!([1], [0; 30], [2]));
        let mut gas = Gas::new(0).unwrap();
        let expected = (u256::ONE << u256::from(248u8)) + u256::from(2u8);
        assert_eq!(memory.get_word(u256::ZERO, &mut gas), Ok(expected));
    }

    #[rstest]
    #[case::zero_len_is_free_at_any_offset(vec![], u256::MAX, u256::MAX, u256::ZERO, 0, vec![])]
    #[case::overlapping_forward(
        cat!([1, 2, 3, 4], [0; 28]),
        u256::ZERO,
        u256::ONE,
        3u8.into(),
        3,
        cat!([1, 1, 2, 3], [0; 28])
    )]
    #[case::overlapping_backward(
        cat!([1, 2, 3, 4], [0; 28]),
        u256::ONE,
        u256::ZERO,
        3u8.into(),
        3,
        cat!([2, 3, 4, 4], [0; 28])
    )]
    #[case::expands_to_the_dest_range(
        vec![1; 32],
        u256::ZERO,
        32u8.into(),
        u256::ONE,
        3 + 3,
        cat!([1; 33], [0; 31])
    )]
    #[case::expands_to_the_src_range(
        vec![1; 32],
        32u8.into(),
        u256::ZERO,
        u256::ONE,
        3 + 3,
        cat!([0], [1; 31], [0; 32])
    )]
    #[case::past_the_content(
        vec![1; 32],
        u256::ZERO,
        u256::ZERO,
        33u8.into(),
        3 * 2 + 3,
        cat!([1; 32], [0; 32])
    )]
    fn memory_copy_within_copies_and_expands_to_cover_both_ranges(
        #[case] memory: Vec<u8>,
        #[case] src_offset: u256,
        #[case] dest_offset: u256,
        #[case] len: u256,
        #[case] gas: i64,
        #[case] expected_memory: Vec<u8>,
    ) {
        let mut memory = Memory(memory);
        let mut gas = Gas::new(gas).unwrap();
        assert_eq!(
            memory.copy_within(src_offset, dest_offset, len, &mut gas),
            Ok(())
        );
        assert_eq!(memory.0, expected_memory);
        assert_eq!(gas, 0);
    }

    #[rstest]
    #[case::out_of_gas_for_the_copy(vec![0; 32], u256::ZERO, u256::ZERO, u256::ONE, 2, 2)]
    #[case::out_of_gas_for_the_expansion(vec![], u256::ZERO, u256::ZERO, u256::ONE, 3 + 2, 2)]
    #[case::src_offset_exceeds_u64(vec![], two_pow_64(), u256::ZERO, u256::ONE, 1_000, 1_000)]
    #[case::dest_offset_exceeds_u64(vec![], u256::ZERO, two_pow_64(), u256::ONE, 1_000, 1_000)]
    #[case::len_exceeds_u64(vec![], u256::ZERO, u256::ZERO, two_pow_64(), 1_000, 1_000)]
    #[case::end_exceeds_u64(vec![], u64::MAX.into(), u256::ZERO, u256::ONE, 1_000, 1_000)]
    fn memory_copy_within_fails_if_out_of_gas_or_a_range_exceeds_u64(
        #[case] memory: Vec<u8>,
        #[case] src_offset: u256,
        #[case] dest_offset: u256,
        #[case] len: u256,
        #[case] gas: i64,
        #[case] gas_left: u64,
    ) {
        let memory_before = memory.clone();
        let mut memory = Memory(memory);
        let mut gas = Gas::new(gas).unwrap();
        assert_eq!(
            memory.copy_within(src_offset, dest_offset, len, &mut gas),
            Err(OutOfGas)
        );
        assert_eq!(memory.0, memory_before);
        assert_eq!(gas, gas_left);
    }
}
