use crate::MARKDOWN_SEPARATOR;
use errgonomic::handle;
use std::io::{self, Write};
use thiserror::Error;

pub fn iter_write_as_markdown<'a>(writer: &mut (impl Write + ?Sized), iter: impl IntoIterator<Item = &'a str>) -> Result<(), IterWriteAsMarkdownError> {
    use IterWriteAsMarkdownError::*;
    for (index, item) in iter.into_iter().enumerate() {
        if index != 0 {
            handle!(writer.write_all(MARKDOWN_SEPARATOR.as_bytes()), WriteSeparator);
        }
        handle!(writer.write_all(item.as_bytes()), WriteAllFailed, item);
    }
    Ok(())
}

#[derive(Error, Debug)]
pub enum IterWriteAsMarkdownError {
    #[error("failed to write Markdown separator")]
    WriteSeparator { source: io::Error },
    #[error("failed to write Markdown item")]
    WriteAllFailed { source: io::Error, item: String },
}
