//! Text views and builders (spec 2.18): the methods of `StrView` and
//! `TextBuilder`, and `s.view()` on a `String`.

use crate::builtins::MethodSig;
use crate::checker::Checker;
use crate::ty::{Prim, Ty};
use kavach_ast::Arg;

/// `StrView`.
pub(crate) fn view_ty() -> Ty {
    Ty::Builtin("StrView", Vec::new())
}

/// Whether `t` is text a view or builder method takes: a `String` or a
/// `StrView`.
pub(crate) fn is_text(t: &Ty) -> bool {
    matches!(t, Ty::Prim(Prim::String) | Ty::Builtin("StrView", _))
}

impl Checker<'_> {
    /// The methods of views and builders, and `s.view(start, end)`.
    pub(crate) fn view_method(&mut self, recv: &Ty, name: &str) -> Option<MethodSig> {
        let int = Ty::Prim(Prim::Int);
        let boolean = Ty::Prim(Prim::Bool);
        let string = Ty::Prim(Prim::String);
        let view = view_ty();
        let sig = |params: Vec<Ty>, ret: Ty| {
            Some(MethodSig {
                params,
                ret,
                named: Vec::new(),
            })
        };
        let views = Ty::List(Box::new(view.clone()));
        match (recv, name) {
            (Ty::Prim(Prim::String), "view") => sig(vec![int.clone(), int], view),
            (Ty::Builtin("StrView", _), _) => match name {
                "len" => sig(vec![], int),
                "is_empty" => sig(vec![], boolean),
                "view" => sig(vec![int.clone(), int], view),
                "trim" | "trim_start" | "trim_end" => sig(vec![], view),
                "split" => sig(vec![string], views),
                "lines" => sig(vec![], views),
                "find" => sig(vec![string], Ty::Optional(Box::new(int))),
                "starts_with" | "ends_with" | "contains" => sig(vec![string], boolean),
                "to_string" => sig(vec![], string),
                _ => None,
            },
            (Ty::Builtin("TextBuilder", _), _) => match name {
                "push" => sig(vec![string], Ty::Unit),
                "push_char" => sig(vec![Ty::Prim(Prim::Char)], Ty::Unit),
                "insert" => sig(vec![int, string], Ty::Unit),
                "replace" => sig(vec![int.clone(), int, string], Ty::Unit),
                "remove" => sig(vec![int.clone(), int], Ty::Unit),
                "clear" => sig(vec![], Ty::Unit),
                "len" => sig(vec![], int),
                "is_empty" => sig(vec![], boolean),
                "view" => sig(vec![], view),
                "to_string" => sig(vec![], string),
                _ => None,
            },
            _ => None,
        }
    }

    /// The calls a signature can't say: `s.view()` with no positions, and
    /// text given as a `String` or a `StrView` to a builder.
    pub(crate) fn view_call(&mut self, recv: &Ty, name: &str, args: &[Arg]) -> Option<Ty> {
        let text_like = matches!(
            recv,
            Ty::Prim(Prim::String) | Ty::Builtin("StrView" | "TextBuilder", _)
        );
        if text_like && name == "view" && args.is_empty() {
            return Some(view_ty());
        }
        let builder = matches!(recv, Ty::Builtin("TextBuilder", _));
        let at = match name {
            "push" => 0,
            "insert" => 1,
            "replace" => 2,
            _ => return None,
        };
        if !builder || args.len() != at + 1 || args.iter().any(|a| a.name.is_some()) {
            return None;
        }
        let int = Ty::Prim(Prim::Int);
        for a in &args[..at] {
            let found = self.value(&a.value, None);
            if !self.coerce(&found, &int) {
                self.mismatch(&a.value, &int, &found);
            }
        }
        let text = &args[at].value;
        let found = self.value(text, None);
        let found = self.inf.resolve(&found);
        if !is_text(&found) && !self.coerce(&found, &Ty::Prim(Prim::String)) {
            let shown = self.show(&found);
            self.error(
                text.span,
                format!(
                    "`{name}` takes a `String` or a `StrView`, but this is `{shown}` (spec 2.18)"
                ),
            );
        }
        Some(Ty::Unit)
    }

    /// `v == "text"`, `v < w`: when one side is a view, the other may be
    /// a view or a `String`. False when `left` is neither, and `right`
    /// is still to be checked.
    pub(crate) fn compare_text(&mut self, left: &Ty, right: &kavach_ast::Expr) -> bool {
        let left = self.inf.resolve(left);
        if !is_text(&left) {
            return false;
        }
        let view = matches!(left, Ty::Builtin("StrView", _));
        let found = self.value(right, if view { None } else { Some(&left) });
        let found = self.inf.resolve(&found);
        let fits = is_text(&found) || matches!(found, Ty::Error | Ty::Never);
        if !fits && !self.coerce(&found, &left) {
            self.mismatch(right, &left, &found);
        }
        true
    }
}
