//! Module constants (spec 1.13): `const count = 10` at the top level of a
//! file, read by every function of its module. Each is checked before the
//! functions, as a function of its own, so what it needs is known, and it
//! may need no capability: it is worked out before `main` starts. In a
//! script (an entry file with no `fn main`), one that uses a capability
//! or `try` stays a statement of `main` instead, as 1.3 has it.

use crate::checker::Checker;
use crate::ty::Ty;
use kavach_ast::visit::{VisitMut, walk_expr};
use kavach_ast::{Expr, ExprKind, File, ItemKind, Stmt, StmtKind};
use kavach_resolver::DefId;

/// Finds a `try` in a constant's value.
struct FindsTry(bool);

impl VisitMut for FindsTry {
    fn expr(&mut self, e: &mut Expr) {
        self.0 |= matches!(e.kind, ExprKind::Try { .. });
        walk_expr(self, e);
    }
}

impl Checker<'_> {
    /// Checks each module constant of `file`, in order.
    pub(crate) fn check_module_consts(&mut self, file: &File) {
        let script = !file.items.iter().any(|i| {
            matches!(&i.kind, ItemKind::Fn(f) if f.name.name == "main"
                && self.res.module_at(f.name.span.start) == 0)
        });
        for item in &file.items {
            let ItemKind::Stmt(s) = &item.kind else {
                continue;
            };
            let Some(def) = self.module_const_def(s) else {
                continue;
            };
            // A script's constant is checked as `main`'s code is: it may
            // pass errors on with `try`.
            let in_script = script && self.res.module_at(s.span.start) == 0;
            let errors = in_script.then(crate::items::error_ty);
            if in_script {
                self.script_candidates.insert(def);
                if let StmtKind::Let { value, .. } = &s.kind {
                    let mut finds = FindsTry(false);
                    finds.expr(&mut value.clone());
                    if finds.0 {
                        self.script_consts.insert(def);
                    }
                }
            }
            let outer = self.current_fn.replace(def);
            self.with_ctx("a module constant", Ty::Unit, errors, None, |c| c.stmt(s));
            self.current_fn = outer;
        }
    }

    /// The definition of module constant `s`, or `None` for any other
    /// top-level statement.
    pub(crate) fn module_const_def(&self, s: &Stmt) -> Option<DefId> {
        let id = kavach_resolver::module_const(s)?;
        let def = *self.decl.get(&id.span.start)?;
        self.res.module_consts.contains(&def).then_some(def)
    }

    /// After capabilities are known: each module constant that uses one
    /// is a script's statement, or an error; and a script's statement is
    /// read by no function.
    pub(crate) fn settle_module_consts(
        &mut self,
        file: &File,
        caps: &dyn Fn(DefId) -> Option<String>,
    ) {
        let mut consts: Vec<DefId> = self.res.module_consts.iter().copied().collect();
        consts.sort_unstable();
        for d in consts {
            let Some(first) = caps(d) else {
                continue;
            };
            if self.script_candidates.contains(&d) {
                self.script_consts.insert(d);
                continue;
            }
            if let Some(span) = self.res.defs[d].span {
                let name = &self.res.defs[d].name;
                self.error(
                    span,
                    format!("`{name}` is a module constant, worked out before `main` starts, so it can't use a capability, and this uses `{first}` (spec 1.13): declare it with `let`, or work it out in `main`"),
                );
            }
        }
        let fns: Vec<(u32, u32)> = (file.items.iter())
            .filter(|i| matches!(i.kind, ItemKind::Fn(_)))
            .map(|i| (i.span.start, i.span.end))
            .collect();
        let mut late: Vec<(u32, DefId)> = (self.res.uses.iter())
            .filter(|(at, d)| {
                self.script_consts.contains(d) && fns.iter().any(|&(s, e)| **at >= s && **at < e)
            })
            .map(|(at, d)| (*at, *d))
            .collect();
        late.sort_unstable();
        for (at, d) in late {
            let name = self.res.defs[d].name.clone();
            let span = kavach_ast::Span {
                start: at,
                end: at + name.len() as u32,
            };
            self.error(
                span,
                format!("`{name}` is worked out by the script's own code, as it uses a capability or `try`, so a function can't read it (spec 1.13): pass it as an argument"),
            );
        }
    }
}
