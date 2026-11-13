//! `kavach audit` (spec 10.5): list unsafe code, revealed secrets, calls
//! of `audited fn`s (spec 10.15) and capability use across the program
//! and everything it pulls in, so a review sees every place the safety
//! promises are loosened.

use kavach_ast::visit::{VisitMut, walk_expr, walk_file};
use kavach_ast::{Expr, ExprKind, ItemKind, Span};
use kavach_driver::Compiled;
use std::process::ExitCode;

pub(crate) fn audit(path: &str) -> ExitCode {
    let c = match crate::load(path) {
        Ok(c) => c,
        Err(code) => return code,
    };
    let mut found = Finder::default();
    let mut file = c.file.clone();
    walk_file(&mut found, &mut file);

    println!("unsafe code:");
    let mut lines = Vec::new();
    for item in &c.file.items {
        if let ItemKind::Fn(f) = &item.kind
            && let Some(lib) = &f.c_library
        {
            lines.push((
                f.name.span,
                format!("extern c \"{lib}\" fn {}", f.name.name),
            ));
        }
    }
    for span in &found.unsafes {
        lines.push((*span, "unsafe block".to_string()));
    }
    list(&c, lines);

    println!("revealed secrets:");
    let lines = found
        .reveals
        .iter()
        .map(|span| (*span, "reveal(…)".to_string()))
        .collect();
    list(&c, lines);

    // Text made into a tagged literal's type while the program runs.
    println!("audited calls:");
    let audited: std::collections::HashMap<usize, &str> = c
        .file
        .items
        .iter()
        .filter_map(|item| match &item.kind {
            ItemKind::Fn(f) if f.is_audited => Some(&f.name),
            _ => None,
        })
        .filter_map(|name| {
            let def = c
                .resolution
                .defs
                .iter()
                .position(|d| d.span == Some(name.span))?;
            Some((def, name.name.as_str()))
        })
        .collect();
    let lines = found
        .calls
        .iter()
        .filter_map(|(callee, span)| {
            let def = c.resolution.uses.get(callee)?;
            audited.get(def).map(|name| (*span, format!("{name}(…)")))
        })
        .collect();
    list(&c, lines);

    println!("capability use:");
    let mut lines = Vec::new();
    for (def, uses) in &c.uses {
        if uses.is_empty() {
            continue;
        }
        let d = &c.resolution.defs[*def];
        let Some(span) = d.span else { continue };
        lines.push((span, format!("fn {}: {}", d.name, uses.join(", "))));
    }
    list(&c, lines);
    ExitCode::SUCCESS
}

/// Prints `lines` sorted by place, as `  path:line  what`.
fn list(c: &Compiled, mut lines: Vec<(Span, String)>) {
    if lines.is_empty() {
        println!("  nothing");
        return;
    }
    lines.sort_by_key(|(span, _)| span.start);
    for (span, what) in lines {
        let (path, line) = c.location(span.start);
        // Relative to where `kavach` runs, so the list is the same on
        // every machine.
        let path = kavach_driver::relative_path(std::path::Path::new(&path));
        println!("  {path}:{line}  {what}");
    }
}

#[derive(Default)]
struct Finder {
    unsafes: Vec<Span>,
    reveals: Vec<Span>,
    /// Each call: where its callee's name starts, and the call.
    calls: Vec<(u32, Span)>,
}

impl VisitMut for Finder {
    fn expr(&mut self, e: &mut Expr) {
        match &e.kind {
            ExprKind::Unsafe(_) => self.unsafes.push(e.span),
            ExprKind::Call { callee, .. } => {
                if matches!(&callee.kind, ExprKind::Name(n) if n.name == "reveal") {
                    self.reveals.push(e.span);
                }
                match &callee.kind {
                    ExprKind::Name(n) | ExprKind::Field { name: n, .. } => {
                        self.calls.push((n.span.start, e.span));
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        walk_expr(self, e);
    }
}
