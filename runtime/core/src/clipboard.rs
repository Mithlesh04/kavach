//! The system clipboard for `std.clipboard` (spec 14.17, decision 0066),
//! through `arboard`, for the interpreter and native code alike.

/// Why the clipboard couldn't be read or written: `ClipboardError` in
/// `std.clipboard`.
#[derive(Debug, PartialEq)]
pub enum ClipError {
    Empty,
    Unavailable(String),
}

fn open() -> Result<arboard::Clipboard, ClipError> {
    arboard::Clipboard::new().map_err(|e| ClipError::Unavailable(e.to_string()))
}

/// The clipboard's text.
pub fn read_text() -> Result<String, ClipError> {
    match open()?.get_text() {
        Ok(text) => Ok(text),
        Err(arboard::Error::ContentNotAvailable) => Err(ClipError::Empty),
        Err(e) => Err(ClipError::Unavailable(e.to_string())),
    }
}

/// Puts `text` on the clipboard.
pub fn write_text(text: &str) -> Result<(), ClipError> {
    open()?
        .set_text(text)
        .map_err(|e| ClipError::Unavailable(e.to_string()))
}
