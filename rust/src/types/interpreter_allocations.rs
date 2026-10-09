#[cfg(feature = "alloc-reuse")]
use std::sync::Mutex;

use crate::types::{Memory, Stack, StackBuffer};

/// The allocations of finished interpreter runs, kept as [`StackBuffer`] and [`Memory`] pairs so
/// that a run only has to acquire a single lock once before it starts and once after it ends.
#[derive(Default)]
pub struct InterpreterAllocationsCache(
    #[cfg(feature = "alloc-reuse")] Mutex<Vec<(Box<StackBuffer>, Memory)>>,
);

impl InterpreterAllocationsCache {
    /// Returns a [`StackBuffer`] and a [`Memory`], reusing the allocations of a finished run when
    /// one is available. Both may still hold the content of that run.
    #[inline(always)]
    #[allow(clippy::unused_self)]
    pub fn new_stack_and_memory(&self) -> (Box<StackBuffer>, Memory) {
        #[cfg(feature = "alloc-reuse")]
        {
            if let Some(allocations) = self.0.lock().unwrap().pop() {
                return allocations;
            }
            std::hint::cold_path();
        }
        (Stack::new_buffer(), Memory::new())
    }

    /// Puts the [`StackBuffer`] and the [`Memory`] back so that a later run can take over their
    /// allocations, or frees them without `alloc-reuse`.
    #[allow(clippy::unused_self)]
    pub fn release_stack_and_memory(&self, stack_buffer: Box<StackBuffer>, memory: Memory) {
        std::cfg_select! {
            feature = "alloc-reuse" => self.0.lock().unwrap().push((stack_buffer, memory)),
            _ => drop((stack_buffer, memory)),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::u256;

    #[test]
    fn new_stack_and_memory_takes_reusable_allocations_or_allocates_new_ones() {
        let (stack_buffer, memory) = InterpreterAllocationsCache::default().new_stack_and_memory();
        assert_eq!(stack_buffer[0], u256::ZERO);
        assert_eq!(memory.as_slice(), []);

        #[cfg(feature = "alloc-reuse")]
        {
            let mut stack_buffer = Stack::new_buffer();
            stack_buffer.fill(u256::ONE);
            let mut memory = Memory::new();
            memory.reset_to(&[1]);
            let allocations = InterpreterAllocationsCache(Mutex::new(vec![(stack_buffer, memory)]));

            let (stack_buffer, memory) = allocations.new_stack_and_memory();
            assert_eq!(stack_buffer[0], u256::ONE);
            assert_eq!(memory.as_slice(), [1]);
            assert!(allocations.0.lock().unwrap().is_empty());
        }
    }

    #[cfg(feature = "alloc-reuse")]
    #[test]
    fn release_stack_and_memory_keeps_allocations_for_reuse() {
        let allocations = InterpreterAllocationsCache::default();
        let mut stack_buffer = Stack::new_buffer();
        stack_buffer.fill(u256::ONE);
        let mut memory = Memory::new();
        memory.reset_to(&[1]);

        allocations.release_stack_and_memory(stack_buffer, memory);
        let reusable = allocations.0.into_inner().unwrap();
        assert_eq!(reusable.len(), 1);
        assert_eq!(reusable[0].0[0], u256::ONE);
        assert_eq!(reusable[0].1.as_slice(), [1]);
    }
}
