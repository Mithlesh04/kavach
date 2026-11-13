//! Tagged literals (spec 10.15): a `literal` declaration's type and
//! checker, and each `sql"…"`, typed as its tag's type and kept, with its
//! text, for the checker to run on once the program is checked.

use crate::checker::{Checker, Shape};
use crate::ty::{Prim, Ty};
use kavach_ast::*;
use kavach_resolver::{DefId, DefKind};

/// A tag: the type its literals make, and the function checking them.
#[derive(Clone, Debug)]
pub(crate) struct LiteralTag {
    pub(crate) ty: Ty,
    pub(crate) check: Option<DefId>,
}

/// One tagged literal, for its tag's checker to run on when the program
/// is compiled.
#[derive(Clone, Debug)]
pub struct LiteralUse {
    pub tag: String,
    /// The checking function.
    pub check: DefId,
    pub text: String,
    /// The string after the tag, quotes included.
    pub span: Span,
}

impl Checker<'_> {
    pub(crate) fn collect_literal(&mut self, l: &LiteralDecl) {
        let Some(&def) = self.decl.get(&l.name.span.start) else {
            return;
        };
        let ty = self.lower(&l.ty, None);
        let check = self.res.uses.get(&l.check.span.start).copied();
        self.literal_tags.insert(def, LiteralTag { ty, check });
    }

    /// The type is a record whose first field is its text; the checker
    /// takes the text and gives `[LiteralProblem]`, using nothing.
    pub(crate) fn check_literal(&mut self, l: &LiteralDecl) {
        let Some(tag) = self
            .decl
            .get(&l.name.span.start)
            .and_then(|d| self.literal_tags.get(d))
            .cloned()
        else {
            return;
        };
        let holds_text = match &tag.ty {
            Ty::Adt(def, _) => matches!(
                self.types.get(def).map(|t| &t.shape),
                Some(Shape::Record(fields)) if fields.first().is_some_and(|f| f.ty == Ty::Prim(Prim::String))
            ),
            _ => false,
        };
        if !holds_text {
            self.error(l.ty.span, format!("the tag `{}` makes a record whose first field is its text, a `String` (spec 10.15)", l.name.name));
        }
        let problems = Ty::List(Box::new(self.builtin_ty("LiteralProblem")));
        let fits = tag.check.and_then(|c| {
            (self.res.defs[c].kind == DefKind::Fn)
                .then(|| self.fns.get(&c).cloned())
                .flatten()
        });
        let fits = fits.is_some_and(|sig| {
            sig.params.len() == 1
                && sig.params[0].ty == Ty::Prim(Prim::String)
                && sig.ret == problems
                && sig.errors.is_none()
                && sig.uses.is_empty()
                && sig.generics.is_empty()
        });
        if !fits {
            self.error(l.check.span, format!("`{}` checks the text of `{}` literals: it must be `fn {}(text: String) -> [LiteralProblem]`, with no `!` and no `uses`, as it runs while the program is compiled (spec 10.15)", l.check.name, l.name.name, l.check.name));
        }
    }

    /// `sql"…"`: a value of the tag's type, its text kept for the check.
    pub(crate) fn tagged(&mut self, tag: &Path, args: &[Arg]) -> Ty {
        let Some(last) = tag.segments.last() else {
            return Ty::Error;
        };
        let def = self.res.uses.get(&last.span.start).copied();
        let Some((def, found)) =
            def.and_then(|d| self.literal_tags.get(&d).cloned().map(|t| (d, t)))
        else {
            if def.is_some() {
                self.error(last.span, format!("`{}` isn't a tag for literals: a package declares one with `literal` (spec 10.15)", last.name));
            }
            return Ty::Error;
        };
        let Some(arg) = args.first() else {
            return Ty::Error;
        };
        // Typed, so code generation knows the text is a `String`.
        self.value(&arg.value, Some(&Ty::Prim(Prim::String)));
        let text = match &arg.value.kind {
            ExprKind::Str(parts) => parts
                .iter()
                .filter_map(|p| match p {
                    StrPart::Text(t) => Some(t.as_str()),
                    StrPart::Expr(_) => None,
                })
                .collect(),
            _ => String::new(),
        };
        if let Some(check) = found.check {
            self.literal_uses.push(LiteralUse {
                tag: self.res.defs[def].name.clone(),
                check,
                text,
                span: arg.value.span,
            });
        }
        found.ty
    }
}
