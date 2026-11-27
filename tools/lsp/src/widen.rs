// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! An overflow of a type smaller than `Int` widened first (spec 1.8):
//! `a + b` made `Int(a) + Int(b)`, as the error's help line writes it.
//! The arithmetic is found in the compile's syntax tree: the one starting
//! where the error points whose widened form is that text.

use crate::fixes::Edit;
use kavach_ast::visit::{VisitMut, walk_expr, walk_file};
use kavach_ast::*;
use kavach_driver::Compiled;

/// How a widening fix's title starts: `kavach fix` applies one only when
/// it leaves fewer errors (`fixes.rs`).
pub(crate) const TITLE: &str = "Widen first: ";

/// The fix for the help line `widen first: wide` of an error at byte `at`
/// of the file `text`, which starts at byte `base` of the source.
pub(crate) fn widen(c: &Compiled, base: usize, text: &str, at: usize, wide: &str) -> Option<Edit> {
    let mut find = Widened {
        at: (base + at) as u32,
        wide,
        found: None,
    };
    walk_file(&mut find, &mut c.file.clone());
    let span = find.found?;
    let local = |x: u32| (x as usize).checked_sub(base);
    let (from, to) = (local(span.start)?, local(span.end)?);
    // `(a + b) * 2`: the brackets are part of the span, and stay.
    let old = text.get(from..to)?;
    let new = match old.starts_with('(') && old.ends_with(')') {
        true => format!("({wide})"),
        false => wide.to_string(),
    };
    Some(Edit {
        title: format!("{TITLE}`{wide}`"),
        from,
        to,
        text: new,
        safe: true,
    })
}

/// The `+`, `-` or `*` that starts at `at` and widens to `wide`.
struct Widened<'w> {
    at: u32,
    wide: &'w str,
    found: Option<Span>,
}

impl VisitMut for Widened<'_> {
    fn expr(&mut self, e: &mut Expr) {
        if let ExprKind::Binary(op, l, r) = &e.kind
            && e.span.start == self.at
            && kavach_checker::widened(*op, l, r).as_deref() == Some(self.wide)
        {
            self.found = Some(e.span);
        }
        walk_expr(self, e);
    }
}

#[cfg(test)]
mod tests {
    /// `src` after the fix for its first error, and whether it is safe.
    fn fixed(src: &str) -> Option<(String, bool)> {
        let c = kavach_driver::compile(src, "fix.kav", Default::default());
        let error = kavach_driver::Severity::Error;
        let m = c.messages.iter().find(|m| m.severity == error);
        let m = m.expect("an error");
        let file = c
            .files
            .iter()
            .find(|f| f.path == "fix.kav")
            .expect("fix.kav");
        let at = (m.span.start - file.start) as usize;
        let e = crate::fixes::fix(&c, "fix.kav", src, &m.text, at)?;
        Some((
            format!("{}{}{}", &src[..e.from], e.text, &src[e.to..]),
            e.safe,
        ))
    }

    #[test]
    fn an_overflow_of_a_small_type_is_widened_first() {
        let start = "fn main() {\n    const a: Int8 = 100\n    const b: Int8 = 100\n";
        let cases = [
            ("    print(a + b)\n", "    print(Int(a) + Int(b))\n"),
            ("    print(a + b + 1)\n", "    print(Int(a) + Int(b) + 1)\n"),
            (
                "    print((a + b) * 2)\n",
                "    print((Int(a) + Int(b)) * 2)\n",
            ),
            ("    print(2 * a)\n", "    print(2 * Int(a))\n"),
        ];
        for (line, want) in cases {
            let src = format!("{start}{line}}}\n");
            let (out, safe) = fixed(&src).expect("a fix");
            assert_eq!(out, format!("{start}{want}}}\n"));
            assert!(safe, "{src}");
            let after = kavach_driver::compile(&out, "fix.kav", Default::default());
            assert!(!after.has_errors(), "{out}\n{:?}", after.messages);
        }
    }

    /// `kavach fix` widens where the result may be an `Int`, and leaves
    /// `const c: Int8 = a + b`, where widening would only trade the
    /// overflow for a type mismatch; the editor still offers it there.
    #[test]
    fn kavach_fix_widens_only_where_it_takes_the_error_away() {
        let start = "fn main() {\n    const a: Int8 = 100\n    const b: Int8 = 100\n";
        let cases = [
            ("    print(a + b)\n", "    print(Int(a) + Int(b))\n"),
            (
                "    const c: Int8 = a + b\n    print(c)\n",
                "    const c: Int8 = a + b\n    print(c)\n",
            ),
        ];
        for (n, (line, want)) in cases.into_iter().enumerate() {
            let src = format!("{start}{line}}}\n");
            let name = format!("kavach-widen-{}-{n}.kav", std::process::id());
            let path = std::env::temp_dir().join(name);
            let path = path.to_string_lossy().into_owned();
            std::fs::write(&path, &src).unwrap();
            let done = crate::fix_file(&path).unwrap();
            let text = std::fs::read_to_string(&path).unwrap();
            std::fs::remove_file(&path).unwrap();
            assert_eq!(text, format!("{start}{want}}}\n"), "{done:?}");
            assert!(fixed(&src).is_some(), "the editor offers it: {src}");
        }
    }

    /// `count += 8` keeps `count`'s type: widening its operands can't
    /// help, so no fix is made.
    #[test]
    fn a_compound_assignment_gets_no_fix() {
        let src = "fn main() {\n    let count: Int8 = 120\n    count += 8\n    print(count)\n}\n";
        assert!(fixed(src).is_none());
    }
}
