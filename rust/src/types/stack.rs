use std::{
    cmp::min,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

use crate::types::{FailStatus, u256};

/// The number of elements in a [`StackBuffer`], which is the EVM stack limit.
const CAPACITY: usize = 1024;

/// Owns the memory a [`Stack`] lives in; the stack itself only borrows it. Between two borrows
/// the buffer keeps the elements, so a stack can be recreated from it with [`Stack::with_len`].
#[derive(Debug)]
pub struct StackBuffer(Box<[u256; CAPACITY]>);

impl StackBuffer {
    pub fn new() -> Self {
        let buffer = vec![u256::ZERO; CAPACITY].into_boxed_slice().try_into();
        // try_into cannot fail, the vector has exactly CAPACITY elements
        Self(buffer.unwrap_or_else(|_| unreachable!()))
    }
}

impl Deref for StackBuffer {
    type Target = [u256; CAPACITY];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for StackBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Default for StackBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// This type is created by calling [`Stack::pop_with_location`] and is intended to replace pushing
/// to the stack directly. It and avoids the stack overflow check when pushing because it is no
/// longer needed. [`PushLocation`] has to be consumed by pushing to it.
/// If this does not happen, the program is still memory safe, however there will be one item so
/// much on the stack.
///
/// Internally it is a wrapper around [`&mut u256`] that ensures that the only possible operation is
/// to write once to this memory location.
#[derive(Debug)]
#[must_use = "PushLocation has to be pushed to."]
pub struct PushLocation<'p>(&'p mut u256);

impl PushLocation<'_> {
    pub fn push(self, value: impl Into<u256>) {
        *self.0 = value.into();
    }
}

/// A stack occupying the top of a borrowed fixed-size buffer and growing downward: element 0 is
/// the top of the stack and pushing moves the top towards the start of the buffer. Anchoring at
/// the moving top instead of the fixed start keeps element accesses at constant offsets, and the
/// downward growth lets the length checks double as the bounds proofs for those accesses.
///
/// It is two words, so it is passed through the handler chain by value in registers, and it
/// borrows the buffer, so it cannot outlive it.
///
/// Conceptually this is the `&mut [u256]` covering the used suffix of the buffer. It is stored as
/// its raw parts because growing must move the pointer below the slice, which the provenance of a
/// reborrowed suffix does not allow; the pointer here retains the provenance of the full buffer it
/// was created from. All unsafe code is confined to this type; its soundness rests on the
/// invariant that `top` points `len` elements before the end of a buffer of `CAP` initialized
/// elements, exclusively borrowed for `'m`.
#[derive(Debug)]
pub struct SuffixStack<'m, const CAP: usize> {
    /// The top element; one past the buffer's last element when the stack is empty.
    top: NonNull<u256>,
    len: usize,
    _buffer: PhantomData<&'m mut [u256; CAP]>,
}

impl<'m, const CAP: usize> SuffixStack<'m, CAP> {
    pub fn new(buffer: &'m mut [u256; CAP]) -> Self {
        Self::with_len(buffer, 0)
    }

    /// The stack made up of the last `len` elements of the buffer, e.g. the one a previous borrow
    /// of the buffer left behind. Lengths beyond the capacity are clamped to it.
    pub fn with_len(buffer: &'m mut [u256; CAP], len: usize) -> Self {
        let len = min(len, CAP);
        let start = NonNull::from(buffer).cast::<u256>();
        Self {
            // SAFETY:
            // len <= CAP, so the top stays inside the buffer or one past its last element.
            top: unsafe { start.add(CAP - len) },
            len,
            _buffer: PhantomData,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The number of free slots below the top.
    pub fn free(&self) -> usize {
        CAP - self.len
    }

    /// The used part of the buffer, top of the stack first.
    pub fn as_slice(&self) -> &[u256] {
        // SAFETY:
        // The len elements starting at top are the used part of the exclusively borrowed buffer,
        // which is initialized in its entirety (invariant).
        unsafe { std::slice::from_raw_parts(self.top.as_ptr(), self.len) }
    }

    /// See [`SuffixStack::as_slice`].
    pub fn as_mut_slice(&mut self) -> &mut [u256] {
        // SAFETY:
        // As in as_slice, and the buffer is exclusively borrowed by self.
        unsafe { std::slice::from_raw_parts_mut(self.top.as_ptr(), self.len) }
    }

    /// The top of the stack.
    pub fn peek(&self) -> Option<&u256> {
        self.as_slice().first()
    }

    /// Iterates over the used part of the buffer, bottom of the stack first.
    pub fn iter_from_bottom(&self) -> impl Iterator<Item = &u256> {
        self.as_slice().iter().rev()
    }

    /// Grows the stack by one slot and returns that slot, which still holds its previous value,
    /// or [`None`] when the stack is full.
    pub fn grow(&mut self) -> Option<&mut u256> {
        if self.len == CAP {
            std::hint::cold_path();
            return None;
        }
        // SAFETY:
        // len < CAP, so there is a free slot directly below the top, and the pointer retains the
        // provenance of the full buffer, so it may be moved onto that slot.
        self.top = unsafe { self.top.sub(1) };
        self.len += 1;
        // SAFETY:
        // The slot is inside the exclusively borrowed, fully initialized buffer.
        Some(unsafe { &mut *self.top.as_ptr() })
    }

    /// Shrinks the stack by `n` slots, or returns [`None`] when it holds fewer than `n` elements.
    pub fn shrink(&mut self, n: usize) -> Option<()> {
        if self.len < n {
            std::hint::cold_path();
            return None;
        }
        // SAFETY:
        // n <= len, so the new top stays inside the buffer or one past its last element.
        self.top = unsafe { self.top.add(n) };
        self.len -= n;
        Some(())
    }
}

/// The interpreter stack.
pub type Stack<'m> = SuffixStack<'m, CAPACITY>;

impl<'m> Stack<'m> {
    /// A stack filled with `values`, where the last value is the top of the stack. Values beyond
    /// the stack capacity are ignored.
    pub fn new_with(buffer: &'m mut StackBuffer, values: &[u256]) -> Self {
        let mut stack = Self::new(buffer);
        for value in values.iter().take(CAPACITY) {
            // push cannot fail, at most CAPACITY values are pushed
            let _ = stack.push(*value);
        }
        stack
    }

    pub fn push(&mut self, value: impl Into<u256>) -> Result<(), FailStatus> {
        let Some(slot) = self.grow() else {
            return Err(FailStatus::StackOverflow);
        };
        *slot = value.into();
        Ok(())
    }

    pub fn swap_with_top<const N: usize>(&mut self) -> Result<(), FailStatus> {
        const { assert!(N > 0) };

        self.check_underflow(N + 1)?;

        // Swapping through two disjoint subslices instead of via [`slice::swap`] lets the
        // compiler shuffle the values in registers instead of copying them through the stack.
        let (top, rest) = self.as_mut_slice().split_first_mut().expect("len > N >= 1");
        std::mem::swap(top, &mut rest[N - 1]);

        Ok(())
    }

    /// Pops `N` entries from the stack and returns them as an array which is ordered such that the
    /// former top of stack is at the end of the array.
    pub fn pop<const N: usize>(&mut self) -> Result<[u256; N], FailStatus> {
        self.check_underflow(N)?;

        let slice = self.as_slice();
        let values = std::array::from_fn(|i| slice[N - 1 - i]);
        self.shrink(N).ok_or(FailStatus::StackUnderflow)?;
        Ok(values)
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
        let values = std::array::from_fn(|i| slice[N - 1 - i]);
        self.shrink(N - 1).ok_or(FailStatus::StackUnderflow)?;
        Ok((PushLocation(&mut self.as_mut_slice()[0]), values))
    }

    pub fn dup<const N: usize>(&mut self) -> Result<(), FailStatus> {
        // Note: N is 1 based (N = x -> duplicate element at index x-1)
        const { assert!(N > 0) };

        self.check_underflow(N)?;
        let value = self.as_slice()[N - 1];
        let Some(slot) = self.grow() else {
            return Err(FailStatus::StackOverflow);
        };
        *slot = value;
        Ok(())
    }

    #[inline(always)]
    pub fn check_overflow(&self, num_elements: usize) -> Result<(), FailStatus> {
        if self.free() < num_elements {
            std::hint::cold_path();
            return Err(FailStatus::StackOverflow);
        }
        Ok(())
    }

    #[inline(always)]
    fn check_underflow(&self, min_len: usize) -> Result<(), FailStatus> {
        if self.len() < min_len {
            std::hint::cold_path();
            return Err(FailStatus::StackUnderflow);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::types::{
        FailStatus,
        stack::{CAPACITY, Stack, StackBuffer},
        u256,
    };

    #[test]
    fn with_len() {
        let mut buffer = StackBuffer::new();
        Stack::new_with(&mut buffer, &[u256::ONE, u256::MAX]);
        let stack = Stack::with_len(&mut buffer, 1);
        assert_eq!(stack.as_slice(), [u256::ONE]);
        let stack = Stack::with_len(&mut buffer, 2);
        assert_eq!(stack.as_slice(), [u256::MAX, u256::ONE]);
        assert_eq!(Stack::with_len(&mut buffer, CAPACITY + 1).len(), CAPACITY);
    }

    #[test]
    fn grow_and_shrink() {
        let mut buffer = StackBuffer::new();
        let mut stack = Stack::new(&mut buffer);
        assert_eq!(stack.len(), 0);
        assert_eq!(stack.free(), CAPACITY);
        *stack.grow().unwrap() = u256::ONE;
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.as_slice(), [u256::ONE]);
        assert_eq!(stack.shrink(2), None);
        assert_eq!(stack.shrink(1), Some(()));
        assert!(stack.is_empty());
    }

    #[test]
    fn new_with() {
        let mut buffer = StackBuffer::new();
        let stack = Stack::new_with(&mut buffer, &[u256::ONE, u256::MAX]);
        assert_eq!(stack.len(), 2);
        assert_eq!(stack.peek(), Some(&u256::MAX));
        assert_eq!(stack.as_slice(), [u256::MAX, u256::ONE]);
        assert_eq!(
            stack.iter_from_bottom().copied().collect::<Vec<_>>(),
            [u256::ONE, u256::MAX]
        );
    }

    #[test]
    fn push() {
        let mut buffer = StackBuffer::new();
        let mut stack = Stack::new(&mut buffer);
        assert_eq!(stack.push(u256::MAX), Ok(()));
        assert_eq!(stack.as_slice(), [u256::MAX]);

        let mut buffer = StackBuffer::new();
        let mut stack = Stack::new_with(&mut buffer, &[u256::ZERO; CAPACITY]);
        assert_eq!(stack.push(u256::ZERO), Err(FailStatus::StackOverflow));
    }

    #[test]
    fn swap_with_top() {
        let mut buffer = StackBuffer::new();
        let mut stack = Stack::new_with(&mut buffer, &[u256::MAX, u256::ONE]);
        assert_eq!(stack.swap_with_top::<1>(), Ok(()));
        assert_eq!(stack.as_slice(), [u256::MAX, u256::ONE]);

        let mut buffer = StackBuffer::new();
        let mut stack = Stack::new_with(&mut buffer, &[u256::MAX, u256::ONE]);
        assert_eq!(stack.swap_with_top::<2>(), Err(FailStatus::StackUnderflow));
    }

    #[test]
    fn pop() {
        let mut buffer = StackBuffer::new();
        let mut stack = Stack::new_with(&mut buffer, &[u256::MAX]);
        assert_eq!(stack.pop::<1>(), Ok([u256::MAX]));
        assert_eq!(stack.len(), 0);

        let mut buffer = StackBuffer::new();
        let mut stack = Stack::new(&mut buffer);
        assert_eq!(stack.pop::<1>(), Err(FailStatus::StackUnderflow));

        let mut buffer = StackBuffer::new();
        let mut stack = Stack::new_with(&mut buffer, &[u256::ONE, u256::MAX]);
        assert_eq!(stack.pop::<2>(), Ok([u256::ONE, u256::MAX]));

        let mut buffer = StackBuffer::new();
        let mut stack = Stack::new_with(&mut buffer, &[u256::MAX]);
        assert_eq!(stack.pop::<2>(), Err(FailStatus::StackUnderflow));
    }

    #[test]
    fn pop_with_location() {
        let mut buffer = StackBuffer::new();
        let mut stack = Stack::new_with(&mut buffer, &[u256::MAX]);
        let (push_location, values) = stack.pop_with_location::<1>().unwrap();
        assert_eq!(values, [u256::MAX]);
        push_location.push(u256::ONE);
        assert_eq!(stack.as_slice(), [u256::ONE]);

        let mut buffer = StackBuffer::new();
        let mut stack = Stack::new(&mut buffer);
        assert_eq!(
            stack.pop_with_location::<1>().unwrap_err(),
            FailStatus::StackUnderflow
        );

        let mut buffer = StackBuffer::new();
        let mut stack = Stack::new_with(&mut buffer, &[u256::ONE, u256::MAX]);
        let (push_location, values) = stack.pop_with_location::<2>().unwrap();
        assert_eq!(values, [u256::ONE, u256::MAX]);
        push_location.push(u256::ZERO);
        assert_eq!(stack.as_slice(), [u256::ZERO]);

        let mut buffer = StackBuffer::new();
        let mut stack = Stack::new_with(&mut buffer, &[u256::MAX]);
        assert_eq!(
            stack.pop_with_location::<2>().unwrap_err(),
            FailStatus::StackUnderflow
        );
    }

    #[test]
    fn dup() {
        let mut buffer = StackBuffer::new();
        let mut stack = Stack::new_with(&mut buffer, &[u256::MAX, u256::ZERO]);
        stack.dup::<1>().unwrap();
        assert_eq!(stack.as_slice(), [u256::ZERO, u256::ZERO, u256::MAX]);

        let mut buffer = StackBuffer::new();
        let mut stack = Stack::new_with(&mut buffer, &[u256::MAX, u256::ZERO]);
        stack.dup::<2>().unwrap();
        assert_eq!(stack.as_slice(), [u256::MAX, u256::ZERO, u256::MAX]);

        let mut buffer = StackBuffer::new();
        let mut stack = Stack::new_with(&mut buffer, &[u256::MAX, u256::ZERO]);
        assert_eq!(stack.dup::<3>(), Err(FailStatus::StackUnderflow));

        let mut buffer = StackBuffer::new();
        let mut stack = Stack::new_with(&mut buffer, &[u256::ZERO; CAPACITY]);
        assert_eq!(stack.dup::<1>(), Err(FailStatus::StackOverflow));
    }

    #[test]
    fn check_overflow() {
        let mut buffer = StackBuffer::new();
        let stack = Stack::new_with(&mut buffer, &[u256::ZERO; CAPACITY - 1]);
        assert_eq!(stack.check_overflow(1), Ok(()));
        assert_eq!(stack.check_overflow(2), Err(FailStatus::StackOverflow));

        let mut buffer = StackBuffer::new();
        let stack = Stack::new_with(&mut buffer, &[u256::ZERO; CAPACITY]);
        assert_eq!(stack.check_overflow(0), Ok(()));
        assert_eq!(stack.check_overflow(1), Err(FailStatus::StackOverflow));
    }

    #[test]
    fn check_underflow() {
        let mut buffer = StackBuffer::new();
        assert_eq!(Stack::new(&mut buffer).check_underflow(0), Ok(()));

        let stack = Stack::new_with(&mut buffer, &[u256::ZERO]);
        assert_eq!(stack.check_underflow(1), Ok(()));
        assert_eq!(stack.check_underflow(2), Err(FailStatus::StackUnderflow));
    }
}
