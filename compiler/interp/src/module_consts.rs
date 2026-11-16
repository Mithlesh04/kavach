//! Module constants (spec 1.13): `const count = 10` at the top level of a
//! file. Each is worked out the first time it is read and kept; `main`'s
//! run works them all out first, so a panic in one comes before `main`.

use crate::interp::Interp;
use crate::value::Value;
use kavach_ast::{ItemKind, Span, StmtKind};
use kavach_resolver::DefId;

impl<'a> Interp<'a> {
    /// The value of module constant `def`, worked out now if not yet.
    pub(crate) fn module_const(
        &mut self,
        def: DefId,
        span: Span,
    ) -> Result<Value<'a>, crate::value::Flow<'a>> {
        if let Some(v) = self.consts.get(&def) {
            return Ok(v.clone());
        }
        let at = self.c.resolution.defs[def].span;
        let value = self.c.file.items.iter().find_map(|item| match &item.kind {
            ItemKind::Stmt(s) => match (&s.kind, kavach_resolver::module_const(s)) {
                (StmtKind::Let { value, .. }, Some(id)) if Some(id.span) == at => Some(value),
                _ => None,
            },
            _ => None,
        });
        let Some(value) = value else {
            let name = &self.c.resolution.defs[def].name;
            return Err(self.panic(format!("`{name}` has no value yet"), span));
        };
        if !self.consts_busy.insert(def) {
            let name = &self.c.resolution.defs[def].name;
            return Err(self.panic(
                format!("module constant `{name}` is worked out from itself"),
                span,
            ));
        }
        let made = self.eval(value);
        self.consts_busy.remove(&def);
        let v = made?;
        self.consts.insert(def, v.clone());
        Ok(v)
    }

    /// Works out every module constant, in the order they are written.
    pub(crate) fn module_consts_first(&mut self) -> Result<(), crate::value::Flow<'a>> {
        let mut defs: Vec<DefId> = self.c.resolution.module_consts.iter().copied().collect();
        defs.sort_by_key(|d| self.c.resolution.defs[*d].span.map(|s| s.start));
        for d in defs {
            let span = self.c.resolution.defs[d]
                .span
                .unwrap_or(Span { start: 0, end: 0 });
            self.module_const(d, span)?;
        }
        Ok(())
    }
}
