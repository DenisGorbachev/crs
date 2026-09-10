use errgonomic::handle;
use serde::Serialize;
use std::io::{self, Write};
use thiserror::Error;

/// Writes a serializable value as one JSONL record, retaining the value on failure.
pub fn write_jsonl<T: Serialize>(writer: &mut (impl Write + ?Sized), value: T) -> Result<(), WriteJsonlError<T>> {
    use WriteJsonlError::*;
    handle!(serde_json::to_writer(&mut *writer, &value), ToWriterFailed, value);
    handle!(writer.write_all(b"\n"), WriteAllFailed, value);
    Ok(())
}

#[derive(Error, Debug)]
pub enum WriteJsonlError<T> {
    #[error("failed to serialize a value as JSON")]
    ToWriterFailed { source: serde_json::Error, value: Box<T> },
    #[error("failed to write the JSONL separator")]
    WriteAllFailed { source: io::Error, value: Box<T> },
}
