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

    #[test]
    fn reused_pair_is_empty() {
        let (mut stack, mut memory) = new_stack_and_memory();
        stack.reset_to(&[1u8.into()]);
        memory.reset_to(&[1]);
        release_stack_and_memory(stack, memory);
        let (stack, memory) = new_stack_and_memory();
        assert!(stack.as_slice().is_empty());
        assert!(memory.as_slice().is_empty());
    }
}
