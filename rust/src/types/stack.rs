use std::cmp::min;
#[cfg(feature = "alloc-reuse")]
use std::sync::Mutex;

use crate::types::{FailStatus, u256};

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

#[cfg(feature = "alloc-reuse")]
static REUSABLE_STACK: Mutex<Vec<Vec<u256>>> = Mutex::new(Vec::new());

#[derive(Debug)]
pub struct Stack(Vec<u256>);

#[cfg(feature = "alloc-reuse")]
impl Drop for Stack {
    fn drop(&mut self) {
        REUSABLE_STACK
            .lock()
            .unwrap()
            .push(std::mem::take(&mut self.0));
    }
}

impl Stack {
    const CAPACITY: usize = 1024;

    pub fn new(inner: &[u256]) -> Self {
        let len = min(inner.len(), Self::CAPACITY);
        let inner = &inner[..len];
        let mut v = std::cfg_select! {
            feature = "alloc-reuse" => REUSABLE_STACK
                .lock()
                .unwrap()
                .pop()
                .unwrap_or_else(|| Vec::with_capacity(Self::CAPACITY)),
            _ => Vec::with_capacity(Self::CAPACITY),
        };
        v.clear();
        #[cfg(feature = "unsafe-stack")]
        // SAFETY:
        // v was either created with capacity at least Self::CAPACITY or taken from REUSABLE_STACK,
        // where Stack::drop only puts vectors that were created that way and are never shrunk.
        // inner is truncated to at most Self::CAPACITY elements.
        // The length bound is part of the hint because the reallocation in extend_from_slice is
        // only elided when both operands of the comparison are known.
        unsafe {
            std::hint::assert_unchecked(
                v.capacity() >= Self::CAPACITY && inner.len() <= Self::CAPACITY,
            );
        }
        v.extend_from_slice(inner);
        Self(v)
    }

    pub fn as_slice(&self) -> &[u256] {
        self.0.as_slice()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn push(&mut self, value: impl Into<u256>) -> Result<(), FailStatus> {
        self.check_overflow(1)?;
        #[cfg(feature = "unsafe-stack")]
        // SAFETY:
        // self.0's capacity is at least Self::CAPACITY and never shrinks, and check_overflow
        // guarantees that the length is below Self::CAPACITY.
        unsafe {
            std::hint::assert_unchecked(
                self.0.capacity() >= Self::CAPACITY && self.0.len() < Self::CAPACITY,
            );
        }
        self.0.push(value.into());
        Ok(())
    }

    pub fn swap_with_top<const N: usize>(&mut self) -> Result<(), FailStatus> {
        const { assert!(N > 0) };

        self.check_underflow(N + 1)?;

        // Swapping through two disjoint subslices instead of via [`slice::swap`] lets the
        // compiler shuffle the values in registers instead of copying them through the stack.
        let len = self.0.len();
        let (rest, top) = self.0.split_at_mut(len - 1);
        std::mem::swap(&mut rest[len - 1 - N], &mut top[0]);

        Ok(())
    }

    /// Pops `N` entries from the stack and returns them as an array which is ordered such that the
    /// former top of stack is at the end of the array.
    pub fn pop<const N: usize>(&mut self) -> Result<[u256; N], FailStatus> {
        self.check_underflow(N)?;

        let new_len = self.0.len() - N;
        let array = *self.0[new_len..].as_array().unwrap();
        self.0.truncate(new_len);
        Ok(array)
    }

    /// Pops `N` entries from the stack, ordered like [`Stack::pop`], and returns a
    /// [`PushLocation`] for the slot the result must be written to. That slot is already
    /// accounted for in the stack's length.
    pub fn pop_with_location<const N: usize>(
        &'_ mut self,
    ) -> Result<(PushLocation<'_>, [u256; N]), FailStatus> {
        const { assert!(N > 0) };

        self.check_underflow(N)?;

        let len = self.len();
        let pop_data = *self.0[len - N..].as_array().unwrap();
        self.0.truncate(len - (N - 1));
        let push_location = PushLocation(&mut self.0[len - N]);
        Ok((push_location, pop_data))
    }

    pub fn peek(&self) -> Option<&u256> {
        self.0.last()
    }

    pub fn dup<const N: usize>(&mut self) -> Result<(), FailStatus> {
        // Note: N is 1 based (N = x -> duplicate element at index x-1)
        const { assert!(N > 0) };

        self.check_underflow(N)?;
        let element = self.0[self.0.len() - N];
        self.push(element)
    }

    #[inline(always)]
    pub fn check_overflow(&self, num_elements: usize) -> Result<(), FailStatus> {
        // len <= CAPACITY (invariant), so this does not underflow
        if Self::CAPACITY - self.0.len() < num_elements {
            std::hint::cold_path();
            return Err(FailStatus::StackOverflow);
        }
        Ok(())
    }

    #[inline(always)]
    fn check_underflow(&self, min_len: usize) -> Result<(), FailStatus> {
        if self.0.len() < min_len {
            std::hint::cold_path();
            return Err(FailStatus::StackUnderflow);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::types::{FailStatus, stack::Stack, u256};

    #[test]
    fn internals() {
        let stack = Stack::new(&[u256::ONE]);
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.as_slice(), &[u256::ONE]);
    }

    #[test]
    fn push() {
        let mut stack = Stack::new(&[]);
        assert_eq!(stack.push(u256::MAX), Ok(()));
        assert_eq!(stack.as_slice(), [u256::MAX]);

        let mut stack = Stack::new(&[u256::ZERO; Stack::CAPACITY]);
        assert_eq!(stack.push(u256::ZERO), Err(FailStatus::StackOverflow));
    }

    #[test]
    fn pop() {
        let mut stack = Stack::new(&[u256::MAX]);
        assert_eq!(stack.pop::<1>(), Ok([u256::MAX]));

        let mut stack = Stack::new(&[]);
        assert_eq!(stack.pop::<1>(), Err(FailStatus::StackUnderflow));

        let mut stack = Stack::new(&[u256::ONE, u256::MAX]);
        assert_eq!(stack.pop::<2>(), Ok([u256::ONE, u256::MAX]));

        let mut stack = Stack::new(&[u256::MAX]);
        assert_eq!(stack.pop::<2>(), Err(FailStatus::StackUnderflow));
    }

    #[test]
    fn pop_with_location() {
        let mut stack = Stack::new(&[u256::MAX]);
        let (push_location, data) = stack.pop_with_location::<1>().unwrap();
        assert_eq!(data, [u256::MAX]);
        push_location.push(u256::ONE);
        assert_eq!(stack.as_slice(), [u256::ONE]);

        let mut stack = Stack::new(&[]);
        assert_eq!(
            stack.pop_with_location::<1>().unwrap_err(),
            FailStatus::StackUnderflow
        );

        let mut stack = Stack::new(&[u256::ONE, u256::MAX]);
        let (push_location, data) = stack.pop_with_location::<2>().unwrap();
        assert_eq!(data, [u256::ONE, u256::MAX]);
        push_location.push(u256::ZERO);
        assert_eq!(stack.as_slice(), [u256::ZERO]);

        let mut stack = Stack::new(&[u256::MAX]);
        assert_eq!(
            stack.pop_with_location::<2>().unwrap_err(),
            FailStatus::StackUnderflow
        );
    }

    #[test]
    fn dup() {
        let mut stack = Stack::new(&[u256::MAX, u256::ZERO]);
        stack.dup::<1>().unwrap();
        assert_eq!(stack.as_slice(), [u256::MAX, u256::ZERO, u256::ZERO]);

        let mut stack = Stack::new(&[u256::MAX, u256::ZERO]);
        stack.dup::<2>().unwrap();
        assert_eq!(stack.as_slice(), [u256::MAX, u256::ZERO, u256::MAX]);

        let mut stack = Stack::new(&[u256::MAX, u256::ZERO]);
        assert_eq!(stack.dup::<3>(), Err(FailStatus::StackUnderflow));

        let mut stack = Stack::new(&[u256::ZERO; 1024]);
        assert_eq!(stack.dup::<1>(), Err(FailStatus::StackOverflow));
    }

    #[test]
    fn swap_with_top() {
        let mut stack = Stack::new(&[u256::MAX, u256::ONE]);
        assert_eq!(stack.swap_with_top::<1>(), Ok(()));
        assert_eq!(stack.as_slice(), [u256::ONE, u256::MAX]);

        let mut stack = Stack::new(&[u256::MAX, u256::ONE]);
        assert_eq!(stack.swap_with_top::<2>(), Err(FailStatus::StackUnderflow));
    }

    #[test]
    fn check_overflow() {
        let stack = Stack::new(&[u256::ZERO; Stack::CAPACITY - 1]);
        assert_eq!(stack.check_overflow(1), Ok(()));
        assert_eq!(stack.check_overflow(2), Err(FailStatus::StackOverflow));
        let stack = Stack::new(&[u256::ZERO; Stack::CAPACITY]);
        assert_eq!(stack.check_overflow(0), Ok(()));
        assert_eq!(stack.check_overflow(1), Err(FailStatus::StackOverflow));
    }

    #[test]
    fn check_underflow() {
        let stack = Stack::new(&[]);
        assert_eq!(stack.check_underflow(0), Ok(()));
        let stack = Stack::new(&[u256::ZERO]);
        assert_eq!(stack.check_underflow(1), Ok(()));
        assert_eq!(stack.check_underflow(2), Err(FailStatus::StackUnderflow));
    }
}
