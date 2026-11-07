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
        let at = (m.span.start - c.files[0].start) as usize;
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
                "pub fn hi() {\n    print(\"hi\")\n}\n\nfn main() {\n    hi()\n}\n",
                "pub fn hi() uses Console {\n",
            ),
            (
                "pub fn hi(n: Int)\n    require n > 0\n{\n    print(n)\n}\n\nfn main() {\n    hi(1)\n}\n",
                "pub fn hi(n: Int)\n    uses Console\n    require n > 0\n{",
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

    /// The closest name may not be the one meant: offered, not applied.
    #[test]
    fn a_misspelled_name_is_only_offered() {
        let (out, safe) = fixed("fn main() {\n    const total = 1\n    print(totl)\n}\n");
        assert!(out.contains("print(total)"), "{out}");
        assert!(!safe);
    }
}
