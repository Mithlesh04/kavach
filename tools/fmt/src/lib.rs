//! `kavach fmt` (spec 12.6, 10.5): one style for all Kavach code, with no
//! options.
//!
//! The formatter changes only the layout of lines, never what is on them:
//!
//! - indentation: 4 spaces per open `{`, `[` or `(`, and one more for a
//!   line that continues the one before (after an operator, or starting
//!   with `.`);
//! - trailing spaces removed, `\r\n` turned into `\n`, one final newline;
//! - at most one blank line in a row, and none just inside `{ … }`;
//! - multi-line strings are left exactly as they are.
//!
//! Comments stay where they are. After formatting, the file's tokens must
//! be the same as before; if not, nothing is changed.
//!
//! Before that, `fix_names` renames declarations in the wrong style, with
//! their uses (spec 1.5; `names.rs`).

use kavach_lexer::{Keyword, Token, TokenKind as T, block_comment_len, lex};

mod names;
pub use names::fix_names;

/// Tokens after which a line continues on the next one.
fn continues(t: &T) -> bool {
    matches!(
        t,
        T::Plus
            | T::Minus
            | T::Star
            | T::Slash
            | T::Percent
            | T::EqEq
            | T::NotEq
            | T::Lt
            | T::Le
            | T::Gt
            | T::Ge
            | T::Eq
            | T::PlusEq
            | T::MinusEq
            | T::StarEq
            | T::SlashEq
            | T::PercentEq
            | T::StarStar
            | T::StarStarEq
            | T::AmpEq
            | T::PipeEq
            | T::CaretEq
            | T::ShlEq
            | T::ShrEq
            | T::QuestionQuestion
            | T::Amp
            | T::Pipe
            | T::Caret
            | T::Shl
            | T::Shr
            | T::DotDot
            | T::DotDotEq
            | T::Arrow
    ) || matches!(t, T::Keyword(Keyword::And | Keyword::Or))
}

fn opens(t: &T) -> bool {
    matches!(t, T::LParen | T::LBracket | T::LBrace)
}

fn closes(t: &T) -> bool {
    matches!(t, T::RParen | T::RBracket | T::RBrace)
}

/// The tokens that make up the code, for the before-and-after check.
fn code_tokens(src: &str) -> Vec<T> {
    lex(src)
        .tokens
        .into_iter()
        .map(|t| t.kind)
        .filter(|k| !matches!(k, T::Newline | T::Eof))
        .collect()
}

/// `src` in Kavach's one style, or why it can't be formatted.
pub fn format(src: &str) -> Result<String, String> {
    let src = src.replace("\r\n", "\n");
    let lexed = lex(&src);
    if let Some(e) = lexed.errors.first() {
        return Err(format!("the file doesn't lex: {}", e.message));
    }
    let tokens: Vec<&Token> = lexed
        .tokens
        .iter()
        .filter(|t| !matches!(t.kind, T::Newline | T::Eof))
        .collect();
    let strings = multi_line_strings(&src, &tokens);
    let blocks = block_comments(&src, &tokens);
    let mut out: Vec<String> = Vec::new();
    // Comment lines waiting for the next code line: (line, depth).
    let mut comments: Vec<(usize, i64)> = Vec::new();
    let mut offset = 0usize;
    let mut next = 0usize;
    // Open delimiters: whether each is a `{`, and the line it opened on.
    // Several opened on one line indent by one level, not several.
    let mut stack: Vec<(bool, usize)> = Vec::new();
    // The last token of the previous line that has code.
    let mut previous: Option<&T> = None;
    for line in src.split('\n') {
        let start = offset;
        offset += line.len() + 1;
        // Tokens that start before this line open and close delimiters.
        while next < tokens.len() && (tokens[next].span.start as usize) < start {
            let t = &tokens[next].kind;
            let opened_on = src[..tokens[next].span.start as usize]
                .matches('\n')
                .count();
            if opens(t) {
                stack.push((matches!(t, T::LBrace), opened_on));
            } else if closes(t) {
                stack.pop();
            }
            next += 1;
        }
        let depth = levels(&stack);
        // Lines inside a multi-line string, and the one with its closing
        // `"""`, are part of the string: that line's indentation decides
        // how much is removed from the text.
        // So are the lines inside a `/* … */` comment after its first.
        if strings
            .iter()
            .chain(&blocks)
            .any(|(a, b)| start > *a && start < *b)
        {
            out.push(line.to_string());
            continue;
        }
        let text = line.trim();
        if text.is_empty() {
            out.push(String::new());
            continue;
        }
        let on_line: Vec<&T> = tokens[next..]
            .iter()
            .take_while(|t| (t.span.start as usize) < offset)
            .map(|t| &t.kind)
            .collect();
        // A line with only a comment: indented like the code below it.
        if on_line.is_empty() {
            comments.push((out.len(), depth));
            out.push(text.to_string());
            continue;
        }
        let leading_closers = on_line.iter().take_while(|t| closes(t)).count();
        // Signature clauses (`-> T`, `uses`, `require`, …) continue the
        // `fn` line above them (spec 3.5).
        let clause = matches!(
            on_line[0],
            T::Arrow
                | T::Bang
                | T::Keyword(Keyword::Uses | Keyword::Require | Keyword::Ensure | Keyword::Example)
        );
        // Inside `( … )` or `[ … ]` the bracket already indents the line.
        let in_brackets = stack.last().is_some_and(|(brace, _)| !brace);
        let continuation = !in_brackets
            && (text.starts_with('.')
                || text.starts_with("?.")
                || clause
                || previous.is_some_and(continues));
        let kept = &stack[..stack.len().saturating_sub(leading_closers)];
        let level = levels(kept) + i64::from(continuation && leading_closers == 0);
        for (at, comment_depth) in comments.drain(..) {
            let l = if leading_closers > 0 {
                comment_depth
            } else {
                level
            };
            out[at] = format!("{}{}", "    ".repeat(l as usize), out[at]);
        }
        out.push(format!("{}{text}", "    ".repeat(level as usize)));
        if let Some(last) = on_line.last() {
            previous = Some(last);
        }
    }
    for (at, comment_depth) in comments {
        out[at] = format!("{}{}", "    ".repeat(comment_depth as usize), out[at]);
    }
    let formatted = tidy_blank_lines(&out);
    if code_tokens(&formatted) != code_tokens(&src) {
        return Err("formatting would change the code; please report this as a bug".into());
    }
    Ok(formatted)
}

/// Indentation levels for these open delimiters: one per line that
/// opened any.
fn levels(stack: &[(bool, usize)]) -> i64 {
    let mut lines: Vec<usize> = stack.iter().map(|(_, l)| *l).collect();
    lines.dedup();
    lines.len() as i64
}

/// The byte ranges of multi-line strings (`"""` … `"""`).
fn multi_line_strings(src: &str, tokens: &[&Token]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut open: Vec<(usize, bool)> = Vec::new();
    for t in tokens {
        let (a, b) = (t.span.start as usize, t.span.end as usize);
        match t.kind {
            T::StrStart => open.push((a, src[a..b].starts_with("\"\"\""))),
            T::StrEnd => {
                if let Some((start, true)) = open.pop() {
                    out.push((start, b));
                }
            }
            _ => {}
        }
    }
    out
}

/// Where the `/* … */` comments are, as byte ranges: the gaps between
/// tokens hold only spaces and comments.
fn block_comments(src: &str, tokens: &[&Token]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut from = 0;
    let gaps = tokens
        .iter()
        .map(|t| (t.span.start as usize, t.span.end as usize));
    for (a, b) in gaps.chain([(src.len(), src.len())]) {
        let mut i = from;
        while i < a {
            let rest = &src[i..a];
            if rest.starts_with("//") {
                i += rest.find('\n').unwrap_or(rest.len());
            } else if rest.starts_with("/*") {
                let len = block_comment_len(&src[i..]).unwrap_or(src.len() - i);
                out.push((i, i + len));
                i += len;
            } else {
                i += rest.chars().next().map_or(1, char::len_utf8);
            }
        }
        from = from.max(b);
    }
    out
}

/// At most one blank line in a row, none at the start or end, none right
/// after a line ending in `{` or before a line starting with `}`.
fn tidy_blank_lines(lines: &[String]) -> String {
    let mut out: Vec<&str> = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if line.is_empty() {
            let after_open = out.last().is_none_or(|l| l.ends_with('{') || l.is_empty());
            let before_close = lines[i + 1..]
                .iter()
                .find(|l| !l.is_empty())
                .is_none_or(|l| l.trim_start().starts_with('}'));
            if after_open || before_close {
                continue;
            }
        }
        out.push(line);
    }
    let mut text = out.join("\n");
    text.push('\n');
    text
}

#[cfg(test)]
mod tests;
