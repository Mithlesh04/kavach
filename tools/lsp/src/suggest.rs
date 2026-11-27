// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Fixes for the compiler's suggestions (spec 12.7): the change a message
//! asks for, made. Each is found from the message and where it points.

use crate::fixes::Edit;
use kavach_ast::visit::{VisitMut, walk_expr, walk_file};
use kavach_ast::*;
use kavach_driver::Compiled;

/// The fix for `message` at byte `at` of the file `text`, which starts at
/// byte `base` of the source.
pub(crate) fn suggest(
    c: &Compiled,
    base: usize,
    text: &str,
    message: &str,
    at: usize,
) -> Option<Edit> {
    let edit = |title: &str, from: usize, to: usize, new: &str, safe: bool| Edit {
        title: title.to_string(),
        from,
        to,
        text: new.to_string(),
        safe,
    };
    // A bare variant (decision 0070): the name, written with its union's.
    if message.contains("(decision 0070)") {
        let name = message.strip_prefix('`')?.split('`').next()?;
        let full = message.split(": write `").nth(1)?.split('`').next()?;
        if !text[at..].starts_with(name) {
            return None;
        }
        let title = format!("Write `{full}`");
        return Some(edit(&title, at, at + name.len(), full, true));
    }
    if message.starts_with("Kavach has no `!` for \"not\"") && text[at..].starts_with('!') {
        return Some(edit("Write `not`", at, at + 1, "not ", true));
    }
    // Another language's operator: `===`, `!==`, `&&`, `||` (spec 1.8).
    if let Some(rest) = message.strip_prefix("Kavach has no `") {
        let op = rest.split('`').next()?;
        let fix = rest.split(": write `").nth(1)?.split('`').next()?;
        if ["===", "!==", "&&", "||"].contains(&op) && text[at..].starts_with(op) {
            let title = format!("Write `{fix}`");
            return Some(edit(&title, at, at + op.len(), fix, true));
        }
    }
    if message.starts_with("`=` sets a variable: to compare, write `==`") {
        return Some(edit("Write `==`", at, at + 1, "==", true));
    }
    if message.contains("one past the last index of") {
        let from = at + text[at..].find("..=")?;
        return Some(edit(
            "Write `..` instead of `..=`",
            from,
            from + 3,
            "..",
            true,
        ));
    }
    if message.contains("but it is a `const`: declare it with `let`") {
        let def = *c.resolution.uses.get(&((base + at) as u32))?;
        let declared = (c.resolution.defs[def].span?.start as usize).checked_sub(base)?;
        let from = text[..declared].rfind("const")?;
        return Some(edit("Declare it with `let`", from, from + 5, "let", true));
    }
    if message.contains("is wrong for a negative odd") {
        return odd(c, base, text, at);
    }
    if let Some(uses) = message
        .split("must say what it uses: add `")
        .nth(1)
        .and_then(|rest| rest.split('`').next())
    {
        return add_uses(c, base, text, at, uses);
    }
    // A `Drop` value given to `_` (spec 6.1): kept to the block's end. It
    // changes when `drop` runs, so the editor only.
    if message.starts_with("`_` names nothing, so this value drops here")
        && text[at..].starts_with('_')
    {
        return Some(edit("Name it `_guard`", at, at + 1, "_guard", false));
    }
    if let Some(cap) = message
        .split("but its `uses` clause doesn't list it: add `")
        .nth(1)
        .and_then(|rest| rest.split('`').next())
    {
        let f = around(&c.file.items, (base + at) as u32)?;
        let end = (f.uses.last()?.span.end as usize).checked_sub(base)?;
        let title = format!("Add `{cap}` to `uses`");
        return Some(edit(&title, end, end, &format!(", {cap}"), true));
    }
    if message.starts_with("`.unwrap()` panics on `none` and says nothing of why") {
        return say_why(c, base, text, at);
    }
    // An overflow of a type smaller than `Int` (spec 1.8): widened first.
    if let Some(wide) = message
        .split("\nhelp: ")
        .nth(1)
        .and_then(|help| help.split("widen first: ").nth(1))
        .and_then(|rest| rest.split(';').next())
    {
        return crate::widen::widen(c, base, text, at, wide);
    }
    if message.contains("hiding the one declared above") {
        // `let total = total + i` made `total = total + i`: it changes what
        // the code does, so the editor only.
        let from = text[..at].trim_end().strip_suffix("let")?.len();
        let name = message.strip_prefix('`')?.split('`').next()?;
        let title = format!("Change the `{name}` declared above");
        return Some(edit(&title, from, at, "", false));
    }
    if let Some(name) = message
        .split("; did you mean `")
        .nth(1)
        .and_then(|rest| rest.split('`').next())
    {
        let wrong = message.strip_prefix('`')?.split('`').next()?;
        if !text[at..].starts_with(wrong) {
            return None;
        }
        // The closest name may not be the one meant: the editor only.
        let title = format!("Change to `{name}`");
        return Some(edit(&title, at, at + wrong.len(), name, false));
    }
    None
}

/// `x.unwrap()` made `x ?? panic("…")`, for the reason to be written in
/// (spec 2.7): in brackets unless it stands alone. The editor only, as the
/// reason is still to write.
fn say_why(c: &Compiled, base: usize, text: &str, at: usize) -> Option<Edit> {
    let mut find = UnwrapAt {
        at: (base + at) as u32,
        found: None,
    };
    walk_file(&mut find, &mut c.file.clone());
    let (receiver, whole) = find.found?;
    let local = |x: u32| (x as usize).checked_sub(base);
    let (from, to) = (local(whole.start)?, local(whole.end)?);
    let new = format!(
        "{} ?? panic(\"…\")",
        &text[local(receiver.start)?..local(receiver.end)?]
    );
    let before = text[..from].trim_end_matches([' ', '\t']);
    let after = text[to..].trim_start_matches([' ', '\t']);
    let alone_before = before.is_empty()
        || before.ends_with(['(', ',', '[', '{', '\n'])
        || before.ends_with("return")
        || (before.ends_with('=') && !before[..before.len() - 1].ends_with(['=', '!', '<', '>']));
    let alone_after = after.is_empty() || after.starts_with([')', ',', ']', '}', '\n', '\r']);
    let new = if alone_before && alone_after {
        new
    } else {
        format!("({new})")
    };
    Some(Edit {
        title: "Say why it can't be `none`".to_string(),
        from,
        to,
        text: new,
        safe: false,
    })
}

/// The `.unwrap()` call that starts at `at`, the smallest if several do:
/// its receiver's span and its own.
struct UnwrapAt {
    at: u32,
    found: Option<(Span, Span)>,
}

impl VisitMut for UnwrapAt {
    fn expr(&mut self, e: &mut Expr) {
        if let ExprKind::Call { callee, args, .. } = &e.kind
            && let ExprKind::Field { base, name, .. } = &callee.kind
            && name.name == "unwrap"
            && args.is_empty()
            && e.span.start == self.at
            && self.found.is_none_or(|(_, w)| e.span.end < w.end)
        {
            self.found = Some((base.span, e.span));
        }
        walk_expr(self, e);
    }
}

/// `x % 2 == 1` made `x % 2 != 0`, and `x % 2 != 1` made `x % 2 == 0`.
fn odd(c: &Compiled, base: usize, text: &str, at: usize) -> Option<Edit> {
    let mut find = Comparison {
        at: (base + at) as u32,
        found: None,
    };
    walk_file(&mut find, &mut c.file.clone());
    let (op, left, whole) = find.found?;
    let new = match op {
        BinaryOp::Eq => "!= 0",
        _ => "== 0",
    };
    let local = |x: u32| (x as usize).checked_sub(base);
    let new = format!("{} {new}", &text[local(left.start)?..local(left.end)?]);
    Some(Edit {
        title: format!("Write `{new}`"),
        from: local(whole.start)?,
        to: local(whole.end)?,
        text: new,
        safe: true,
    })
}

/// The `==` or `!=` whose left side starts at `at`: its operator, its
/// left side's span and its own.
struct Comparison {
    at: u32,
    found: Option<(BinaryOp, Span, Span)>,
}

impl VisitMut for Comparison {
    fn expr(&mut self, e: &mut Expr) {
        if let ExprKind::Binary(op @ (BinaryOp::Eq | BinaryOp::NotEq), l, _) = &e.kind
            && l.span.start == self.at
        {
            self.found = Some((*op, l.span, e.span));
        }
        walk_expr(self, e);
    }
}

/// `uses …` added to the `pub` function around byte `at`: on the line of
/// its signature, or on a line of its own before its first clause.
fn add_uses(c: &Compiled, base: usize, text: &str, at: usize, uses: &str) -> Option<Edit> {
    let f = around(&c.file.items, (base + at) as u32)?;
    let local = |x: u32| (x as usize).checked_sub(base);
    let first_clause = (f.requires.iter().chain(&f.ensures))
        .map(|e| e.span.start)
        .chain(f.examples.iter().map(|x| x.expr.span.start))
        .min();
    let title = format!("Add `{uses}`");
    if let Some(clause) = first_clause {
        // The clause's own line: `uses` goes on a line before it.
        let clause = local(clause)?;
        let line = text[..clause].rfind('\n')? + 1;
        let indent: String = text[line..].chars().take_while(|c| *c == ' ').collect();
        let new = format!("{indent}{uses}\n");
        return Some(Edit {
            title,
            from: line,
            to: line,
            text: new,
            safe: true,
        });
    }
    let body = local(f.body.as_ref()?.span.start)?;
    let end = text[..body].trim_end().len();
    Some(Edit {
        title,
        from: end,
        to: end,
        text: format!(" {uses}"),
        safe: true,
    })
}

/// The function or method whose body holds `at`.
fn around(items: &[Item], at: u32) -> Option<&FnDecl> {
    items.iter().find_map(|item| match &item.kind {
        ItemKind::Fn(f) => {
            let s = f.body.as_ref()?.span;
            (s.start <= at && at < s.end).then_some(f)
        }
        ItemKind::Type(TypeDecl {
            body: TypeBody::Record(r),
            ..
        }) => around(&r.methods, at),
        ItemKind::Extend(e) => around(&e.methods, at),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    /// `src` after the fix for its first message, and whether that fix
    /// is safe.
    fn fixed(src: &str) -> (String, bool) {
        let c = kavach_driver::compile(src, "fix.kav", Default::default());
        let m = c.messages.first().expect("a message");
        // The program's own file, not a standard library module it uses.
        let file = c
            .files
            .iter()
            .find(|f| f.path == "fix.kav")
            .expect("fix.kav");
        let at = (m.span.start - file.start) as usize;
        let e = crate::fixes::fix(&c, "fix.kav", src, &m.text, at).expect("a fix");
        (
            format!("{}{}{}", &src[..e.from], e.text, &src[e.to..]),
            e.safe,
        )
    }

    #[test]
    fn each_suggestion_is_made() {
        let cases = [
            (
                "fn main() {\n    let x = 3\n    if x = 4 {\n        print(x)\n    }\n}\n",
                "if x == 4 {",
            ),
            (
                "fn show(xs: [Int]) {\n    for i in 0..=xs.len() {\n        print(xs[i])\n    }\n}\n\nfn main() {\n    show([1])\n}\n",
                "for i in 0..xs.len() {",
            ),
            (
                "fn main() {\n    const xs = [1]\n    xs.push(2)\n    print(xs)\n}\n",
                "    let xs = [1]\n",
            ),
            (
                "fn odd(x: Int) -> Bool {\n    x % 2 == 1\n}\n\nfn main() {\n    print(odd(3))\n}\n",
                "    x % 2 != 0\n",
            ),
            (
                "fn even(x: Int) -> Bool {\n    x % 2 != 1\n}\n\nfn main() {\n    print(even(4))\n}\n",
                "    x % 2 == 0\n",
            ),
            (
                // `Console` is a default capability (spec 8.1, rule 7), so
                // these use `Clock`, which a function must still write.
                "use std.time\n\npub fn hi() {\n    time.sleep(1.ms)\n}\n\nfn main() {\n    hi()\n}\n",
                "pub fn hi() uses Clock {\n",
            ),
            (
                "use std.time\n\npub fn hi(n: Int)\n    require n > 0\n{\n    time.sleep(1.ms)\n}\n\nfn main() {\n    hi(1)\n}\n",
                "pub fn hi(n: Int)\n    uses Clock\n    require n > 0\n{",
            ),
            (
                "use std.time\n\nfn shout(s: String) uses Fs {\n    print(s)\n    time.sleep(1.ms)\n}\n\nfn main() {\n    shout(\"x\")\n}\n",
                "fn shout(s: String) uses Fs, Clock {\n",
            ),
        ];
        for (src, want) in cases {
            let (out, safe) = fixed(src);
            assert!(out.contains(want), "{src}\n---\n{out}");
            assert!(safe, "{src}");
            let after = kavach_driver::compile(&out, "fix.kav", Default::default());
            assert!(after.messages.is_empty(), "{out}\n{:?}", after.messages);
        }
    }

    /// A `let` that hides an outer variable: made an assignment to it,
    /// offered, not applied.
    #[test]
    fn a_hiding_let_is_only_offered() {
        let src = "fn main() {\n    let total = 0\n    for i in 0..3 {\n        let total = total + i\n    }\n    print(total)\n}\n";
        let (out, safe) = fixed(src);
        assert!(out.contains("        total = total + i\n"), "{out}");
        assert!(!safe);
    }

    /// A `Drop` value given to `_` is named, to keep it to the block's
    /// end (spec 6.1); `.unwrap()` says why it can't be `none` (spec 2.7).
    /// Both change what the code does, or wait for words: offered only.
    #[test]
    fn fixes_that_change_the_code_are_only_offered() {
        let guard = "type Guard: Drop {\n    name: String\n\n    fn drop(self) {\n        print(self.name)\n    }\n}\n\nfn main() {\n    const _ = Guard(name: \"x\")\n    print(1)\n}\n";
        let (out, safe) = fixed(guard);
        assert!(
            out.contains("    const _guard = Guard(name: \"x\")\n"),
            "{out}"
        );
        assert!(!safe);
        let cases = [
            ("const y = x.unwrap()", "const y = x ?? panic(\"…\")"),
            ("print(x.unwrap())", "print(x ?? panic(\"…\"))"),
            ("print(x.unwrap() + 1)", "print((x ?? panic(\"…\")) + 1)"),
            ("print(1 == x.unwrap())", "print(1 == (x ?? panic(\"…\")))"),
        ];
        for (line, want) in cases {
            let src = format!(
                "fn find() -> Int? {{\n    none\n}}\n\nfn main() {{\n    const x = find()\n    {line}\n    print(y)\n}}\n"
            );
            let src = src.replace(
                "    print(y)\n",
                if line.starts_with("const") {
                    "    print(y)\n"
                } else {
                    ""
                },
            );
            let (out, safe) = fixed(&src);
            assert!(out.contains(want), "{out}");
            assert!(!safe);
        }
    }

    /// The closest name may not be the one meant: offered, not applied.
    #[test]
    fn a_misspelled_name_is_only_offered() {
        let (out, safe) = fixed("fn main() {\n    const total = 1\n    print(totl)\n}\n");
        assert!(out.contains("print(total)"), "{out}");
        assert!(!safe);
    }
}
