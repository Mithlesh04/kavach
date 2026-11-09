//! `file://` URIs, as editors send them, to paths and back.

/// The path a `file://` URI names; `None` for other schemes (an unsaved
/// editor tab) or bad escapes. `file:///C:/a.kav` is `C:/a.kav`.
pub fn to_path(uri: &str) -> Option<String> {
    let rest = uri.strip_prefix("file://")?;
    let path = decode(rest)?;
    let b = path.as_bytes();
    if b.len() >= 3 && b[0] == b'/' && b[1].is_ascii_alphabetic() && b[2] == b':' {
        return Some(path[1..].to_string());
    }
    if !path.starts_with('/') {
        // `file://server/share/a.kav`: a network path.
        return Some(format!("//{path}"));
    }
    Some(path)
}

/// The `file://` URI of a path.
pub fn from_path(path: &str) -> String {
    let path = path.replace('\\', "/");
    let mut uri = String::from("file://");
    if !path.starts_with('/') {
        uri.push('/');
    }
    for b in path.trim_start_matches("//").bytes() {
        if b.is_ascii_alphanumeric() || b"/-._~:".contains(&b) {
            uri.push(b as char);
        } else {
            uri.push_str(&format!("%{b:02X}"));
        }
    }
    uri
}

/// Undoes `%XX` escapes.
fn decode(text: &str) -> Option<String> {
    let mut bytes = Vec::new();
    let mut rest = text.bytes();
    while let Some(b) = rest.next() {
        if b == b'%' {
            let hex = [rest.next()?, rest.next()?];
            bytes.push(u8::from_str_radix(std::str::from_utf8(&hex).ok()?, 16).ok()?);
        } else {
            bytes.push(b);
        }
    }
    String::from_utf8(bytes).ok()
}
