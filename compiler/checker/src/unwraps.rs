//! `.unwrap()` outside tests (spec 2.7): a warning, as it panics on
//! `none` with no word of why. `test` blocks and `example` clauses may
//! use it freely. A `.unwrap()` the compiler knows always panics is an
//! error instead (`always_panics.rs`), so it gets no warning too.

use crate::checker::Checker;
use crate::ty::Ty;
use kavach_ast::visit::{VisitMut, walk_expr, walk_item};
use kavach_ast::*;
use std::collections::HashSet;

const UNWRAP: &str = "`.unwrap()` panics on `none` and says nothing of why. Handle `none` with `if let`, `match` or `?? default`, or say why it can't happen: `?? panic(\"…\")` (spec 2.7)";

impl Checker<'_> {
    pub(crate) fn check_unwraps(&mut self, file: &File) {
        let optionals = self
            .expr_types
            .iter()
            .filter(|(_, t)| matches!(t, Ty::Optional(_)))
            .map(|(s, _)| (s.start, s.end))
            .collect();
        let mut found = Unwraps {
            optionals,
            spans: Vec::new(),
        };
        for item in &file.items {
            if matches!(item.kind, ItemKind::Test(_)) {
                continue;
            }
            let mut item = item.clone();
            no_examples(&mut item);
            walk_item(&mut found, &mut item);
        }
        for span in found.spans {
            if !self.errors.iter().any(|d| d.span == span) {
                self.warn(span, UNWRAP);
            }
        }
    }
}

/// `item` without its functions' `example` clauses.
fn no_examples(item: &mut Item) {
    let methods = match &mut item.kind {
        ItemKind::Fn(f) => {
            f.examples.clear();
            return;
        }
        ItemKind::Type(t) => match &mut t.body {
            TypeBody::Record(r) => &mut r.methods,
            _ => return,
        },
        ItemKind::Interface(i) => &mut i.methods,
        ItemKind::Extend(e) => &mut e.methods,
        ItemKind::Capability(c) => &mut c.methods,
        ItemKind::Agent(a) => {
            for m in &mut a.members {
                if let AgentMember::On(f) = m {
                    f.examples.clear();
                }
            }
            return;
        }
        _ => return,
    };
    methods.iter_mut().for_each(no_examples);
}

struct Unwraps {
    /// The spans of the expressions whose type is optional.
    optionals: HashSet<(u32, u32)>,
    spans: Vec<Span>,
}

impl VisitMut for Unwraps {
    fn expr(&mut self, e: &mut Expr) {
        if let ExprKind::Call { callee, args, .. } = &e.kind
            && let ExprKind::Field {
                base,
                name,
                optional: false,
            } = &callee.kind
            && name.name == "unwrap"
            && args.is_empty()
            && self.optionals.contains(&(base.span.start, base.span.end))
        {
            self.spans.push(e.span);
        }
        walk_expr(self, e);
    }
}
