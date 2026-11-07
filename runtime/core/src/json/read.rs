//! Reading the pieces of JSON text (spec 2.12): spaces, strings with
//! their escapes, numbers and words, and where in the text an error is.

use super::{Build, Error, Tree};
use std::borrow::Cow;

/// Whether the `f64` nearest to `text` surely writes back as `text`
/// (`format_float`), without writing it: a number with a point and no
/// exponent, at most 15 significant digits (which an `f64` always gives
/// back), no trailing zero but a lone `.0`, and in `format_float`'s plain
/// range (1e-5 up to 1e16). Most fractions in real data are like this.
pub(super) fn writes_back(text: &str) -> bool {
    let unsigned = text.strip_prefix('-').unwrap_or(text);
    let Some((whole, fraction)) = unsigned.split_once('.') else {
        return false;
    };
    let plain = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if !plain(whole) || !plain(fraction) || (fraction.ends_with('0') && fraction != "0") {
        return false;
    }
    let zeros = fraction.bytes().take_while(|b| *b == b'0').count();
    let significant = if whole == "0" {
        fraction.len() - zeros
    } else {
        whole.len() + fraction.len()
    };
    significant <= 15 && (whole != "0" || zeros <= 4 || fraction == "0")
}

pub(super) struct Parser<'a> {
    pub(super) s: &'a [u8],
    pub(super) text: &'a str,
    pub(super) at: usize,
    /// Comments count as space, and a trailing comma is allowed (JSONC).
    pub(super) comments: bool,
}

impl<'a> Parser<'a> {
    pub(super) fn peek(&self) -> Option<u8> {
        self.s.get(self.at).copied()
    }

    pub(super) fn next_byte(&mut self) -> Option<u8> {
        let b = self.peek();
        self.at += 1;
        b
    }

    pub(super) fn space(&mut self) {
        loop {
            while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
                self.at += 1;
            }
            if !self.comments || self.peek() != Some(b'/') {
                return;
            }
            let rest = &self.s[self.at..];
            if rest.starts_with(b"//") {
                let end = rest.iter().position(|&c| c == b'\n').unwrap_or(rest.len());
                self.at += end;
            } else if rest.starts_with(b"/*") {
                // An unended comment runs to the end: the text then ends
                // where a value or a bracket should be.
                let end = rest[2..].windows(2).position(|w| w == b"*/");
                self.at += end.map_or(rest.len(), |e| e + 4);
            } else {
                return;
            }
        }
    }

    /// The byte after any space, without moving.
    pub(super) fn after_space(&mut self) -> Option<u8> {
        let at = self.at;
        self.space();
        let b = self.peek();
        self.at = at;
        b
    }

    /// The line and column of byte `at`.
    pub(super) fn place(&self, at: usize) -> (usize, usize) {
        let before = &self.text[..at.min(self.text.len())];
        let line = before.matches('\n').count() + 1;
        let start = before.rfind('\n').map_or(0, |i| i + 1);
        (line, before[start..].chars().count() + 1)
    }

    pub(super) fn syntax_at(&self, at: usize, message: &str) -> Error {
        let (line, column) = self.place(at);
        Error::Syntax {
            line,
            column,
            message: message.to_string(),
        }
    }

    pub(super) fn syntax(&self, message: &str) -> Error {
        self.syntax_at(self.at, message)
    }

    /// An error at the byte just read.
    pub(super) fn syntax_before(&self, message: &str) -> Error {
        self.syntax_at(self.at.saturating_sub(1), message)
    }

    /// `"key"` and the `:` after it.
    pub(super) fn key(&mut self) -> Result<Cow<'a, str>, Error> {
        if self.peek() != Some(b'"') {
            return Err(self.syntax("expected a key in quotes"));
        }
        let key = self.string()?;
        self.space();
        if self.next_byte() != Some(b':') {
            return Err(self.syntax_before("expected `:` after the key"));
        }
        self.space();
        Ok(key)
    }

    /// A number, `true`, `false` or `null`.
    pub(super) fn scalar<B: Build>(&mut self, b: &mut B) -> Result<B::Value, Error> {
        match self.peek() {
            Some(b'-' | b'0'..=b'9') => {
                let start = self.at;
                Ok(match self.number()? {
                    Tree::Int(n) => b.int(n),
                    Tree::Float(f) => b.float(f, &self.text[start..self.at]),
                    Tree::Number(text) => b.number(&text),
                    _ => b.null(),
                })
            }
            Some(b't') => self.word("true").map(|_| b.bool(true)),
            Some(b'f') => self.word("false").map(|_| b.bool(false)),
            Some(b'n') => self.word("null").map(|_| b.null()),
            Some(_) => Err(self.syntax("expected a value")),
            None => Err(self.syntax("the text ends where a value should be")),
        }
    }

    pub(super) fn word(&mut self, word: &str) -> Result<(), Error> {
        if self.s[self.at..].starts_with(word.as_bytes()) {
            self.at += word.len();
            Ok(())
        } else {
            Err(self.syntax("expected a value"))
        }
    }

    pub(super) fn digits(&mut self) -> usize {
        let start = self.at;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.at += 1;
        }
        self.at - start
    }

    pub(super) fn number(&mut self) -> Result<Tree, Error> {
        let (written, integer) = self.number_text()?;
        self.classify(written, integer)
    }

    /// A number's text, checked as RFC 8259 writes numbers, and whether it
    /// is written without a fraction or exponent.
    pub(super) fn number_text(&mut self) -> Result<(&'a str, bool), Error> {
        let start = self.at;
        if self.peek() == Some(b'-') {
            self.at += 1;
        }
        let first = self.peek();
        let whole = self.digits();
        if whole == 0 || (first == Some(b'0') && whole > 1) {
            return Err(self.syntax_at(start, "a number needs digits, without leading zeros"));
        }
        let mut integer = true;
        if self.peek() == Some(b'.') {
            self.at += 1;
            integer = false;
            if self.digits() == 0 {
                return Err(self.syntax("expected digits after `.`"));
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.at += 1;
            integer = false;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.at += 1;
            }
            if self.digits() == 0 {
                return Err(self.syntax("expected digits in the exponent"));
            }
        }
        Ok((&self.text[start..self.at], integer))
    }

    /// The tree of number `written`: kept as an `i64` or `f64` only when it
    /// writes back the same (decision 0071); otherwise by its text.
    fn classify(&self, written: &str, integer: bool) -> Result<Tree, Error> {
        if integer {
            // JSON has no leading zeros, so only `-0` writes back otherwise.
            if let (Ok(n), false) = (written.parse::<i64>(), written == "-0") {
                return Ok(Tree::Int(n));
            }
        } else if let Ok(f) = written.parse::<f64>()
            && (writes_back(written) || (f.is_finite() && crate::format_float(f) == written))
        {
            return Ok(Tree::Float(f));
        }
        Ok(Tree::Number(written.to_string()))
    }

    /// A string in quotes, with its escapes decoded: borrowed from the
    /// text when it has none.
    pub(super) fn string(&mut self) -> Result<Cow<'a, str>, Error> {
        self.at += 1;
        let first = self.at;
        while matches!(self.peek(), Some(b) if b != b'"' && b != b'\\' && b >= 0x20) {
            self.at += 1;
        }
        if self.peek() == Some(b'"') {
            self.at += 1;
            return Ok(Cow::Borrowed(&self.text[first..self.at - 1]));
        }
        self.at = first;
        let mut out = String::new();
        loop {
            // Copy the plain run up to the next quote, escape or control.
            let run = self.at;
            while matches!(self.peek(), Some(b) if b != b'"' && b != b'\\' && b >= 0x20) {
                self.at += 1;
            }
            out.push_str(&self.text[run..self.at]);
            match self.next_byte() {
                Some(b'"') => return Ok(Cow::Owned(out)),
                Some(b'\\') => self.escape(&mut out)?,
                Some(_) => return Err(self.syntax_before("a control character must be escaped")),
                None => return Err(self.syntax("the text ends inside a string")),
            }
        }
    }

    pub(super) fn escape(&mut self, out: &mut String) -> Result<(), Error> {
        let c = match self.next_byte() {
            Some(b'"') => '"',
            Some(b'\\') => '\\',
            Some(b'/') => '/',
            Some(b'b') => '\u{8}',
            Some(b'f') => '\u{c}',
            Some(b'n') => '\n',
            Some(b'r') => '\r',
            Some(b't') => '\t',
            Some(b'u') => {
                let start = self.at - 2;
                let high = self.hex4()?;
                let code = if (0xD800..0xDC00).contains(&high) {
                    // A surrogate pair: `😀`.
                    if !self.s[self.at..].starts_with(b"\\u") {
                        return Err(self.syntax_at(start, "an unpaired surrogate"));
                    }
                    self.at += 2;
                    let low = self.hex4()?;
                    if !(0xDC00..0xE000).contains(&low) {
                        return Err(self.syntax_at(start, "an unpaired surrogate"));
                    }
                    0x10000 + ((high - 0xD800) << 10) + (low - 0xDC00)
                } else {
                    high
                };
                char::from_u32(code)
                    .ok_or_else(|| self.syntax_at(start, "an unpaired surrogate"))?
            }
            _ => return Err(self.syntax_before("an unknown escape")),
        };
        out.push(c);
        Ok(())
    }

    pub(super) fn hex4(&mut self) -> Result<u32, Error> {
        let digits = self.text.get(self.at..self.at + 4).unwrap_or("");
        match u32::from_str_radix(digits, 16) {
            Ok(n) if digits.len() == 4 && digits.bytes().all(|b| b.is_ascii_hexdigit()) => {
                self.at += 4;
                Ok(n)
            }
            _ => Err(self.syntax("expected four hex digits after `\\u`")),
        }
    }
}
