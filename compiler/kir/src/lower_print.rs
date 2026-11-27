// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `print(a, b, …, sep: s, end: e)` (spec 1.10). With the usual line end
//! and a `sep` written as plain text, the values are printed piece by
//! piece, as `print(x)` always is. Otherwise the line is made as text,
//! as interpolation makes it, and written at once with `io.write`'s op,
//! which shows it at once even without a line end.

use crate::ir::{self, ExprKind as K, PrintPart, Stmt, Type};
use crate::lower::{Lowered, Lowerer};
use crate::rt_op::RtOp;
use kavach_ast::{self as ast, ExprKind};

/// The text of `e` when it is a string with no `{…}` in it.
fn plain_text(e: &ast::Expr) -> Option<String> {
    let ExprKind::Str(parts) = &e.kind else {
        return None;
    };
    let mut text = String::new();
    for p in parts {
        match p {
            ast::StrPart::Text(t) => text.push_str(t),
            ast::StrPart::Expr(_) => return None,
        }
    }
    Some(text)
}

impl<'a> Lowerer<'a> {
    /// `print(…)` as a statement.
    pub(crate) fn print_stmt(
        &mut self,
        args: &'a [ast::Arg],
        span: ast::Span,
        out: &mut Vec<Stmt>,
    ) -> Lowered<()> {
        let named = |n: &str| {
            args.iter()
                .find(|a| a.name.as_ref().is_some_and(|x| x.name == n))
                .map(|a| &a.value)
        };
        let sep = named("sep").map_or_else(|| Some(" ".to_string()), plain_text);
        let line_end = named("end").is_none_or(|e| plain_text(e).as_deref() == Some("\n"));
        let Some(sep) = sep.filter(|_| line_end) else {
            return self.print_whole(args, span, out);
        };
        let mut parts = Vec::new();
        let values = args.iter().filter(|a| a.name.is_none()).map(|a| &a.value);
        for (i, v) in values.enumerate() {
            if i > 0 && !sep.is_empty() {
                parts.push(PrintPart::Text(sep.clone()));
            }
            // A text worked out before its task started is printed whole.
            let held = self.held_parts.contains_key(&(v.span.start, v.span.end));
            match &v.kind {
                ExprKind::Str(pieces) if !held => {
                    for p in pieces {
                        parts.push(match p {
                            ast::StrPart::Text(t) => PrintPart::Text(t.clone()),
                            ast::StrPart::Expr(x) => PrintPart::Value(self.expr(x)?),
                        });
                    }
                }
                _ => parts.push(PrintPart::Value(self.expr(v)?)),
            }
        }
        out.push(Stmt::Print(parts));
        Ok(())
    }

    /// `print` with another `end`, or a `sep` worked out while the program
    /// runs: each argument worked out once, in the order written (spec
    /// 3.10), then the line made and written whole.
    fn print_whole(
        &mut self,
        args: &'a [ast::Arg],
        span: ast::Span,
        out: &mut Vec<Stmt>,
    ) -> Lowered<()> {
        let line = self.line(span);
        let str_ty = self.str_type();
        let text = |t: &str| ir::Expr {
            kind: K::Str(t.to_string()),
            ty: str_ty,
            line,
        };
        let (mut sep, mut end) = (text(" "), text("\n"));
        let mut values = Vec::new();
        for a in args {
            let mut v = self.expr(&a.value)?;
            if !matches!(v.kind, K::Str(_)) {
                let local = self.new_local(v.ty);
                out.push(Stmt::Let(local, v));
                v = self.local_expr(local, line);
            }
            match a.name.as_ref().map(|n| n.name.as_str()) {
                Some("sep") => sep = v,
                Some("end") => end = v,
                _ => values.push(v),
            }
        }
        let mut parts = Vec::with_capacity(values.len() * 2 + 1);
        for (i, v) in values.into_iter().enumerate() {
            if i > 0 {
                parts.push(sep.clone());
            }
            parts.push(v);
        }
        parts.push(end);
        let whole = ir::Expr {
            kind: K::Format(parts),
            ty: str_ty,
            line,
        };
        out.push(Stmt::Expr(ir::Expr {
            kind: K::Rt(RtOp::IoWrite, vec![whole]),
            ty: Type::Unit,
            line,
        }));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::ir::{ExprKind, PrintPart, Stmt};
    use crate::rt_op::RtOp;

    /// How the prints in `main` are made: a `print` statement's parts,
    /// text as `T"…"` and a value as `V`; `W` for a line written whole.
    fn prints(body: &str) -> Vec<String> {
        let src = format!("fn main() {{\n    let n = 1\n    n += 1\n{body}\n}}\n");
        let c = kavach_driver::compile(&src, "t.kav", Default::default());
        assert!(!c.has_errors(), "{}", c.render());
        let program = crate::lower(&c).expect("lowers");
        let main = program.functions.iter().find(|f| f.name == "main");
        let shown = |p: &PrintPart| match p {
            PrintPart::Text(t) => format!("T{t:?}"),
            PrintPart::Value(_) => "V".to_string(),
        };
        main.expect("main")
            .body
            .stmts
            .iter()
            .filter_map(|s| match s {
                Stmt::Print(parts) => Some(parts.iter().map(shown).collect::<Vec<_>>().join(" ")),
                Stmt::Expr(e) if matches!(e.kind, ExprKind::Rt(RtOp::IoWrite, _)) => {
                    Some("W".to_string())
                }
                _ => None,
            })
            .collect()
    }

    /// One value prints as it always did; several print piece by piece
    /// while `sep` is plain text and the line ends as usual; any other
    /// layout makes the line whole.
    #[test]
    fn each_print_is_made_as_its_layout_needs() {
        let cases = [
            ("    print(n)", "V"),
            ("    print(\"n\")", "T\"n\""),
            ("    print()", ""),
            ("    print(\"n =\", n)", "T\"n =\" T\" \" V"),
            ("    print(n, n, sep: \", \")", "V T\", \" V"),
            ("    print(n, n, sep: \"\")", "V V"),
            ("    print(n, end: \"\")", "W"),
            ("    const s = \"-\"\n    print(n, n, sep: s)", "W"),
        ];
        for (body, want) in cases {
            assert_eq!(prints(body), [want], "{body}");
        }
    }
}
