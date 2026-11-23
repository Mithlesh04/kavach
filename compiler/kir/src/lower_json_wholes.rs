// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `json.decode` into a type that holds whole numbers with their own
//! limits (spec 2.1, 2.12): `[UInt<bits: 12>]`, a record with an
//! `Int<range: 1..=65535>` field. Such values are kept in an `Int`, so
//! the type the runtime reads doesn't say their limits. The call passes
//! a *plan* of where they are, which the decoder walks beside the type
//! (`runtime/native/src/rt_json_wholes.rs` describes its numbers).

use crate::ir::Shape;
use crate::lower::{Lowered, Lowerer};
use kavach_ast as ast;
use kavach_checker::Ty;
use kavach_resolver::{DefId, DefKind};
use std::collections::{HashMap, HashSet};

/// The plan's nodes so far, and the node made for each record or union
/// type, so a type that holds itself refers back to its node.
struct Plan {
    nodes: Vec<Vec<i128>>,
    made: Vec<(Ty, usize)>,
}

impl<'a> Lowerer<'a> {
    /// The plan for the decode at `span`, as numbers: empty when its type
    /// holds no whole number with its own limits.
    pub(crate) fn whole_plan(&mut self, span: ast::Span) -> Lowered<Vec<i128>> {
        let Some(t) = self.c.expr_types.get(&(span.start, span.end)).cloned() else {
            return Ok(Vec::new());
        };
        let t = match t {
            Ty::Result(ok, _) => *ok,
            t => t,
        };
        let mut plan = Plan {
            nodes: vec![vec![0]],
            made: Vec::new(),
        };
        if self.plan_node(&t, &mut plan, span)? == 0 {
            return Ok(Vec::new());
        }
        Ok(plan.nodes.concat())
    }

    /// The node for `t`, added to `plan`: 0 when it holds nothing of the
    /// kind.
    fn plan_node(&mut self, t: &Ty, plan: &mut Plan, span: ast::Span) -> Lowered<usize> {
        if let Some((_, at)) = plan.made.iter().find(|(m, _)| m == t) {
            return Ok(*at);
        }
        let t = match t {
            Ty::Sized(x, ..) | Ty::Frozen(x, ..) => return self.plan_node(x, plan, span),
            Ty::Param(d) => match self.wholes.get(d) {
                Some(p) => &Ty::Prim(*p),
                None => return Ok(0),
            },
            t => t,
        };
        if !self.holds_whole(t, true, &mut HashSet::new()) {
            return Ok(0);
        }
        let at = plan.nodes.len();
        plan.nodes.push(Vec::new());
        let node = match t {
            Ty::Prim(p) => {
                let (lo, hi) = p.int_range().unwrap_or((0, 0));
                let (bits, signed) = match p {
                    kavach_checker::Prim::Bits(n, signed) => (i128::from(*n), *signed),
                    _ => (0, true),
                };
                vec![1, lo, hi, bits, i128::from(signed)]
            }
            Ty::List(x) | Ty::Set(x) | Ty::Optional(x) => {
                vec![2, self.plan_node(x, plan, span)? as i128]
            }
            Ty::Map(k, v) => {
                let k = self.plan_node(k, plan, span)? as i128;
                vec![3, k, self.plan_node(v, plan, span)? as i128]
            }
            Ty::Tuple(ts) => {
                let mut node = vec![4, ts.len() as i128];
                for t in ts {
                    node.push(self.plan_node(t, plan, span)? as i128);
                }
                node
            }
            Ty::Anon(_) | Ty::Adt(..) => {
                if matches!(t, Ty::Adt(..)) {
                    plan.made.push((t.clone(), at));
                }
                self.plan_fields(t, plan, span)?
            }
            _ => vec![0],
        };
        plan.nodes[at] = node;
        Ok(at)
    }

    /// The node of record, union or anonymous record `t`: each variant's
    /// fields, or the record's, in the order of its shape.
    fn plan_fields(&mut self, t: &Ty, plan: &mut Plan, span: ast::Span) -> Lowered<Vec<i128>> {
        let ty = self.kir_type(t, span)?;
        let shape = self.shapes.shape(ty).cloned();
        let (union, groups): (bool, Vec<Vec<Option<Ty>>>) = match (t, shape) {
            (Ty::Anon(fs), Some(Shape::Record { fields, .. })) => {
                let of = |name: &str| fs.iter().find(|(n, _)| n == name).map(|(_, t)| t.clone());
                (false, vec![fields.iter().map(|f| of(&f.name)).collect()])
            }
            (Ty::Adt(def, args), Some(Shape::Record { fields, .. })) => {
                let params = self.params_of(*def, args);
                let types = fields.iter().map(|f| self.field_ty(*def, &f.name, &params));
                (false, vec![types.collect()])
            }
            (Ty::Adt(def, args), Some(Shape::Union { variants, .. })) => {
                let params = self.params_of(*def, args);
                let defs = &self.c.resolution.defs;
                let groups = (variants.iter())
                    .map(|v| {
                        let owner = (0..defs.len()).find(|d| {
                            defs[*d].name == v.name
                                && matches!(defs[*d].kind, DefKind::Variant { of } if of == *def)
                        });
                        (v.fields.iter())
                            .map(|f| owner.and_then(|o| self.field_ty(o, &f.name, &params)))
                            .collect()
                    })
                    .collect();
                (true, groups)
            }
            _ => return Ok(vec![0]),
        };
        let mut node = if union {
            vec![6, groups.len() as i128]
        } else {
            vec![5]
        };
        for types in groups {
            node.push(types.len() as i128);
            for t in types {
                let part = match t {
                    Some(t) => self.plan_node(&t, plan, span)?,
                    None => 0,
                };
                node.push(part as i128);
            }
        }
        Ok(node)
    }

    /// What `def`'s type parameters stand for, given its type arguments.
    fn params_of(&self, def: DefId, args: &[Ty]) -> HashMap<DefId, Ty> {
        (self.type_generics(def).into_iter())
            .zip(args.iter().cloned())
            .collect()
    }

    /// The type of field `name` of record or variant `owner`.
    fn field_ty(&self, owner: DefId, name: &str, params: &HashMap<DefId, Ty>) -> Option<Ty> {
        let t = self.c.field_types.get(&(owner, name.to_string()))?;
        Some(substitute(t, params))
    }

    /// Whether `t` holds a whole number with its own limits, at any depth;
    /// with `fields`, in a record's or union's fields too.
    pub(crate) fn holds_whole(&self, t: &Ty, fields: bool, seen: &mut HashSet<DefId>) -> bool {
        match t {
            Ty::Prim(p) => p.is_narrow(),
            Ty::Param(d) => self.wholes.contains_key(d),
            Ty::List(x) | Ty::Set(x) | Ty::Optional(x) => self.holds_whole(x, fields, seen),
            Ty::Map(k, v) | Ty::Result(k, v) => {
                self.holds_whole(k, fields, seen) || self.holds_whole(v, fields, seen)
            }
            Ty::Tuple(ts) | Ty::Builtin(_, ts) => {
                ts.iter().any(|t| self.holds_whole(t, fields, seen))
            }
            Ty::Anon(fs) => fs.iter().any(|(_, t)| self.holds_whole(t, fields, seen)),
            Ty::Sized(x, _, _) | Ty::Frozen(x, _, _) => self.holds_whole(x, fields, seen),
            Ty::Adt(def, args) => {
                if !seen.insert(*def) {
                    return false;
                }
                let defs = &self.c.resolution.defs;
                let of_def = |d: DefId| {
                    d == *def || matches!(defs[d].kind, DefKind::Variant { of } if of == *def)
                };
                args.iter().any(|a| self.holds_whole(a, fields, seen))
                    || (fields
                        && (self.c.field_types.iter())
                            .any(|((d, _), t)| of_def(*d) && self.holds_whole(t, true, seen)))
            }
            _ => false,
        }
    }
}

/// `t` with each type parameter in `params` replaced by its type.
fn substitute(t: &Ty, params: &HashMap<DefId, Ty>) -> Ty {
    let all = |ts: &[Ty]| ts.iter().map(|t| substitute(t, params)).collect();
    let one = |t: &Ty| Box::new(substitute(t, params));
    match t {
        Ty::Param(d) => params.get(d).cloned().unwrap_or(Ty::Param(*d)),
        Ty::List(t) => Ty::List(one(t)),
        Ty::Set(t) => Ty::Set(one(t)),
        Ty::Optional(t) => Ty::Optional(one(t)),
        Ty::Map(k, v) => Ty::Map(one(k), one(v)),
        Ty::Tuple(ts) => Ty::Tuple(all(ts)),
        Ty::Adt(d, ts) => Ty::Adt(*d, all(ts)),
        Ty::Builtin(n, ts) => Ty::Builtin(n, all(ts)),
        Ty::Sized(x, k, n) => Ty::Sized(one(x), *k, n.clone()),
        Ty::Anon(fs) => Ty::Anon(
            fs.iter()
                .map(|(n, t)| (n.clone(), substitute(t, params)))
                .collect(),
        ),
        t => t.clone(),
    }
}
