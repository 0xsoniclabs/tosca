#[cfg(feature = "alloc-reuse")]
use std::sync::Mutex;

use crate::types::{Memory, StackBuffer};

/// The allocations of finished interpreter runs, kept as [`StackBuffer`] and [`Memory`] pairs so
/// that a run only has to acquire a single lock once when it is created and once when it is
/// dropped.
#[cfg(feature = "alloc-reuse")]
static REUSABLE_INTERPRETER_ALLOCATIONS: Mutex<Vec<(StackBuffer, Memory)>> = Mutex::new(Vec::new());

/// Returns a [`StackBuffer`] and an empty [`Memory`], reusing the allocations of a finished run
/// when one is available.
#[inline(always)]
pub fn new_stack_buffer_and_memory() -> (StackBuffer, Memory) {
    #[cfg(feature = "alloc-reuse")]
    {
        if let Some((stack_buffer, mut memory)) =
            REUSABLE_INTERPRETER_ALLOCATIONS.lock().unwrap().pop()
        {
            memory.reset_to(&[]);
            return (stack_buffer, memory);
        }
        std::hint::cold_path();
    }
    (StackBuffer::new(), Memory::new())
}

/// Puts the [`StackBuffer`] and the [`Memory`] back into the reuse cache so that a later run can
/// take over their allocations.
pub fn release_stack_buffer_and_memory(stack_buffer: StackBuffer, memory: Memory) {
    std::cfg_select! {
        feature = "alloc-reuse" => REUSABLE_INTERPRETER_ALLOCATIONS
            .lock()
            .unwrap()
            .push((stack_buffer, memory)),
        _ => drop((stack_buffer, memory)),
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_stack_buffer_and_memory_returns_empty_memory() {
        let (stack_buffer, mut memory) = new_stack_buffer_and_memory();
        assert_eq!(memory.len(), 0);

        // ensure the memory is not empty
        memory.reset_to(&[1]);
        release_stack_buffer_and_memory(stack_buffer, memory);

        let (_, memory) = new_stack_buffer_and_memory(); // reused
        assert_eq!(memory.len(), 0);
    }
}
