//! `kavach outline` (spec 13.3): every declaration as written, with
//! function bodies replaced by `{ … }`. Doc comments, signatures,
//! contracts and examples stay: they are what callers need.

use kavach_ast::{AgentMember, Item, ItemKind, Span, TypeBody};
use kavach_driver::Compiled;

/// The outline of a whole file. Top-level statements are summarized.
pub fn outline(c: &Compiled) -> String {
    let mut parts = Vec::new();
    let mut statements = 0;
    for item in c.own_items() {
        match &item.kind {
            ItemKind::Stmt(_) => statements += 1,
            _ => parts.push(outline_item(c, item)),
        }
    }
    if statements > 0 {
        parts.push(format!(
            "// {statements} top-level statement(s) run as `main`"
        ));
    }
    let mut out = parts.join("\n\n");
    out.push('\n');
    out
}

/// One declaration, with every function body inside it elided.
pub fn outline_item(c: &Compiled, item: &Item) -> String {
    let mut bodies = Vec::new();
    collect_bodies(item, &mut bodies);
    bodies.sort_by_key(|s| s.start);
    let (start, end) = (item.span.start as usize, item.span.end as usize);
    let mut out = String::new();
    let mut at = start;
    for body in bodies {
        let (b0, b1) = (body.start as usize, body.end as usize);
        if b0 < at || b1 > end {
            continue;
        }
        out.push_str(&c.src[at..b0]);
        out.push_str("{ … }");
        at = b1;
    }
    out.push_str(&c.src[at..end]);
    out.trim_end().to_string()
}

fn collect_bodies(item: &Item, out: &mut Vec<Span>) {
    let fn_body =
        |f: &kavach_ast::FnDecl, out: &mut Vec<Span>| out.extend(f.body.as_ref().map(|b| b.span));
    let methods = |items: &[Item], out: &mut Vec<Span>| {
        for m in items {
            collect_bodies(m, out);
        }
    };
    match &item.kind {
        ItemKind::Fn(f) => fn_body(f, out),
        ItemKind::Type(t) => {
            if let TypeBody::Record(r) = &t.body {
                methods(&r.methods, out);
            }
        }
        ItemKind::Extend(e) => methods(&e.methods, out),
        ItemKind::Interface(i) => methods(&i.methods, out),
        ItemKind::Capability(cap) => methods(&cap.methods, out),
        ItemKind::Agent(a) => {
            for m in &a.members {
                if let AgentMember::On(f) = m {
                    fn_body(f, out);
                }
            }
        }
        ItemKind::Test(t) => out.push(t.body.span),
        ItemKind::Use(_) | ItemKind::Stmt(_) => {}
    }
}
