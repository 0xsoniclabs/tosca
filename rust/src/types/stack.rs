use std::cmp::min;

use crate::{
    types::{FailStatus, u256},
    utils::Gas,
};

/// This type is created by calling [`Stack::pop_with_location`] and is intended to replace pushing
/// to the stack directly. It avoids the stack overflow check when pushing because it is no
/// longer needed. [`PushLocation`] has to be consumed by pushing to it.
/// If this does not happen, the program is still memory safe, however there will be one item too
/// much on the stack.
///
/// Internally it is a wrapper around [`&mut u256`] that ensures that the only possible operation is
/// to write once to this memory location.
#[derive(Debug)]
#[must_use = "PushLocation has to be pushed to."]
pub struct PushLocation<'p>(&'p mut u256);

impl PushLocation<'_> {
    /// Writes `value` to the slot, which is the top of the stack.
    pub fn push(self, value: impl Into<u256>) {
        *self.0 = value.into();
    }
}

/// The elements of a [`Stack`].
pub type StackBuffer = [u256; Stack::CAPACITY];

/// The gas counter and the size of a [`Stack`] packed into one word, so that both fit into one
/// register, see [Return value](crate::interpreter#return-value). The gas counter takes the upper
/// 48 bits. Only [`GasAndSize::new`] packs them, from a [`Gas`] and a [`Stack`], so both keep the
/// invariants of their types.
#[derive(Clone, Copy, Debug)]
pub struct GasAndSize(u64);

impl GasAndSize {
    /// Packs `gas_left` and the size of `stack`.
    #[expect(clippy::needless_pass_by_value)]
    pub fn new(gas_left: Gas, stack: Stack<'_>) -> Self {
        Self(gas_left.as_u64() << 16 | stack.size as u64)
    }

    /// Returns the gas counter.
    pub fn gas(self) -> Gas {
        // Cannot fail: the upper 48 bits are at most `Gas::MAX`, so the compiler drops the check.
        Gas::new((self.0 >> 16).cast_signed()).unwrap()
    }

    /// Returns the size of the stack.
    fn stack_size(self) -> usize {
        (self.0 & 0xffff) as usize
    }
}

/// The interpreter stack: a borrowed [`StackBuffer`] and the size of its used part, two words that
/// a run passes from handler to handler.
pub struct Stack<'s> {
    /// The elements, bottom first. Only the first `size` bytes are on the stack.
    buffer: &'s mut StackBuffer,
    /// The used part of the buffer in bytes. Bytes, not elements, so that it is the offset of the
    /// top: x86 addressing cannot scale an index by the element size of 32.
    ///
    /// Invariant: a multiple of the element size and at most the size of the buffer. The hint in
    /// [`Stack::len`] relies on it with `unsafe-stack`.
    size: usize,
}

impl<'s> Stack<'s> {
    /// The maximum number of elements.
    const CAPACITY: usize = 1024;

    /// Creates a zeroed buffer for a stack.
    #[inline(always)]
    pub fn new_buffer() -> Box<StackBuffer> {
        // Built on the heap, not on the machine stack like Box::new would.
        let buffer = vec![u256::ZERO; Self::CAPACITY].into_boxed_slice();
        // Cannot fail: the buffer has CAPACITY elements.
        buffer.try_into().unwrap()
    }

    /// Lends `buffer` out as an empty stack.
    #[inline(always)]
    pub fn empty(buffer: &'s mut StackBuffer) -> Self {
        Self { buffer, size: 0 }
    }

    /// Lends `buffer` out as the stack whose size `state` holds.
    pub fn new(buffer: &'s mut StackBuffer, state: GasAndSize) -> Self {
        Self {
            buffer,
            size: state.stack_size(),
        }
    }

    /// Replaces the content of the stack with the first [`Self::CAPACITY`] elements of `values`.
    pub fn reset_to(&mut self, values: &[u256]) {
        let len = min(values.len(), Self::CAPACITY);
        self.buffer[..len].copy_from_slice(&values[..len]);
        self.size = len * size_of::<u256>();
    }

    /// Returns the elements on the stack, bottom first.
    pub fn as_slice(&self) -> &[u256] {
        &self.buffer[..self.len()]
    }

    /// Returns the elements on the stack, bottom first, for modification.
    fn as_mut_slice(&mut self) -> &mut [u256] {
        let len = self.len();
        &mut self.buffer[..len]
    }

    /// Returns the number of elements on the stack.
    pub fn len(&self) -> usize {
        #[cfg(feature = "unsafe-stack")]
        // SAFETY:
        // size is a multiple of the element size and at most the size of the buffer (invariant).
        unsafe {
            std::hint::assert_unchecked(
                self.size.is_multiple_of(size_of::<u256>())
                    && self.size <= size_of::<StackBuffer>(),
            );
        }
        self.size / size_of::<u256>()
    }

    /// Shrinks the stack by `n` slots, or empties it when it holds fewer.
    fn shrink(&mut self, n: usize) {
        self.size = self.size.saturating_sub(n * size_of::<u256>());
    }

    /// Pushes `value` or fails with [`FailStatus::StackOverflow`] if the stack is full.
    pub fn push(&mut self, value: impl Into<u256>) -> Result<(), FailStatus> {
        self.check_overflow(1)?;
        self.buffer[self.len()] = value.into();
        self.size += size_of::<u256>();
        Ok(())
    }

    /// Swaps the top element with the `N`th element below it or fails with
    /// [`FailStatus::StackUnderflow`] if the stack holds `N` or fewer elements.
    pub fn swap_with_top<const N: usize>(&mut self) -> Result<(), FailStatus> {
        const { assert!(N > 0) };

        self.check_underflow(N + 1)?;

        // Swapping through disjoint borrows of the top and the rest instead of via [`slice::swap`]
        // lets the compiler shuffle the values in registers instead of copying them through the
        // stack.
        // Cannot fail and is optimized away: len > N >= 1.
        let (top, rest) = self.as_mut_slice().split_last_mut().unwrap();
        let len = rest.len();
        std::mem::swap(top, &mut rest[len - N]);

        Ok(())
    }

    /// Pops `N` entries from the stack and returns them as an array which is ordered such that the
    /// former top of stack is at the end of the array.
    pub fn pop<const N: usize>(&mut self) -> Result<[u256; N], FailStatus> {
        self.check_underflow(N)?;

        let slice = self.as_slice();
        let array = *slice[slice.len() - N..].as_array().unwrap();
        self.shrink(N);
        Ok(array)
    }

    /// Pops `N` entries from the stack, ordered like [`Stack::pop`], and returns a
    /// [`PushLocation`] for the slot the result must be written to. That slot is already
    /// accounted for in the stack's length.
    pub fn pop_with_location<const N: usize>(
        &mut self,
    ) -> Result<(PushLocation<'_>, [u256; N]), FailStatus> {
        const { assert!(N > 0) };

        self.check_underflow(N)?;

        let slice = self.as_slice();
        let pop_data = *slice[slice.len() - N..].as_array().unwrap();
        self.shrink(N - 1);
        // Not indexed with len - 1: LLVM then loses that the slot is not null and tests it for the
        // error case of the returned Result.
        // Cannot fail and is optimized away: len >= 1 after the shrink.
        let slot = self.as_mut_slice().iter_mut().next_back().unwrap();
        Ok((PushLocation(slot), pop_data))
    }

    /// Returns the top element or [`None`] if the stack is empty.
    pub fn peek(&self) -> Option<&u256> {
        self.as_slice().last()
    }

    /// Pushes a copy of the `N`th element from the top, 1 being the top itself. Fails with
    /// [`FailStatus::StackUnderflow`] if the stack holds fewer than `N` elements and with
    /// [`FailStatus::StackOverflow`] if it is full.
    pub fn dup<const N: usize>(&mut self) -> Result<(), FailStatus> {
        const { assert!(N > 0) };

        self.check_underflow(N)?;
        let element = self.as_slice()[self.len() - N];
        self.push(element)
    }

    /// Fails with [`FailStatus::StackOverflow`] if fewer than `num_elements` slots are free.
    #[inline(always)]
    pub fn check_overflow(&self, num_elements: usize) -> Result<(), FailStatus> {
        // len <= CAPACITY (invariant), so this does not underflow
        if Self::CAPACITY - self.len() < num_elements {
            std::hint::cold_path();
            return Err(FailStatus::StackOverflow);
        }
        Ok(())
    }

    /// Fails with [`FailStatus::StackUnderflow`] if the stack holds fewer than `min_len`
    /// elements.
    #[inline(always)]
    fn check_underflow(&self, min_len: usize) -> Result<(), FailStatus> {
        if self.len() < min_len {
            std::hint::cold_path();
            return Err(FailStatus::StackUnderflow);
        }
        Ok(())
    }
}

impl std::fmt::Debug for Stack<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Stack").field(&self.as_slice()).finish()
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    /// Returns `values` as [`u256`]s.
    fn u256s(values: &[u8]) -> Vec<u256> {
        values.iter().copied().map(u256::from).collect()
    }

    /// Creates a stack on `buffer` holding `values`, bottom first, without using the methods under
    /// test.
    fn stack<'s>(buffer: &'s mut StackBuffer, values: &[u8]) -> Stack<'s> {
        for (slot, value) in buffer.iter_mut().zip(values) {
            *slot = u256::from(*value);
        }
        let size = values.len() * size_of::<u256>();
        assert!(size <= size_of::<StackBuffer>());
        Stack { buffer, size }
    }

    #[test]
    fn push_location_push_writes_value_to_slot() {
        let mut slot = u256::ZERO;
        PushLocation(&mut slot).push(1u8);
        assert_eq!(slot, u256::ONE);
    }

    #[rstest]
    #[case::empty(0, &[], 0)]
    #[case::gas_only(1, &[], 1 << 16)]
    #[case::size_only(0, &[1, 2], 2 * 32)]
    #[case::both(5, &[1, 2], (5 << 16) | (2 * 32))]
    #[case::full(Gas::MAX, &[1; Stack::CAPACITY], (Gas::MAX << 16) | (Stack::CAPACITY as u64 * 32))]
    fn gas_and_size_new_packs_gas_into_upper_48_bits_and_stack_size_into_lower_16(
        #[case] gas: u64,
        #[case] values: &[u8],
        #[case] expected: u64,
    ) {
        let gas = Gas::new(gas.cast_signed()).unwrap();
        let mut buffer = Stack::new_buffer();
        assert_eq!(GasAndSize::new(gas, stack(&mut buffer, values)).0, expected);
    }

    #[rstest]
    #[case::empty(0, 0)]
    #[case::ignores_size(2 * 32, 0)]
    #[case::gas(5 << 16, 5)]
    #[case::both((5 << 16) | (2 * 32), 5)]
    #[case::max((Gas::MAX << 16) | (Stack::CAPACITY as u64 * 32), Gas::MAX)]
    fn gas_and_size_gas_returns_upper_48_bits(#[case] word: u64, #[case] expected: u64) {
        assert_eq!(GasAndSize(word).gas(), expected);
    }

    #[rstest]
    #[case::empty(0, 0)]
    #[case::ignores_gas(5 << 16, 0)]
    #[case::size(2 * 32, 2 * 32)]
    #[case::both((5 << 16) | (2 * 32), 2 * 32)]
    #[case::max((Gas::MAX << 16) | (Stack::CAPACITY as u64 * 32), Stack::CAPACITY * 32)]
    fn gas_and_size_stack_size_returns_lower_16_bits(#[case] word: u64, #[case] expected: usize) {
        assert_eq!(GasAndSize(word).stack_size(), expected);
    }

    #[test]
    fn new_buffer_creates_zeroed_elements() {
        assert!(Stack::new_buffer().iter().all(|value| *value == u256::ZERO));
    }

    #[test]
    fn empty_lends_elements_as_empty_stack() {
        let mut buffer = Stack::new_buffer();
        buffer.fill(u256::ONE);
        assert_eq!(Stack::empty(&mut buffer).len(), 0);
    }

    #[rstest]
    #[case::empty(&[])]
    #[case::values(&[1, 2])]
    #[case::full(&[1; Stack::CAPACITY])]
    fn new_lends_elements_as_stack_of_the_size_in_state(#[case] values: &[u8]) {
        let mut buffer = Stack::new_buffer();
        let state = GasAndSize::new(Gas::new(7).unwrap(), stack(&mut buffer, values));
        assert_eq!(Stack::new(&mut buffer, state).as_slice(), u256s(values));
    }

    #[rstest]
    #[case::empty_with_nothing(&[], &[], &[])]
    #[case::empty_with_values(&[], &[1, 2], &[1, 2])]
    #[case::values_with_other_values(&[1, 2, 3], &[4], &[4])]
    #[case::values_with_nothing(&[1, 2], &[], &[])]
    #[case::truncates_to_capacity(&[], &[1; Stack::CAPACITY + 1], &[1; Stack::CAPACITY])]
    fn reset_to_replaces_content_with_at_most_capacity_elements(
        #[case] existing: &[u8],
        #[case] new: &[u8],
        #[case] expected: &[u8],
    ) {
        let mut buffer = Stack::new_buffer();
        let mut stack = stack(&mut buffer, existing);
        stack.reset_to(&u256s(new));
        assert_eq!(stack.as_slice(), u256s(expected));
    }

    #[rstest]
    #[case::empty(&[])]
    #[case::values(&[1, 2])]
    #[case::full(&[1; Stack::CAPACITY])]
    fn as_slice_returns_elements_bottom_first(#[case] values: &[u8]) {
        assert_eq!(
            stack(&mut Stack::new_buffer(), values).as_slice(),
            u256s(values)
        );
    }

    #[rstest]
    #[case::empty(&[])]
    #[case::values(&[1, 2])]
    #[case::full(&[1; Stack::CAPACITY])]
    fn as_mut_slice_returns_elements_bottom_first_for_modification(#[case] values: &[u8]) {
        let mut buffer = Stack::new_buffer();
        let mut stack = stack(&mut buffer, values);
        let slice = stack.as_mut_slice();
        assert_eq!(slice, u256s(values));
        slice.fill(u256::from(3u8));
        assert_eq!(stack.as_slice(), vec![u256::from(3u8); values.len()]);
    }

    #[rstest]
    #[case::empty(&[], 0)]
    #[case::values(&[1, 2], 2)]
    #[case::full(&[1; Stack::CAPACITY], Stack::CAPACITY)]
    fn len_returns_number_of_elements(#[case] values: &[u8], #[case] expected: usize) {
        assert_eq!(stack(&mut Stack::new_buffer(), values).len(), expected);
    }

    #[rstest]
    #[case::nothing(&[1, 2], 0, &[1, 2])]
    #[case::some(&[1, 2], 1, &[1])]
    #[case::all(&[1, 2], 2, &[])]
    #[case::more_than_all(&[1, 2], 3, &[])]
    fn shrink_removes_top_elements_and_empties_if_it_holds_fewer(
        #[case] values: &[u8],
        #[case] n: usize,
        #[case] expected: &[u8],
    ) {
        let mut buffer = Stack::new_buffer();
        let mut stack = stack(&mut buffer, values);
        stack.shrink(n);
        assert_eq!(stack.as_slice(), u256s(expected));
    }

    #[rstest]
    #[case::empty(&[], Ok(()), &[3])]
    #[case::values(&[1, 2], Ok(()), &[1, 2, 3])]
    #[case::last_slot(&[3; Stack::CAPACITY - 1], Ok(()), &[3; Stack::CAPACITY])]
    #[case::full(&[1; Stack::CAPACITY], Err(FailStatus::StackOverflow), &[1; Stack::CAPACITY])]
    fn push_pushes_value_and_fails_if_the_stack_is_full(
        #[case] values: &[u8],
        #[case] expected: Result<(), FailStatus>,
        #[case] remaining: &[u8],
    ) {
        let mut buffer = Stack::new_buffer();
        let mut stack = stack(&mut buffer, values);
        assert_eq!(stack.push(3u8), expected);
        assert_eq!(stack.as_slice(), u256s(remaining));
    }

    #[rstest]
    #[case::first(|stack: &mut Stack| stack.swap_with_top::<1>(), &[1, 2, 3], Ok(()), &[1, 3, 2])]
    #[case::second(|stack: &mut Stack| stack.swap_with_top::<2>(), &[1, 2, 3], Ok(()), &[3, 2, 1])]
    #[case::underflow(|stack: &mut Stack| stack.swap_with_top::<2>(), &[1, 2], Err(FailStatus::StackUnderflow), &[1, 2])]
    fn swap_with_top_swaps_top_with_nth_element_below_and_fails_on_underflow(
        #[case] swap: fn(&mut Stack) -> Result<(), FailStatus>,
        #[case] values: &[u8],
        #[case] expected: Result<(), FailStatus>,
        #[case] remaining: &[u8],
    ) {
        let mut buffer = Stack::new_buffer();
        let mut stack = stack(&mut buffer, values);
        assert_eq!(swap(&mut stack), expected);
        assert_eq!(stack.as_slice(), u256s(remaining));
    }

    #[rstest]
    #[case::one(&[1, 2], Ok([2]), &[1])]
    #[case::two_top_last(&[1, 2, 3], Ok([2, 3]), &[1])]
    #[case::all(&[1, 2], Ok([1, 2]), &[])]
    #[case::underflow(&[1], Err::<[_; 2], _>(FailStatus::StackUnderflow), &[1])]
    fn pop_removes_top_elements_and_returns_them_top_last_and_fails_on_underflow<const N: usize>(
        #[case] values: &[u8],
        #[case] expected: Result<[u8; N], FailStatus>,
        #[case] remaining: &[u8],
    ) {
        let mut buffer = Stack::new_buffer();
        let mut stack = stack(&mut buffer, values);
        assert_eq!(
            stack.pop::<N>(),
            expected.map(|values| values.map(u256::from))
        );
        assert_eq!(stack.as_slice(), u256s(remaining));
    }

    #[rstest]
    #[case::one(&[1, 2], Ok([2]), &[1, 9])]
    #[case::two_top_last(&[1, 2, 3], Ok([2, 3]), &[1, 9])]
    #[case::all(&[1, 2], Ok([1, 2]), &[9])]
    #[case::underflow(&[1], Err::<[_; 2], _>(FailStatus::StackUnderflow), &[1])]
    fn pop_with_location_returns_top_elements_and_the_result_slot_and_fails_on_underflow<
        const N: usize,
    >(
        #[case] values: &[u8],
        #[case] expected: Result<[u8; N], FailStatus>,
        #[case] remaining: &[u8],
    ) {
        let mut buffer = Stack::new_buffer();
        let mut stack = stack(&mut buffer, values);
        let data = stack.pop_with_location::<N>().map(|(slot, data)| {
            slot.push(9u8);
            data
        });
        assert_eq!(data, expected.map(|values| values.map(u256::from)));
        assert_eq!(stack.as_slice(), u256s(remaining));
    }

    #[rstest]
    #[case::empty(&[], None)]
    #[case::values(&[1, 2], Some(u256::from(2u8)))]
    fn peek_returns_top_element(#[case] values: &[u8], #[case] expected: Option<u256>) {
        assert_eq!(
            stack(&mut Stack::new_buffer(), values).peek(),
            expected.as_ref()
        );
    }

    #[rstest]
    #[case::top(|stack: &mut Stack| stack.dup::<1>(), &[1, 2], Ok(()), &[1, 2, 2])]
    #[case::second(|stack: &mut Stack| stack.dup::<2>(), &[1, 2], Ok(()), &[1, 2, 1])]
    #[case::underflow(|stack: &mut Stack| stack.dup::<3>(), &[1, 2], Err(FailStatus::StackUnderflow), &[1, 2])]
    #[case::overflow(
        |stack: &mut Stack| stack.dup::<1>(),
        &[1; Stack::CAPACITY],
        Err(FailStatus::StackOverflow),
        &[1; Stack::CAPACITY]
    )]
    fn dup_pushes_copy_of_nth_element_and_fails_on_underflow_or_overflow(
        #[case] dup: fn(&mut Stack) -> Result<(), FailStatus>,
        #[case] values: &[u8],
        #[case] expected: Result<(), FailStatus>,
        #[case] remaining: &[u8],
    ) {
        let mut buffer = Stack::new_buffer();
        let mut stack = stack(&mut buffer, values);
        assert_eq!(dup(&mut stack), expected);
        assert_eq!(stack.as_slice(), u256s(remaining));
    }

    #[rstest]
    #[case::empty_up_to_capacity(&[], Stack::CAPACITY, Ok(()))]
    #[case::empty_beyond_capacity(&[], Stack::CAPACITY + 1, Err(FailStatus::StackOverflow))]
    #[case::almost_full_one(&[1; Stack::CAPACITY - 1], 1, Ok(()))]
    #[case::almost_full_two(&[1; Stack::CAPACITY - 1], 2, Err(FailStatus::StackOverflow))]
    #[case::full_none(&[1; Stack::CAPACITY], 0, Ok(()))]
    #[case::full_one(&[1; Stack::CAPACITY], 1, Err(FailStatus::StackOverflow))]
    fn check_overflow_fails_if_fewer_slots_are_free(
        #[case] values: &[u8],
        #[case] num_elements: usize,
        #[case] expected: Result<(), FailStatus>,
    ) {
        assert_eq!(
            stack(&mut Stack::new_buffer(), values).check_overflow(num_elements),
            expected
        );
    }

    #[rstest]
    #[case::empty_none(&[], 0, Ok(()))]
    #[case::empty_one(&[], 1, Err(FailStatus::StackUnderflow))]
    #[case::exact(&[1, 2], 2, Ok(()))]
    #[case::more(&[1, 2], 3, Err(FailStatus::StackUnderflow))]
    fn check_underflow_fails_if_the_stack_holds_fewer_elements(
        #[case] values: &[u8],
        #[case] min_len: usize,
        #[case] expected: Result<(), FailStatus>,
    ) {
        assert_eq!(
            stack(&mut Stack::new_buffer(), values).check_underflow(min_len),
            expected
        );
    }

    #[test]
    fn debug_formats_only_the_elements_on_the_stack() {
        assert_eq!(
            format!("{:?}", stack(&mut Stack::new_buffer(), &[1, 2])),
            format!("Stack({:?})", u256s(&[1, 2]))
        );
    }
}
