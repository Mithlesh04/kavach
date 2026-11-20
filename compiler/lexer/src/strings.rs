// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! String literals (spec 1.7, decisions 0131 and 0143): `"…"` is exactly
//! what is written, on one line; a template in backticks fills in `{…}`
//! and reads escapes, and spans lines when its text starts on the next
//! line. `"""…"""` is an error, read as text over lines to recover.
//!
//! A string becomes StrStart, Text and interpolation tokens, then StrEnd.
//! While an interpolation is open, the main loop reads its tokens; the `}`
//! that closes it hands control back to `string_body`.

use crate::lexer::{Delim, Lexer, StrKind};
use crate::token::{Token, TokenKind};

/// Indentation bookkeeping for one open multi-line string. The common
/// indentation is only known at the closing delimiter, so line starts are
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
    /// `"""…"""` and `tag"""…"""` are gone: text over lines is written in
    /// backticks (spec 1.7, decision 0143). One error, from `start` (the
    /// tag, if any) to the quotes' end. The text is still read up to its
    /// closing `"""`, so the rest of the file is checked as usual.
    pub(crate) fn triple_quotes(&mut self, start: usize, tag: &str) {
        let message = format!(
            "Kavach has no `{tag}\"\"\"`: write the text in backticks, `` {tag}`…` ``, and write `\\{{` for a brace, `` \\` `` for a backtick and `\\\\` for a backslash (spec 1.7)"
        );
        self.error(start, self.pos + 3, message);
        self.open_string(StrKind::RawMulti, 3);
    }

    pub(crate) fn open_string(&mut self, kind: StrKind, delimiter_len: usize) {
        let start = self.pos;
        let tagged = matches!(self.tokens.last(), Some(t) if t.kind == TokenKind::Ident && t.span.end as usize == start);
        if kind.is_raw() && !tagged {
            self.plain.push(start as u32);
        }
        self.pos += delimiter_len;
        self.push(TokenKind::StrStart, start);
        self.in_string = Some(kind);
        if kind.is_multi() {
            self.eat_while(|c| c == ' ' || c == '\t' || c == '\r');
            if self.peek() == Some('\n') {
                self.pos += 1;
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
                    // Inside a `{` the line doesn't close, that `{` is the
                    // mistake (`"{"`): the line's end reports it.
                    if matches!(self.delims.last(), Some((Delim::Interp(k), _)) if !k.is_multi()) {
                        self.push(TokenKind::StrEnd, self.pos);
                        self.in_string = None;
                        return;
                    }
                    let message = match kind {
                        StrKind::Template => {
                            let message = "a template on one line can't continue on the next: start its text on the line after the backtick to span lines";
                            if self.goes_on_over_lines() {
                                // Read on to its backtick as text over
                                // lines, so the mistake is reported once.
                                self.error(self.pos, self.pos, message);
                                self.pos += 1;
                                self.multiline.push(MultiLine {
                                    lines: Vec::new(),
                                    at_line_start: true,
                                    line_has_content: false,
                                });
                                self.in_string = Some(StrKind::TemplateMulti);
                                return;
                            }
                            message
                        }
                        _ => {
                            "text in quotes can't continue on the next line: close it with `\"`, or write text over lines in backticks (spec 1.7)"
                        }
                    };
                    return self.unterminated(kind, message);
                }
                _ if rest.starts_with(kind.closer()) => {
                    return self.close_string(kind, &mut text, text_start);
                }
                Some('\\') if !kind.is_raw() => {
                    if let Some(c) = self.escape('`') {
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

    /// At a line break in a one-line template outside any `{…}`: whether
    /// its text looks meant to go on over lines, as a JavaScript template
    /// does, because the next backtick is the last on its line. A
    /// forgotten closing backtick usually meets another template's
    /// opening one instead, with its closing one after it.
    fn goes_on_over_lines(&self) -> bool {
        if self
            .delims
            .iter()
            .any(|(d, _)| matches!(d, Delim::Interp(_)))
        {
            return false;
        }
        let rest = self.rest();
        let mut chars = rest.char_indices();
        while let Some((i, c)) = chars.next() {
            match c {
                '\\' => {
                    chars.next();
                }
                '`' => return !rest[i + 1..].split('\n').next().unwrap_or("").contains('`'),
                _ => {}
            }
        }
        false
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
            || rest.starts_with("\"\"\"")
            || rest.starts_with('`');
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
            // The closing delimiter is on its own line: that line and the line
            // break before it are not part of the string.
            if let Some(line) = self.multi().lines.pop() {
                let cut = if line.offset > 0 { line.offset - 1 } else { 0 };
                text.truncate(cut);
            }
        }
        self.flush(text, text_start);
        let start = self.pos;
        self.pos += kind.closer().len();
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
