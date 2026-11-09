//! Destructuring (spec 1.12): one part of a list, map or record, for a
//! name or a nested pattern. A missing part, or `none` where there is a
//! default or a nested pattern, takes the default; without one, every
//! name inside it is `none`.

use crate::interp::Interp;
use crate::value::{Flow, Value};
use kavach_ast::{Expr, ListItem, MapKey, Pattern, PatternKind};
use kavach_resolver::DefId;
use std::rc::Rc;

type Bound<'a> = Vec<(DefId, Value<'a>)>;

impl<'a> Interp<'a> {
    pub(crate) fn take_part(
        &mut self,
        pat: &'a Pattern,
        part: Option<Value<'a>>,
        default: Option<&'a Expr>,
        out: &mut Bound<'a>,
    ) -> Result<(), Flow<'a>> {
        let nested = matches!(pat.kind, PatternKind::List { .. } | PatternKind::Record(_));
        let part = match part {
            Some(Value::None) if default.is_some() || nested => None,
            other => other,
        };
        let value = match (part, default) {
            (Some(v), _) => v,
            (None, Some(d)) => self.eval(d)?,
            (None, None) => return self.bind_missing(pat, out),
        };
        self.match_into(pat, &value, out)?;
        Ok(())
    }

    /// Every name in `pat`, for a part that isn't there: its default, or
    /// `none`.
    fn bind_missing(&mut self, pat: &'a Pattern, out: &mut Bound<'a>) -> Result<(), Flow<'a>> {
        match &pat.kind {
            PatternKind::Binding(id) => {
                if let Some(&def) = self.decl.get(&id.span.start) {
                    out.push((def, Value::None));
                }
            }
            PatternKind::List { items, rest } => {
                for it in items {
                    self.take_part(&it.pattern, None, it.default.as_ref(), out)?;
                }
                if let Some(def) = rest.as_ref().and_then(|r| self.decl.get(&r.span.start)) {
                    out.push((*def, Value::List(Rc::new(Vec::new()))));
                }
            }
            PatternKind::Record(fields) => {
                for f in fields {
                    self.take_part(&f.pattern, None, f.default.as_ref(), out)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// The key an item of a map pattern reads: written, or a bare name's
    /// text. `None` for an item without either.
    pub(crate) fn map_key(&mut self, it: &'a ListItem) -> Result<Option<Value<'a>>, Flow<'a>> {
        Ok(match &it.key {
            Some(MapKey::Literal(e)) => Some(self.eval(e)?),
            Some(MapKey::Name(id)) => Some(self.str_value(&id.name)),
            None => match &it.pattern.kind {
                PatternKind::Binding(id) => Some(self.str_value(&id.name)),
                _ => None,
            },
        })
    }
}
