//! `cli.parse<T>()` (spec 14.6): the command line read into a record of
//! options, or a union of commands.

use crate::checker::{Checker, Shape};
use crate::stdlib::number;
use crate::ty::{Prim, Ty};
use kavach_ast::{Arg, Span, TypeExpr};

impl Checker<'_> {
    /// `cli.parse<T>()`: `T` must be a record of options, or a union of
    /// commands whose fields are options (spec 14.6).
    pub(crate) fn cli_parse(&mut self, generics: &[TypeExpr], args: &[Arg], span: Span) -> Ty {
        self.note_cap("Env", span);
        if !args.is_empty() {
            self.error(span, "`cli.parse` takes no arguments: write the options as a record type, `cli.parse<Args>()`");
        }
        let Some(t) = generics.first().map(|g| self.lower(g, None)) else {
            self.error(
                span,
                "`cli.parse` needs the type to parse into: `cli.parse<Args>()`",
            );
            return Ty::Error;
        };
        let shape = match &t {
            Ty::Adt(def, _) => self.types.get(def).map(|i| &i.shape),
            _ => None,
        };
        let fields = match shape {
            Some(Shape::Record(fields)) if !fields.is_empty() => fields.clone(),
            Some(Shape::Union(variants)) => {
                variants.iter().flat_map(|v| v.fields.clone()).collect()
            }
            _ => {
                self.error(span, "`cli.parse` needs a record whose fields are the options, or a union whose variants are the commands");
                Vec::new()
            }
        };
        for f in &fields {
            let inner = match &f.ty {
                Ty::Optional(t) => (**t).clone(),
                t => t.clone(),
            };
            if !(number(&inner) || matches!(inner, Ty::Prim(Prim::String | Prim::Bool))) {
                let shown = self.show(&f.ty);
                self.error(span, format!("option `{}` is `{shown}`: options can be text, numbers, `Bool` or optionals of these", f.name));
            }
        }
        Ty::Result(Box::new(t), Box::new(Ty::Builtin("CliError", Vec::new())))
    }
}
