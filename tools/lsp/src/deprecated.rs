//! The fix for a call to a deprecated function (spec 12.6): the call
//! becomes the function's `fix` expression, its parameters replaced by
//! the call's arguments.

use crate::fixes::Edit;
use kavach_ast::visit::{VisitMut, walk_expr, walk_file};
use kavach_ast::*;
use kavach_driver::Compiled;
use kavach_resolver::DefKind;
use std::collections::HashSet;

/// The fix for the call whose callee starts at byte `at` of the file
/// that starts at byte `base` of the source.
pub(crate) fn rewrite(c: &Compiled, base: usize, at: usize) -> Option<Edit> {
    let mut find = FindCall {
        at: (base + at) as u32,
        call: None,
    };
    walk_file(&mut find, &mut c.file.clone());
    let (callee, name, args, span) = find.call?;
    let res = &c.resolution;
    let def = *res.uses.get(&name.start)?;
    let declared = res.defs[def].span?;
    let f = c.file.items.iter().find_map(|i| match &i.kind {
        ItemKind::Fn(f) if f.name.span == declared => Some(f),
        _ => None,
    })?;
    let fix = f.deprecated.as_ref()?.fix.as_ref()?;
    // Each parameter's text: its argument, or else its default.
    let params: Vec<&Param> = f.params.iter().filter(|p| p.name.name != "self").collect();
    let mut positional = args.iter().filter(|a| a.name.is_none());
    let mut texts = Vec::new();
    for p in &params {
        let arg = match args
            .iter()
            .find(|a| a.name.as_ref().is_some_and(|n| n.name == p.name.name))
        {
            Some(a) => Some(&a.value),
            None => positional.next().map(|a| &a.value),
        };
        let value = arg.or(p.default.as_ref())?;
        texts.push((p.name.span, value));
    }
    // `billing.refund(o)`: names of the function's module get `billing.`.
    let called = callee.text(&c.src);
    let module = called.rfind('.').map_or("", |i| &called[..=i]);
    let mut names = Names {
        found: Vec::new(),
        alone: HashSet::from([(fix.span.start, fix.span.end)]),
    };
    names.expr(&mut fix.clone());
    let mut edits: Vec<(Span, String)> = Vec::new();
    for id in names.found {
        let Some(&d) = res.uses.get(&id.span.start) else {
            continue;
        };
        let def = &res.defs[d];
        if let Some((_, value)) = texts.iter().find(|(s, _)| Some(*s) == def.span) {
            let text = value.span.text(&c.src);
            let text = match names.alone.contains(&(id.span.start, id.span.end)) {
                true => text.to_string(),
                false => grouped(value, text),
            };
            edits.push((id.span, text));
        } else if !module.is_empty() && matches!(def.kind, DefKind::Fn | DefKind::Type) {
            edits.push((id.span, format!("{module}{}", id.name)));
        }
    }
    let mut new = fix.span.text(&c.src).to_string();
    edits.sort_by_key(|(s, _)| std::cmp::Reverse(s.start));
    for (s, text) in edits {
        let (a, b) = (
            (s.start - fix.span.start) as usize,
            (s.end - fix.span.start) as usize,
        );
        new.replace_range(a..b, &text);
    }
    Some(Edit {
        title: format!("Replace with `{new}`"),
        from: (span.start as usize).checked_sub(base)?,
        to: (span.end as usize).checked_sub(base)?,
        text: new,
        safe: true,
    })
}

/// `text` of `e`, in parentheses unless it is one piece already.
fn grouped(e: &Expr, text: &str) -> String {
    match e.kind {
        ExprKind::Name(_)
        | ExprKind::Int(_)
        | ExprKind::Float(_)
        | ExprKind::Str(_)
        | ExprKind::Bool(_)
        | ExprKind::Call { .. }
        | ExprKind::Field { .. }
        | ExprKind::Index { .. }
        | ExprKind::List(_)
        | ExprKind::Tuple(_) => text.to_string(),
        _ => format!("({text})"),
    }
}

/// The call whose callee starts at `at`: the callee's span (from the
/// module, in `billing.refund`), its name's, the arguments and the
/// call's span.
struct FindCall {
    at: u32,
    call: Option<(Span, Span, Vec<Arg>, Span)>,
}

impl VisitMut for FindCall {
    fn expr(&mut self, e: &mut Expr) {
        if let ExprKind::Call { callee, args, .. } = &e.kind
            && let (true, ExprKind::Name(id)) = (callee.span.start == self.at, &callee.kind)
        {
            self.call = Some((callee.span, id.span, args.clone(), e.span));
        }
        walk_expr(self, e);
    }
}

/// Every name an expression reads, and where an expression stands
/// alone: the whole of it, or a call's argument. An argument put there
/// needs no parentheses.
struct Names {
    found: Vec<Ident>,
    alone: HashSet<(u32, u32)>,
}

impl VisitMut for Names {
    fn expr(&mut self, e: &mut Expr) {
        match &e.kind {
            ExprKind::Name(id) => self.found.push(id.clone()),
            ExprKind::Call { args, .. } => {
                (self.alone).extend(args.iter().map(|a| (a.value.span.start, a.value.span.end)));
            }
            _ => {}
        }
        walk_expr(self, e);
    }
}

#[cfg(test)]
mod tests {
    const HALF: &str = "deprecated \"use half_of instead\" fix half_of(n * 2, by: by)\nfn half(n: Int, by: Int = 4) -> Int {\n    n / by\n}\n\nfn half_of(n: Int, by: Int) -> Int {\n    n / by\n}\n\n";

    /// What the call on the line `call` becomes.
    fn rewritten(call: &str) -> String {
        let src = format!("{HALF}fn main() {{\n    print({call})\n}}\n");
        let c = kavach_driver::compile(&src, "fix.kav", Default::default());
        let at = src.rfind(call).unwrap();
        let e = crate::fixes::fix(
            &c,
            "fix.kav",
            &src,
            "`half` is deprecated: use half_of instead",
            at,
        )
        .unwrap();
        e.text
    }

    #[test]
    fn calls_become_the_fix_with_their_arguments() {
        let cases = [
            ("half(10)", "half_of(10 * 2, by: 4)"),
            ("half(10, 3)", "half_of(10 * 2, by: 3)"),
            ("half(by: 5, n: 1)", "half_of(1 * 2, by: 5)"),
            ("half(3 + 4, by: 1 + 1)", "half_of((3 + 4) * 2, by: 1 + 1)"),
            ("half(f(2))", "half_of(f(2) * 2, by: 4)"),
        ];
        for (call, want) in cases {
            assert_eq!(rewritten(call), want, "{call}");
        }
    }

    /// Through its module, the fix's own names keep the module; through
    /// an import, `half_of` isn't in scope, so the call is left.
    #[test]
    fn a_project_call_keeps_its_module_or_is_left() {
        let root = std::env::temp_dir().join(format!("kavach-deprecated-{}", std::process::id()));
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("kavach.toml"), "[package]\nname = \"depp\"\n").unwrap();
        std::fs::write(
            root.join("src").join("billing.kav"),
            "deprecated \"use half_of instead\" fix half_of(n)\npub fn half(n: Int) -> Int {\n    n / 2\n}\n\npub fn half_of(n: Int) -> Int {\n    n / 2\n}\n",
        )
        .unwrap();
        let main = root.join("src").join("main.kav");
        let src = "use billing\nuse billing.{half}\n\nfn main() {\n    print(billing.half(4))\n    print(half(6))\n}\n";
        std::fs::write(&main, src).unwrap();
        let done = crate::fix_file(&main.to_string_lossy()).unwrap();
        let text = std::fs::read_to_string(&main).unwrap();
        std::fs::remove_dir_all(&root).unwrap();
        assert_eq!(done, ["Replace with `billing.half_of(4)`"]);
        assert_eq!(text, src.replace("billing.half(4)", "billing.half_of(4)"));
    }
}
