// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Lowering a tagged template whose tag takes parameters (spec 10.15,
//! decision 0142): `` sql`… {owner}` `` is its tag's record, whose text
//! has the tag's `param(n)` in place of the n-th `{…}`, and whose
//! `params` hold the values, each converted as the checker said.

use crate::ir::{self, ExprKind as K, Shape, Type};
use crate::lower::{Lowered, Lowerer};
use kavach_ast as ast;
use kavach_checker::{LiteralUse, ParamConv};
use kavach_resolver::DefId;

impl<'a> Lowerer<'a> {
    /// The tagged literal of type `def` and `ty` whose text is `args`,
    /// when its tag takes parameters (`param`); `None` for any other.
    pub(crate) fn tagged_template(
        &mut self,
        def: DefId,
        args: &'a [ast::Arg],
        ty: Type,
        line: u32,
        span: ast::Span,
    ) -> Option<Lowered<ir::Expr>> {
        let arg = args.first()?;
        let c = self.c;
        let u = c.literal_uses.iter().find(|u| u.span == arg.value.span)?;
        let f = *self.fn_ids.get(&u.param?)?;
        let ast::ExprKind::Str(parts) = &arg.value.kind else {
            return None;
        };
        Some(self.template_record(def, u, f, parts, ty, line, span))
    }

    #[allow(clippy::too_many_arguments)]
    fn template_record(
        &mut self,
        def: DefId,
        u: &'a LiteralUse,
        f: ir::FuncId,
        parts: &'a [ast::StrPart],
        ty: Type,
        line: u32,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let at = |kind, ty| ir::Expr { kind, ty, line };
        let str_ty = self.str_type();
        let list_ty = match self.shapes.shape(ty) {
            Some(Shape::Record { fields, .. }) => {
                fields.iter().find(|f| f.name == "params").map(|f| f.ty)
            }
            _ => None,
        };
        let Some(list_ty) = list_ty else {
            return Err(self.unsupported("this tagged template", span));
        };
        let item = match self.shapes.shape(list_ty) {
            Some(Shape::List(item)) => *item,
            _ => return Err(self.unsupported("this tagged template", span)),
        };
        let mut text = Vec::new();
        let mut params = Vec::new();
        for part in parts {
            match part {
                ast::StrPart::Text(t) => text.push(at(K::Str(t.clone()), str_ty)),
                ast::StrPart::Expr(_) => {
                    let n = at(K::Int(1 + params.len() as i128), Type::I64);
                    text.push(at(K::Call(f, vec![n]), str_ty));
                    params.push(part);
                }
            }
        }
        let mut values = Vec::new();
        for (i, part) in params.into_iter().enumerate() {
            let ast::StrPart::Expr(x) = part else {
                continue;
            };
            let value = match u.holes.get(i).map(|h| &h.conv) {
                Some(ParamConv::Json) => self.json_item(x, item)?,
                Some(ParamConv::Variant { name, .. }) => self.param_variant(x, name, item, line)?,
                _ => {
                    let value = self.expr(x)?;
                    self.coerce(value, item)?
                }
            };
            values.push(value);
        }
        let mut text = Some(at(K::Format(text), str_ty));
        let mut list = Some(at(K::List(values), list_ty));
        self.construct_given(def, None, ty, line, span, &mut |_, i, name| {
            let made = match (i, name) {
                (0, _) => text.take(),
                (_, "params") => list.take(),
                _ => None,
            };
            Ok(made.map(|x| (x, None)))
        })
    }

    /// `x` as variant `name` of union `p`, whose one field takes it.
    fn param_variant(
        &mut self,
        x: &'a ast::Expr,
        name: &str,
        p: Type,
        line: u32,
    ) -> Lowered<ir::Expr> {
        let value = self.expr(x)?;
        let (variant, field) = match self.shapes.shape(p) {
            Some(Shape::Union { variants, .. }) => variants
                .iter()
                .position(|v| v.name == name)
                .and_then(|i| Some((i as u32, variants[i].fields.first()?.ty))),
            _ => None,
        }
        .ok_or_else(|| self.unsupported("this parameter", x.span))?;
        let value = self.coerce(value, field)?;
        Ok(ir::Expr {
            kind: K::Record {
                variant,
                fields: vec![value],
            },
            ty: p,
            line,
        })
    }
}
