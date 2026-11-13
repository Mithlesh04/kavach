//! String literals: plain `"…"`, raw `r"…"` and multi-line `"""…"""`, with
//! `{…}` interpolation (spec 1.7).
//!
//! A string becomes StrStart, Text and interpolation tokens, then StrEnd.
//! While an interpolation is open, the main loop reads its tokens; the `}`
//! that closes it hands control back to `string_body`.

use crate::lexer::{Delim, Lexer, StrKind};
use crate::token::{Token, TokenKind};

/// Indentation bookkeeping for one open multi-line string. The common
/// indentation is only known at the closing `"""`, so line starts are
/// recorded and the indentation is removed from the Text tokens at the end.
pub(crate) struct MultiLine {
    lines: Vec<LineStart>,
    at_line_start: bool,
    line_has_content: bool,
}

struct LineStart {
    /// Index of the Text token that holds this line's start.
    token: usize,
    /// Byte offset of the line's start inside that token's text.
    offset: usize,
    /// Number of leading spaces and tabs.
    indent: usize,
    /// Only whitespace; blank lines don't count for the common indentation.
    blank: bool,
}

impl Lexer<'_> {
    pub(crate) fn open_string(&mut self, kind: StrKind, delimiter_len: usize) {
        let start = self.pos;
        self.pos += delimiter_len;
        self.push(TokenKind::StrStart, start);
        self.in_string = Some(kind);
        if kind.is_multi() {
            self.eat_while(|c| c == ' ' || c == '\t' || c == '\r');
            if self.peek() == Some('\n') {
                self.pos += 1;
            } else {
                self.error(
                    start,
                    self.pos,
                    "multi-line text starts on the line after `\"\"\"`",
                );
            }
            self.multiline.push(MultiLine {
                lines: Vec::new(),
                at_line_start: true,
                line_has_content: false,
            });
        }
    }

    /// Reads string text until the string ends or an interpolation starts.
    pub(crate) fn string_body(&mut self, kind: StrKind) {
        let multi = kind.is_multi();
        let mut text = String::new();
        let text_start = self.pos;
        loop {
            if multi && self.multi().at_line_start {
                self.line_start(&mut text);
            }
            let rest = self.rest();
            match self.peek() {
                None => {
                    self.flush(&mut text, text_start);
                    return self.unterminated(kind, "this text is never closed");
                }
                Some('\n') if !multi => {
                    self.flush(&mut text, text_start);
                    return self.unterminated(
                        kind,
                        "text can't continue on the next line: close it with `\"`, or use `\"\"\"` for multi-line text",
                    );
                }
                Some('"') if !multi || rest.starts_with("\"\"\"") => {
                    return self.close_string(kind, &mut text, text_start);
                }
                Some('\\') if !kind.is_raw() => {
                    if let Some(c) = self.escape('"') {
                        text.push(c);
                    }
                    self.mark_content(kind);
                }
                Some('{') if !kind.is_raw() => {
                    self.flush(&mut text, text_start);
                    let start = self.pos;
                    self.pos += 1;
                    self.push(TokenKind::InterpStart, start);
                    self.delims.push((Delim::Interp(kind), start));
                    self.in_string = None;
                    return self.mark_content(kind);
                }
                Some('\r') if multi && rest.starts_with("\r\n") => self.pos += 1,
                Some('\n') => {
                    text.push('\n');
                    self.pos += 1;
                    self.multi().at_line_start = true;
                }
                Some(c) => {
                    text.push(c);
                    self.pos += c.len_utf8();
                    self.mark_content(kind);
                }
            }
        }
    }

    fn multi(&mut self) -> &mut MultiLine {
        self.multiline
            .last_mut()
            .expect("a multi-line string is open")
    }

    fn mark_content(&mut self, kind: StrKind) {
        if kind.is_multi() {
            self.multi().line_has_content = true;
        }
    }

    fn line_start(&mut self, text: &mut String) {
        let offset = text.len();
        let mut indent = 0;
        while let Some(c @ (' ' | '\t')) = self.peek() {
            text.push(c);
            self.pos += 1;
            indent += 1;
        }
        let rest = self.rest();
        let blank = rest.is_empty()
            || rest.starts_with('\n')
            || rest.starts_with("\r\n")
            || rest.starts_with("\"\"\"");
        let token = self.tokens.len();
        let m = self.multi();
        m.lines.push(LineStart {
            token,
            offset,
            indent,
            blank,
        });
        m.at_line_start = false;
        m.line_has_content = false;
    }

    fn flush(&mut self, text: &mut String, start: usize) {
        if !text.is_empty() {
            let text = std::mem::take(text);
            self.push(TokenKind::Text(text), start);
        }
    }

    fn close_string(&mut self, kind: StrKind, text: &mut String, text_start: usize) {
        if kind.is_multi() && !self.multi().line_has_content {
            // The closing `"""` is on its own line: that line and the line
            // break before it are not part of the string.
            if let Some(line) = self.multi().lines.pop() {
                let cut = if line.offset > 0 { line.offset - 1 } else { 0 };
                text.truncate(cut);
            }
        }
        self.flush(text, text_start);
        let start = self.pos;
        self.pos += if kind.is_multi() { 3 } else { 1 };
        self.push(TokenKind::StrEnd, start);
        self.in_string = None;
        if kind.is_multi() {
            self.dedent();
        }
    }

    /// Ends a string that was never closed, keeping the token structure whole
    /// (a zero-width StrEnd) so the parser can continue.
    fn unterminated(&mut self, kind: StrKind, message: &str) {
        self.error(self.pos, self.pos, message);
        self.push(TokenKind::StrEnd, self.pos);
        self.in_string = None;
        if kind.is_multi() {
            self.dedent();
        }
    }

    /// Removes the common indentation of the multi-line string that just ended.
    fn dedent(&mut self) {
        let Some(m) = self.multiline.pop() else {
            return;
        };
        let common = m
            .lines
            .iter()
            .filter(|l| !l.blank)
            .map(|l| l.indent)
            .min()
            .unwrap_or(0);
        for line in m.lines.iter().rev() {
            let n = line.indent.min(common);
            if n == 0 {
                continue;
            }
            if let Some(Token {
                kind: TokenKind::Text(s),
                ..
            }) = self.tokens.get_mut(line.token)
            {
                s.replace_range(line.offset..line.offset + n, "");
            }
        }
        // A line that held only indentation before an interpolation leaves an
        // empty Text token behind. Remove those, last first so indexes stay valid.
        let mut touched: Vec<usize> = m.lines.iter().map(|l| l.token).collect();
        touched.dedup();
        for index in touched.into_iter().rev() {
            if matches!(self.tokens.get(index), Some(Token { kind: TokenKind::Text(s), .. }) if s.is_empty())
            {
                self.tokens.remove(index);
            }
        }
    }
}
