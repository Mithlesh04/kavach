//! Calls to a deprecated function (spec 12.6): a warning that shows the
//! message. `kavach fix` and the editor's quick fix rewrite the call with
//! the `fix` expression (`tools/lsp/src/fixes.rs`).

use crate::checker::Checker;
use kavach_ast::visit::{VisitMut, walk_expr, walk_file};
use kavach_ast::*;
use kavach_resolver::DefId;
use std::collections::HashMap;

impl Checker<'_> {
    pub(crate) fn check_deprecated(&mut self, file: &File) {
        let mut found = Calls {
            checker: self,
            deprecated: HashMap::new(),
            warnings: Vec::new(),
        };
        for item in &file.items {
            if let ItemKind::Fn(f) = &item.kind
                && let (Some(d), Some(&def)) =
                    (&f.deprecated, found.checker.decl.get(&f.name.span.start))
            {
                found
                    .deprecated
                    .insert(def, (f.name.name.clone(), d.message.clone()));
            }
        }
        if found.deprecated.is_empty() {
            return;
        }
        walk_file(&mut found, &mut file.clone());
        for (span, message) in std::mem::take(&mut found.warnings) {
            self.warn(span, &message);
        }
    }
}

struct Calls<'c, 'r> {
    checker: &'c Checker<'r>,
    /// Each deprecated function's name and message.
    deprecated: HashMap<DefId, (String, String)>,
    warnings: Vec<(Span, String)>,
}

impl VisitMut for Calls<'_, '_> {
    fn expr(&mut self, e: &mut Expr) {
        if let ExprKind::Call { callee, .. } = &e.kind
            && let ExprKind::Name(id) = &callee.kind
        {
            let def = self.checker.res.uses.get(&id.span.start);
            if let Some((name, message)) = def.and_then(|d| self.deprecated.get(d)) {
                self.warnings.push((
                    callee.span,
                    format!("`{name}` is deprecated: {message} (spec 12.6)"),
                ));
            }
        }
        walk_expr(self, e);
    }
}
