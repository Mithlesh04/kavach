// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Tagged templates with parameters (spec 10.15, decision 0142): the
//! value of `` sql`… {owner}` ``, whose text has the tag's placeholder
//! in place of each `{…}` and whose `params` hold the values; and where
//! each character of the text its checker reads is in the source.

use crate::interp::Interp;
use crate::value::{Eval, Flow, Value};
use kavach_ast::{Arg, ExprKind, Span, StrPart};
use kavach_checker::{LiteralUse, ParamConv, Prim};
use kavach_resolver::DefId;
use std::rc::Rc;

impl<'a> Interp<'a> {
    /// The tagged literal of type `def` whose text is `args`, when its
    /// tag takes parameters (`param`); `None` for any other.
    pub(crate) fn tagged_template(
        &mut self,
        def: DefId,
        args: &'a [Arg],
        span: Span,
    ) -> Option<Eval<'a>> {
        let arg = args.first()?;
        let c = self.c;
        let u = c.literal_uses.iter().find(|u| u.span == arg.value.span)?;
        let param = self.fns.get(&u.param?).copied()?;
        let ExprKind::Str(parts) = &arg.value.kind else {
            return None;
        };
        Some(self.template_value(def, u, param, parts, span))
    }

    fn template_value(
        &mut self,
        def: DefId,
        u: &'a LiteralUse,
        param: &'a kavach_ast::FnDecl,
        parts: &'a [StrPart],
        span: Span,
    ) -> Eval<'a> {
        // The text first, then the values, as native code does.
        let mut text = String::new();
        let mut n = 0;
        for part in parts {
            match part {
                StrPart::Text(t) => text.push_str(t),
                StrPart::Expr(_) => {
                    n += 1;
                    let arg = vec![(None, Value::Int(n, Prim::Int))];
                    if let Value::Str(s) = self.call_fn(param, None, arg, span)? {
                        text.push_str(&s);
                    }
                }
            }
        }
        let mut params = Vec::new();
        for part in parts {
            let StrPart::Expr(x) = part else {
                continue;
            };
            let conv = u.holes.get(params.len()).map(|h| &h.conv);
            params.push(match conv {
                Some(ParamConv::Json) => self.json_item(x)?,
                Some(ParamConv::Variant { union, name }) => {
                    let v = self.eval(x)?;
                    self.construct(*union, Some(name), vec![(None, v)], x.span)?
                }
                _ => self.eval(x)?,
            });
        }
        let text = self.str_value(&text);
        let params = Value::List(Rc::new(params.into()));
        self.construct(
            def,
            None,
            vec![(None, text), (Some("params"), params)],
            span,
        )
    }

    /// The text a tagged template's checker reads, with the placeholders
    /// in, and where each of its characters is in the source (`None` if
    /// that can't be worked out). An `Err` is why a placeholder failed.
    pub(crate) fn checked_template(
        &mut self,
        u: &LiteralUse,
        steps: u64,
    ) -> Result<(String, Option<Vec<(usize, usize)>>), String> {
        let mut placeholders = Vec::new();
        if let Some(f) = u.param.and_then(|p| self.fns.get(&p).copied()) {
            for n in 1..=u.holes.len() {
                self.steps = 0;
                self.step_limit = Some(steps);
                let arg = vec![(None, Value::Int(n as i128, Prim::Int))];
                let written = self.call_fn(f, None, arg, u.span);
                self.step_limit = None;
                match written {
                    Ok(Value::Str(s)) => placeholders.push(s.to_string()),
                    Err(Flow::Panic(why, _)) => return Err(why.to_string()),
                    _ => placeholders.push(String::new()),
                }
            }
        }
        let mut text = String::new();
        for (i, piece) in u.pieces.iter().enumerate() {
            text.push_str(piece);
            text.push_str(placeholders.get(i).map_or("", |p| p.as_str()));
        }
        let holes: Vec<Span> = u.holes.iter().map(|h| h.span).collect();
        let lengths: Vec<usize> = placeholders.iter().map(|p| p.chars().count()).collect();
        let places = template_places(&self.c.src, u.span.start as usize, &holes, &lengths);
        let places = (places.len() == text.chars().count()).then_some(places);
        Ok((text, places))
    }
}

/// One piece of a template's source, in order.
enum Unit {
    /// A character, or an escape, which is one character of the text.
    Char(usize, usize),
    /// A space or tab at the start of a line, which the common
    /// indentation may remove.
    Indent(usize),
    /// A `{…}` and how many characters its placeholder has.
    Hole(usize, usize, usize),
}

/// A line of a template over lines.
struct Line {
    /// Its first unit: the line break before it is the one before that.
    first: usize,
    indent: usize,
    /// Only spaces: it doesn't count for the common indentation.
    blank: bool,
    content: bool,
}

/// Where each character of the text of the template whose backtick is
/// at `open` is in `src`, as the lexer reads it (spec 1.7): escapes are
/// one character, the common indentation of lines is removed, and the
/// `{…}` around `holes[n]` stands for `lengths[n]` characters.
pub(crate) fn template_places(
    src: &str,
    open: usize,
    holes: &[Span],
    lengths: &[usize],
) -> Vec<(usize, usize)> {
    let mut pos = open + 1;
    let after = &src[pos..];
    let multi = after
        .trim_start_matches([' ', '\t', '\r'])
        .starts_with('\n');
    if multi {
        pos += after.find('\n').map_or(0, |i| i + 1);
    }
    let mut units = Vec::new();
    let mut lines: Vec<Line> = Vec::new();
    let mut line_start = multi;
    let mut hole = 0;
    loop {
        if line_start {
            let first = units.len();
            let mut indent = 0;
            while matches!(src[pos..].chars().next(), Some(' ' | '\t')) {
                units.push(Unit::Indent(pos));
                pos += 1;
                indent += 1;
            }
            let rest = &src[pos..];
            let blank = rest.is_empty()
                || rest.starts_with('\n')
                || rest.starts_with("\r\n")
                || rest.starts_with('`');
            lines.push(Line {
                first,
                indent,
                blank,
                content: false,
            });
            line_start = false;
        }
        let rest = &src[pos..];
        let Some(c) = rest.chars().next() else {
            break;
        };
        let end = match c {
            '`' => break,
            '\\' if rest.starts_with("\\u{") => pos + rest.find('}').map_or(rest.len(), |i| i + 1),
            '\\' => pos + 1 + rest[1..].chars().next().map_or(0, char::len_utf8),
            '{' => {
                let close = holes.get(hole).map_or(pos, |h| h.end as usize);
                let end = src[close..].find('}').map_or(src.len(), |i| close + i + 1);
                units.push(Unit::Hole(
                    pos,
                    end,
                    lengths.get(hole).copied().unwrap_or(0),
                ));
                hole += 1;
                pos = end;
                if let Some(line) = lines.last_mut() {
                    line.content = true;
                }
                continue;
            }
            '\r' if multi && rest.starts_with("\r\n") => {
                pos += 1;
                continue;
            }
            '\n' => {
                units.push(Unit::Char(pos, pos + 1));
                pos += 1;
                line_start = true;
                continue;
            }
            c => pos + c.len_utf8(),
        };
        units.push(Unit::Char(pos, end));
        pos = end;
        if let Some(line) = lines.last_mut() {
            line.content = true;
        }
    }
    let mut kept = vec![true; units.len()];
    let common = lines
        .iter()
        .filter(|l| !l.blank)
        .map(|l| l.indent)
        .min()
        .unwrap_or(0);
    for line in &lines {
        for k in kept
            .iter_mut()
            .skip(line.first)
            .take(line.indent.min(common))
        {
            *k = false;
        }
    }
    // A closing backtick on its own line: that line and the line break
    // before it are not part of the text.
    if let Some(last) = lines.last().filter(|l| !l.content) {
        for k in kept.iter_mut().skip(last.first.saturating_sub(1)) {
            *k = false;
        }
    }
    let mut places = Vec::new();
    for (unit, _) in units.iter().zip(&kept).filter(|(_, k)| **k) {
        match *unit {
            Unit::Char(start, end) => places.push((start, end)),
            Unit::Indent(at) => places.push((at, at + 1)),
            Unit::Hole(start, end, n) => places.extend(std::iter::repeat_n((start, end), n)),
        }
    }
    places
}
