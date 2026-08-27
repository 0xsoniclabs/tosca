use std::{borrow::Cow, io::Write};

#[cfg(feature = "fn-ptr-conversion-dispatch")]
use crate::Opcode;
use crate::interpreter::Interpreter;
#[cfg(feature = "fn-ptr-conversion-dispatch")]
use crate::types::{CodeByteType, code_byte_type};

pub trait Observer<const STEPPABLE: bool> {
    fn pre_op(&mut self, interpreter: &Interpreter<STEPPABLE>);

    fn post_op(&mut self, interpreter: &Interpreter<STEPPABLE>);

    fn log(&mut self, message: Cow<str>);
}

pub struct NoOpObserver();

impl<const STEPPABLE: bool> Observer<STEPPABLE> for NoOpObserver {
    fn pre_op(&mut self, _interpreter: &Interpreter<STEPPABLE>) {}

    fn post_op(&mut self, _interpreter: &Interpreter<STEPPABLE>) {}

    fn log(&mut self, _message: Cow<str>) {}
}

pub struct LoggingObserver<W: Write> {
    writer: W,
}

impl<W: Write> LoggingObserver<W> {
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
                    // Data and invalid bytes are dispatched to the Opcode::Invalid handler, so
                    // pre_op is reached for them, but they have no Opcode variant to log.
                    if code_byte_type(op).0 == CodeByteType::DataOrInvalid {
                        return;
                    }
                    // SAFETY:
                    // Every other code byte type is a byte the Opcode enum has a variant for.
                    unsafe { std::mem::transmute::<u8, Opcode>(op) }
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
        writeln!(self.writer, "{op:?}, {gas}, {top}").unwrap();
        self.writer.flush().unwrap();
    }

    fn post_op(&mut self, _interpreter: &Interpreter<STEPPABLE>) {}

    fn log(&mut self, message: Cow<str>) {
        writeln!(self.writer, "{message}").unwrap();
        self.writer.flush().unwrap();
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ObserverType {
    NoOp,
    Logging,
}
