// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! A tagged template's parameters (spec 10.15, decision 0142): in
//! `` sql`… where owner = {owner}` `` each `{…}` is a value the tag's
//! record holds in its `params: [P]`, never pasted into the text. Each is
//! converted to `P` here, and how is kept for both back ends.

use crate::checker::{Checker, Shape};
use crate::literals::LiteralTag;
use crate::ty::{Prim, Ty};
use kavach_ast::*;
use kavach_resolver::{DefId, DefKind};

/// One `{…}` of a tagged template.
#[derive(Clone, Debug)]
pub struct Hole {
    /// The expression inside the braces.
    pub span: Span,
    pub conv: ParamConv,
}

/// How a parameter's value becomes the tag's parameter type `P`.
#[derive(Clone, Debug, PartialEq)]
pub enum ParamConv {
    /// The value is a `P` already.
    Same,
    /// `P` is `Json`: converted as an item of a `Json` list (spec 2.12).
    Json,
    /// The one variant of union `P` whose only field has the value's type.
    Variant { union: DefId, name: String },
}

impl Checker<'_> {
    /// `param p` on a tag: `fn p(n: Int) -> String`, pure, and the tag's
    /// record holds the values in `params: [P]`.
    pub(crate) fn check_param(&mut self, l: &LiteralDecl, p: &Ident, tag: &LiteralTag) {
        let sig = tag.param.and_then(|d| {
            (self.res.defs[d].kind == DefKind::Fn)
                .then(|| self.fns.get(&d).cloned())
                .flatten()
        });
        let fits = sig.is_some_and(|sig| {
            sig.params.len() == 1
                && sig.params[0].ty == Ty::Prim(Prim::Int)
                && sig.ret == Ty::Prim(Prim::String)
                && sig.errors.is_none()
                && sig.uses.is_empty()
                && sig.generics.is_empty()
        });
        if !fits {
            self.error(p.span, format!("`{}` writes the n-th parameter of `{}` literals: it must be `fn {}(n: Int) -> String`, with no `!` and no `uses`, as it runs while the program is compiled (spec 10.15)", p.name, l.name.name, p.name));
        }
        if self.params_item(&tag.ty).is_none() {
            self.error(p.span, format!("a tag with `param` makes a record with a field `params: [P]`, which holds the values of its `{{…}}` in order: `{}` has none (spec 10.15)", self.show(&tag.ty)));
        }
    }

    /// `P`, from field `params: [P]` of the tag's record `ty`.
    pub(crate) fn params_item(&self, ty: &Ty) -> Option<Ty> {
        let Ty::Adt(def, args) = ty else {
            return None;
        };
        let Some(Shape::Record(fields)) = self.types.get(def).map(|t| &t.shape) else {
            return None;
        };
        let field = fields.iter().find(|f| f.name == "params")?;
        let Ty::List(item) = &field.ty else {
            return None;
        };
        Some(self.inf.subst(item, &self.subst_of(*def, args)))
    }

    /// The `{…}` of a tagged template of tag `name`: each typed, and
    /// converted to the tag's parameter type.
    pub(crate) fn holes(&mut self, name: &str, tag: &LiteralTag, parts: &[StrPart]) -> Vec<Hole> {
        let item = tag.param.and_then(|_| self.params_item(&tag.ty));
        let mut holes = Vec::new();
        for part in parts {
            let StrPart::Expr(x) = part else {
                continue;
            };
            let conv = match &item {
                Some(p) => self.param_conv(name, x, p),
                None => {
                    self.value(x, None);
                    if tag.param.is_none() {
                        self.error(x.span, format!("`{name}` takes no parameters, so a `{{…}}` can't go in its text: its `literal` names no `param` function. Write `\\{{` for a brace (spec 10.15)"));
                    }
                    ParamConv::Same
                }
            };
            holes.push(Hole { span: x.span, conv });
        }
        holes
    }

    /// How `x` becomes a `p`, the parameter type of tag `name`.
    fn param_conv(&mut self, name: &str, x: &Expr, p: &Ty) -> ParamConv {
        if self.is_json(p) {
            self.json_item(x, p);
            return ParamConv::Json;
        }
        let t = self.value(x, None);
        if self.coerce(&t, p) {
            return ParamConv::Same;
        }
        // A number literal is an `Int` or a `Float` (spec 1.7).
        if let Ty::Var(v) = self.inf.shallow(&t) {
            match self.inf.kind(v) {
                crate::infer::VarKind::IntLiteral => self.inf.unify(&t, &Ty::Prim(Prim::Int)),
                crate::infer::VarKind::FloatLiteral => self.inf.unify(&t, &Ty::Prim(Prim::Float)),
                crate::infer::VarKind::Any => false,
            };
        }
        let t = self.inf.resolve(&t);
        let p = self.inf.resolve(p);
        let mut fits = Vec::new();
        if let Ty::Adt(union, args) = &p
            && let Some(Shape::Union(variants)) = self.types.get(union).map(|i| &i.shape)
        {
            let subst = self.subst_of(*union, args);
            for v in variants {
                if let [f] = v.fields.as_slice()
                    && self.inf.resolve(&self.inf.subst(&f.ty, &subst)) == t
                {
                    fits.push((*union, v.name.clone()));
                }
            }
        }
        let (shown, want) = (self.show(&t), self.show(&p));
        match fits.as_slice() {
            [(union, variant)] => {
                return ParamConv::Variant {
                    union: *union,
                    name: variant.clone(),
                };
            }
            [] => self.error(x.span, format!("this `{shown}` doesn't convert to `{want}`, the type of `{name}`'s parameters: give a `{want}`, or a value that one of its variants holds as its only field (spec 10.15)")),
            _ => {
                let names: Vec<String> = fits.iter().map(|(_, v)| format!("`{want}.{v}`")).collect();
                self.error(x.span, format!("this `{shown}` fits more than one variant of `{want}` ({}): give the `{want}` itself (spec 10.15)", names.join(", ")));
            }
        }
        ParamConv::Same
    }
}
