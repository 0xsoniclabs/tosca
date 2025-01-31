use std::{borrow::Cow, io::Write};

use crate::{interpreter::Interpreter, Opcode};

pub trait Observer<const STEPPABLE: bool> {
    fn pre_op(&mut self, _interpreter: &Interpreter<STEPPABLE>) {}

    fn post_op(&mut self, _interpreter: &Interpreter<STEPPABLE>) {}

    fn log(&mut self, _message: Cow<str>) {}
}

pub struct NoOpObserver();

impl<const STEPPABLE: bool> Observer<STEPPABLE> for NoOpObserver {}

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
        #[cfg(not(feature = "needs-fn-ptr-conversion"))]
        let op = interpreter.code_reader.get().unwrap();
        #[cfg(feature = "needs-fn-ptr-conversion")]
        let op = {
            let op = interpreter.code_reader[interpreter.code_reader.pc()];
            // SAFETY:
            // pre_op is called after the op is fetched, which means that code_reader.get() returned
            // Some(..) which in turn means that the code analysis determined that this byte is a
            // valid Opcode.
            unsafe { std::mem::transmute::<u8, Opcode>(op) }
        };
        let gas = interpreter.gas_left.as_u64();
        let top = interpreter
            .stack
            .peek()
            .map(ToString::to_string)
            .unwrap_or("-empty-".to_owned());
        writeln!(self.writer, "{op:?}, {gas}, {top}").unwrap();
        self.writer.flush().unwrap();
    }

    fn log(&mut self, message: Cow<str>) {
        writeln!(self.writer, "{message}").unwrap();
        self.writer.flush().unwrap();
    }
}

pub struct CountObserver<W: Write> {
    writer: W,
    op_count: u64,
}

impl<W: Write> CountObserver<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            op_count: 0,
        }
    }
}

impl<W: Write> Drop for CountObserver<W> {
    fn drop(&mut self) {
        writeln!(self.writer, "{}", self.op_count).unwrap();
        self.writer.flush().unwrap();
    }
}

impl<W: Write, const STEPPABLE: bool> Observer<STEPPABLE> for CountObserver<W> {
    fn pre_op(&mut self, _interpreter: &Interpreter<STEPPABLE>) {
        self.op_count += 1;
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ObserverType {
    NoOp,
    Logging,
    Counts,
}
