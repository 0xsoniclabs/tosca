use std::{borrow::Cow, io::Write};

#[cfg(feature = "fn-ptr-conversion-dispatch")]
use crate::Opcode;
use crate::interpreter::Interpreter;

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
        // pre_op is called after the op is fetched so this will always be Ok(..)
        let op = std::cfg_select! {
            feature = "fn-ptr-conversion-dispatch" => {{
                let op = interpreter.code_reader[interpreter.code_reader.pc()];
                // SAFETY:
                // pre_op is called after the op is fetched, which means that code_reader.get()
                // returned Some(..) which in turn means that the code analysis determined that this
                // byte is a valid Opcode.
                unsafe { std::mem::transmute::<u8, Opcode>(op) }
            }}
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
