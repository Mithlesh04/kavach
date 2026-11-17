//! The system clipboard for `std.clipboard` (spec 14.17, decision 0066),
//! through `arboard`, for the interpreter and native code alike.

/// Why the clipboard couldn't be read or written: `ClipboardError` in
/// `std.clipboard`.
#[derive(Debug, PartialEq)]
pub enum ClipError {
    Empty,
    Unavailable(String),
}

/// The program's one clipboard, opened when first used. On Linux the text
/// a program writes lives in the program itself (X11 and Wayland ask the
/// owner for it), so the clipboard is kept open while the program runs;
/// opened per call, the text was gone as soon as `write_text` returned.
#[cfg(all(feature = "devices", not(target_os = "macos")))]
fn with<T>(
    f: impl FnOnce(&mut arboard::Clipboard) -> Result<T, ClipError>,
) -> Result<T, ClipError> {
    use std::sync::Mutex;
    static ONE: Mutex<Option<arboard::Clipboard>> = Mutex::new(None);
    let mut one = ONE.lock().unwrap_or_else(|e| e.into_inner());
    if one.is_none() {
        *one = Some(arboard::Clipboard::new().map_err(|e| ClipError::Unavailable(e.to_string()))?);
    }
    match one.as_mut() {
        Some(c) => f(c),
        None => Err(ClipError::Unavailable("no clipboard".to_string())),
    }
}

/// On macOS the system keeps the text, and the pasteboard's handle can't
/// be kept in a `static`: opened per call.
#[cfg(all(feature = "devices", target_os = "macos"))]
fn with<T>(
    f: impl FnOnce(&mut arboard::Clipboard) -> Result<T, ClipError>,
) -> Result<T, ClipError> {
    let mut c = arboard::Clipboard::new().map_err(|e| ClipError::Unavailable(e.to_string()))?;
    f(&mut c)
}

/// The clipboard's text.
#[cfg(feature = "devices")]
pub fn read_text() -> Result<String, ClipError> {
    with(|c| match c.get_text() {
        Ok(text) => Ok(text),
        Err(arboard::Error::ContentNotAvailable) => Err(ClipError::Empty),
        Err(e) => Err(ClipError::Unavailable(e.to_string())),
    })
}

/// Puts `text` on the clipboard.
#[cfg(feature = "devices")]
pub fn write_text(text: &str) -> Result<(), ClipError> {
    with(|c| {
        c.set_text(text)
            .map_err(|e| ClipError::Unavailable(e.to_string()))
    })
}

/// The clipboard's text.
#[cfg(not(feature = "devices"))]
pub fn read_text() -> Result<String, ClipError> {
    Err(ClipError::Unavailable(
        "built without clipboard support".to_string(),
    ))
}

/// Puts `text` on the clipboard.
#[cfg(not(feature = "devices"))]
pub fn write_text(_text: &str) -> Result<(), ClipError> {
    Err(ClipError::Unavailable(
        "built without clipboard support".to_string(),
    ))
}
