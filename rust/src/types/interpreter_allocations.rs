#[cfg(feature = "alloc-reuse")]
use std::sync::Mutex;

use crate::types::{Memory, Stack};

/// The allocations of finished interpreter runs, kept as [`Stack`] and [`Memory`] pairs so that a
/// run only has to acquire a single lock once when it is created and once when it is dropped.
#[cfg(feature = "alloc-reuse")]
static REUSABLE_INTERPRETER_ALLOCATIONS: Mutex<Vec<(Stack, Memory)>> = Mutex::new(Vec::new());

/// Returns an empty [`Stack`] and [`Memory`], reusing the allocations of a finished run when one is
/// available.
#[inline(always)]
pub fn new_stack_and_memory() -> (Stack, Memory) {
    #[cfg(feature = "alloc-reuse")]
    {
        if let Some((mut stack, mut memory)) =
            REUSABLE_INTERPRETER_ALLOCATIONS.lock().unwrap().pop()
        {
            stack.reset_to(&[]);
            memory.reset_to(&[]);
            return (stack, memory);
        }
        std::hint::cold_path();
    }
    (Stack::new(), Memory::new())
}

/// Puts the [`Stack`] and the [`Memory`] back into the reuse cache so that a later run can take
/// over their allocations.
pub fn release_stack_and_memory(stack: Stack, memory: Memory) {
    std::cfg_select! {
        feature = "alloc-reuse" => REUSABLE_INTERPRETER_ALLOCATIONS
            .lock()
            .unwrap()
            .push((stack, memory)),
        _ => drop((stack, memory)),
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::u256;

    #[test]
    fn new_stack_and_memory_returns_empty_stack_and_memory() {
        let (mut stack, mut memory) = new_stack_and_memory();
        assert_eq!(stack.len(), 0);
        assert_eq!(memory.len(), 0);

        // ensure the stack and memory are not empty
        stack.reset_to(&[u256::ONE]);
        memory.reset_to(&[1]);
        release_stack_and_memory(stack, memory);

        let (stack, memory) = new_stack_and_memory(); // reused
        assert_eq!(stack.len(), 0);
        assert_eq!(memory.len(), 0);
    }

    #[cfg(feature = "alloc-reuse")]
    #[test]
    fn new_stack_and_memory_reuses_allocations() {
        let (mut stack, mut memory) = new_stack_and_memory();
        // ensure the stack and memory are not empty so that the backing Vecs are allocated
        stack.reset_to(&[u256::ONE]);
        memory.reset_to(&[1]);
        let stack_ptr = stack.as_slice().as_ptr();
        let memory_ptr = memory.as_slice().as_ptr();

        release_stack_and_memory(stack, memory);

        let (mut stack, mut memory) = new_stack_and_memory(); // reused
        // ensure the stack and memory are not empty so the slices point to the same backing Vecs as
        // before
        stack.reset_to(&[u256::ONE]);
        memory.reset_to(&[1]);
        assert_eq!(stack.as_slice().as_ptr(), stack_ptr);
        assert_eq!(memory.as_slice().as_ptr(), memory_ptr);
    }
}
