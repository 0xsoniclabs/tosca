use std::cmp::min;

use evmc_vm::{ExecutionMessage, MessageFlags, Revision};

use crate::{
    types::{FailStatus, u256},
    utils::Gas,
};

pub trait SliceExt {
    /// Returns a slice of the original slice starting at `offset` and with length `len`, clamped to
    /// the bounds of the original slice.
    fn get_within_bounds(&self, offset: u256, len: u64) -> &[u8];

    /// Copies the contents of `src` into the slice, padding with zeros if `src` is shorter and
    /// truncating it if it is longer. Consumes gas for the copy operation.
    fn copy_padded(&mut self, src: &[u8], gas_left: &mut Gas) -> Result<(), FailStatus>;
}

impl SliceExt for [u8] {
    #[inline(always)]
    fn get_within_bounds(&self, offset: u256, len: u64) -> &[u8] {
        if len == 0 {
            return &[];
        }
        let (offset, offset_overflow) = offset.into_u64_with_overflow();
        if offset_overflow {
            return &[];
        }
        let offset = offset as usize;
        let len = len as usize;
        let end = offset.saturating_add(len);
        if offset >= self.len() {
            &[]
        } else {
            &self[offset..min(end, self.len())]
        }
    }

    #[inline(always)]
    fn copy_padded(&mut self, src: &[u8], gas_left: &mut Gas) -> Result<(), FailStatus> {
        gas_left.consume_copy_cost(self.len() as u64)?;
        let len = min(src.len(), self.len());
        self[..len].copy_from_slice(&src[..len]);
        self[len..].fill(0);
        Ok(())
    }
}

/// Returns the number of 32-byte words needed to store `byte_len` bytes.
#[inline(always)]
pub fn word_size(byte_len: u64) -> Result<u64, FailStatus> {
    let (end, overflow) = byte_len.overflowing_add(31);
    if overflow {
        std::hint::cold_path();
        return Err(FailStatus::OutOfGas);
    }
    Ok(end / 32)
}

/// Checks if the given `revision` is at least `min_revision`. Returns `Ok(())` if it is, or
/// `Err(FailStatus::UndefinedInstruction)` if it is not.
#[inline(always)]
pub fn check_min_revision(min_revision: Revision, revision: Revision) -> Result<(), FailStatus> {
    if revision < min_revision {
        std::hint::cold_path();
        return Err(FailStatus::UndefinedInstruction);
    }
    Ok(())
}

/// Checks if the given `message` is not read-only. Returns `Ok(())` if it is not read-only, or
/// `Err(FailStatus::StaticModeViolation)` if it is read-only.
#[inline(always)]
pub fn check_not_read_only(message: &ExecutionMessage) -> Result<(), FailStatus> {
    if message.flags & MessageFlags::EVMC_STATIC as u32 != 0 {
        std::hint::cold_path();
        return Err(FailStatus::StaticModeViolation);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use evmc_vm::{MessageFlags, Revision};
    use rstest::rstest;

    use super::*;
    use crate::types::{FailStatus, MockExecutionMessage, u256};

    #[rstest]
    #[case::empty_slice(&[], u256::ZERO, 1, &[])]
    #[case::zero_len(&[1], u256::ZERO, 0, &[])]
    #[case::whole_slice(&[1], u256::ZERO, 1, &[1])]
    #[case::len_past_end(&[1], u256::ZERO, 2, &[1])]
    #[case::sub_slice(&[1, 2, 3], u256::ONE, 1, &[2])]
    #[case::sub_slice_past_end(&[1, 2, 3], u256::ONE, 5, &[2, 3])]
    #[case::offset_at_end(&[1], u256::ONE, 1, &[])]
    #[case::offset_overflows_u64(&[1], u256::MAX, 1, &[])]
    #[case::offset_past_end(&[1], u256::from(u64::MAX), 1, &[])]
    #[case::end_overflows_usize(&[1, 2, 3], u256::ONE, u64::MAX, &[2, 3])]
    fn get_within_bounds_clamps_to_the_slice(
        #[case] data: &[u8],
        #[case] offset: u256,
        #[case] len: u64,
        #[case] expected: &[u8],
    ) {
        assert_eq!(data.get_within_bounds(offset, len), expected);
    }

    #[rstest]
    #[case::empty(vec![], &[], 1_000_000, Ok(()), vec![])]
    #[case::only_padding(vec![1], &[], 1_000_000, Ok(()), vec![0])]
    #[case::no_padding(vec![1], &[2], 1_000_000, Ok(()), vec![2])]
    #[case::partial_padding(vec![1, 2], &[3], 1_000_000, Ok(()), vec![3, 0])]
    #[case::src_truncated(vec![1], &[2, 3], 1_000_000, Ok(()), vec![2])]
    #[case::out_of_gas(vec![1], &[2], 0, Err(FailStatus::OutOfGas), vec![1])]
    fn copy_padded_copies_src_and_zeroes_the_rest(
        #[case] mut dest: Vec<u8>,
        #[case] src: &[u8],
        #[case] gas: i64,
        #[case] expected: Result<(), FailStatus>,
        #[case] expected_dest: Vec<u8>,
    ) {
        assert_eq!(dest.copy_padded(src, &mut Gas::new(gas)), expected);
        assert_eq!(dest, expected_dest);
    }

    #[test]
    fn word_size_returns_the_number_of_32_byte_words_needed() {
        assert_eq!(word_size(0), Ok(0));
        assert_eq!(word_size(1), Ok(1));
        assert_eq!(word_size(32), Ok(1));
        assert_eq!(word_size(33), Ok(2));
        assert_eq!(word_size(u64::MAX), Err(FailStatus::OutOfGas));
    }

    #[rstest]
    #[case::equal(Revision::EVMC_ISTANBUL, Revision::EVMC_ISTANBUL, Ok(()))]
    #[case::newer(Revision::EVMC_ISTANBUL, Revision::EVMC_CANCUN, Ok(()))]
    #[case::older(
        Revision::EVMC_CANCUN,
        Revision::EVMC_ISTANBUL,
        Err(FailStatus::UndefinedInstruction)
    )]
    fn check_min_revision_returns_whether_revision_is_at_least_min_revision(
        #[case] min_revision: Revision,
        #[case] revision: Revision,
        #[case] expected: Result<(), FailStatus>,
    ) {
        assert_eq!(check_min_revision(min_revision, revision), expected);
    }

    #[rstest]
    #[case::no_flags(0, Ok(()))]
    #[case::delegated(MessageFlags::EVMC_DELEGATED as u32, Ok(()))]
    #[case::static_(
        MessageFlags::EVMC_STATIC as u32,
        Err(FailStatus::StaticModeViolation)
    )]
    #[case::static_and_delegated(
        MessageFlags::EVMC_STATIC as u32 | MessageFlags::EVMC_DELEGATED as u32,
        Err(FailStatus::StaticModeViolation)
    )]
    fn check_not_read_only_returns_whether_static_flag_is_not_set(
        #[case] flags: u32,
        #[case] expected: Result<(), FailStatus>,
    ) {
        let message = MockExecutionMessage {
            flags,
            ..Default::default()
        };
        assert_eq!(check_not_read_only(&message.into()), expected);
    }
}
