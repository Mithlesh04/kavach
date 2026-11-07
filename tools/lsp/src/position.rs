//! Positions. The compiler counts bytes from the start of a file; the
//! Language Server Protocol counts lines and, within a line, UTF-16 code
//! units (its default encoding).

use serde_json::{Value, json};

/// The LSP position `{line, character}` of byte `offset` in `text`.
/// Offsets past the end, or inside a character, move back to a boundary.
pub fn position(text: &str, offset: usize) -> Value {
    let mut offset = offset.min(text.len());
    while !text.is_char_boundary(offset) {
        offset -= 1;
    }
    let line_start = text[..offset].rfind('\n').map_or(0, |i| i + 1);
    let line = text[..offset].matches('\n').count();
    let character = text[line_start..offset].encode_utf16().count();
    json!({"line": line, "character": character})
}

/// The byte offset in `text` of an LSP position. A position past the end
/// of its line means the end of the line; past the last line, the end.
pub fn offset(text: &str, position: &Value) -> usize {
    let line = position["line"].as_u64().unwrap_or(0);
    let character = position["character"].as_u64().unwrap_or(0) as usize;
    let mut start = 0;
    for _ in 0..line {
        match text[start..].find('\n') {
            Some(i) => start += i + 1,
            None => return text.len(),
        }
    }
    let end = text[start..].find('\n').map_or(text.len(), |i| start + i);
    let mut units = 0;
    for (i, ch) in text[start..end].char_indices() {
        if units >= character {
            return start + i;
        }
        units += ch.len_utf16();
    }
    end
}
