use std::{borrow::Cow, io::Write};

use crate::interpreter::Interpreter;

/// Hooks that the interpreter calls during execution, e.g. for tracing.
pub trait Observer<const STEPPABLE: bool> {
    /// Called before the op at the current program counter is executed.
    fn pre_op(&mut self, interpreter: &Interpreter<STEPPABLE>);

    /// Called after an op was executed successfully.
    fn post_op(&mut self, interpreter: &Interpreter<STEPPABLE>);

    /// Called with a free-form message from the interpreter.
    fn log(&mut self, message: Cow<str>);
}

/// An [`Observer`] that ignores all events.
pub struct NoOpObserver();

impl<const STEPPABLE: bool> Observer<STEPPABLE> for NoOpObserver {
    fn pre_op(&mut self, _interpreter: &Interpreter<STEPPABLE>) {}

    fn post_op(&mut self, _interpreter: &Interpreter<STEPPABLE>) {}

    fn log(&mut self, _message: Cow<str>) {}
}

/// An [`Observer`] that writes the op, the gas left and the top of the stack before each op and
/// all log messages to `writer`, one line per event.
pub struct LoggingObserver<W: Write> {
    writer: W,
}

impl<W: Write> LoggingObserver<W> {
    /// Creates a new [`LoggingObserver`] that writes to `writer`.
    pub fn new(writer: W) -> Self {
        Self { writer }
    }
}

impl<W: Write, const STEPPABLE: bool> Observer<STEPPABLE> for LoggingObserver<W> {
    fn pre_op(&mut self, interpreter: &Interpreter<STEPPABLE>) {
        let op = std::cfg_select! {
            feature = "fn-ptr-conversion-dispatch" => {
                {
                    // The terminator entry past the end of the code is not an op, so don't log it.
                    let Some(&op) = interpreter.code_reader[..].get(interpreter.code_reader.pc())
                    else {
                        return;
                    };
                    op
                }
            }
            // pre_op is called after the op is fetched so this will always be Ok(..)
            _ => interpreter.code_reader.get().unwrap(),
        };
        let gas = interpreter.gas_left.as_u64();
        let top = std::fmt::from_fn(|f| match interpreter.stack.peek() {
            Some(top) => write!(f, "{top}"),
            None => f.write_str("-empty-"),
        });
        writeln!(
            self.writer,
            "op: {op:#04x}, gas left: {gas}, top of stack: {top}"
        )
        .unwrap();
        self.writer.flush().unwrap();
    }

    fn post_op(&mut self, _interpreter: &Interpreter<STEPPABLE>) {}

    fn log(&mut self, message: Cow<str>) {
        writeln!(self.writer, "{message}").unwrap();
        self.writer.flush().unwrap();
    }
}

/// Selects the [`Observer`] the interpreter runs with.
#[derive(Debug, Clone, Copy)]
pub enum ObserverType {
    NoOp,
    Logging,
}

#[cfg(test)]
mod tests {
    use evmc_vm::Revision;
    use rstest::rstest;

    use super::*;
    use crate::{
        Opcode,
        types::{
            CodeAnalysisCache, MockExecutionContextTrait, MockExecutionMessage,
            hash_cache::HashCache, u256,
        },
    };

    #[rstest]
    #[case::empty_stack(&[Opcode::Add as u8], &[], "op: 0x01, gas left: 100, top of stack: -empty-\n")]
    #[case::top_of_stack(&[Opcode::Add as u8], &[u256::ONE, u256::from(2u8)], "op: 0x01, gas left: 100, top of stack: 2\n")]
    // Only fn-ptr dispatch reaches pre_op for invalid bytes.
    #[cfg_attr(
        feature = "fn-ptr-conversion-dispatch",
        case::invalid(&[0x0c], &[], "op: 0x0c, gas left: 100, top of stack: -empty-\n")
    )]
    fn logging_observer_pre_op_writes_op_gas_and_top_of_stack(
        #[case] code: &[u8],
        #[case] stack: &[u256],
        #[case] expected: &str,
    ) {
        let code_analysis_cache = CodeAnalysisCache::default();
        let hash_cache = HashCache::default();
        let mut context = MockExecutionContextTrait::new();
        let message = MockExecutionMessage {
            gas: 100,
            ..Default::default()
        }
        .into();
        let mut interpreter = Interpreter::new(
            Revision::EVMC_ISTANBUL,
            &message,
            &mut context,
            code,
            &code_analysis_cache,
            &hash_cache,
        );
        interpreter.stack.reset_to(stack);

        let mut observer = LoggingObserver::new(Vec::new());
        observer.pre_op(&interpreter);
        assert_eq!(String::from_utf8(observer.writer).unwrap(), expected);
    }

    #[test]
    fn logging_observer_log_writes_message_as_line() {
        let mut observer = LoggingObserver::new(Vec::new());
        Observer::<false>::log(&mut observer, "a".into());
        Observer::<false>::log(&mut observer, "b".into());
        assert_eq!(String::from_utf8(observer.writer).unwrap(), "a\nb\n");
    }
}
