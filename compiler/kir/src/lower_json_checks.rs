//! The invariants of decoded values in native code (spec 2.12 and 7.3,
//! decision 0086). Before a `json.decode`, each record type with
//! invariants that the decoded type can hold is registered with the
//! runtime, with its check: a function giving the bits of the invariants
//! a value breaks, and their texts (`runtime/native/src/rt_json_checks.rs`).
//! A broken one is a problem the decode reports, not a panic.

use crate::ir::{self, BinOp, Block, ExprKind as K, Shape, Stmt, Type, UnOp};
use crate::lower::{Lowered, Lowerer};
use crate::rt_op::RtOp;
use kavach_ast::{self as ast, ItemKind, TypeBody};

impl<'a> Lowerer<'a> {
    /// The statements that register the checks for the record types `ty`
    /// can hold; none when no such type has invariants.
    pub(crate) fn invariant_checks(&mut self, ty: Type, line: u32) -> Lowered<Vec<Stmt>> {
        let (mut seen, mut todo, mut out) = (Vec::new(), vec![ty], Vec::new());
        while let Some(t) = todo.pop() {
            if seen.contains(&t) {
                continue;
            }
            seen.push(t);
            let Some(shape) = self.shapes.shape(t).cloned() else {
                continue;
            };
            match &shape {
                Shape::List(x) | Shape::Set(x) | Shape::Optional(x) => todo.push(*x),
                Shape::Map(k, v) => todo.extend([*k, *v]),
                Shape::Tuple(ts) => todo.extend(ts),
                Shape::Union { variants, .. } => {
                    todo.extend(variants.iter().flat_map(|v| v.fields.iter().map(|f| f.ty)))
                }
                Shape::Record { fields, .. } => {
                    todo.extend(fields.iter().map(|f| f.ty));
                    let invariants = self.invariants_of(t);
                    if let (false, Type::Ref(id)) = (invariants.is_empty(), t) {
                        out.push(self.register_check(t, id, invariants, line)?);
                    }
                }
                _ => {}
            }
        }
        Ok(out)
    }

    /// The `invariant`s of record type `t` (spec 7.3).
    pub(crate) fn invariants_of(&self, t: Type) -> &'a [ast::Expr] {
        let Some(def) = self.shapes.def_of(t) else {
            return &[];
        };
        let decl_span = self.c.resolution.defs[def].span;
        let body = self.c.file.items.iter().find_map(|item| match &item.kind {
            ItemKind::Type(d) if Some(d.name.span) == decl_span => Some(&d.body),
            _ => None,
        });
        match body {
            Some(TypeBody::Record(r)) => &r.invariants,
            _ => &[],
        }
    }

    /// `JsonCheckRegister` for record type `t`, number `id`.
    fn register_check(
        &mut self,
        t: Type,
        id: u32,
        invariants: &'a [ast::Expr],
        line: u32,
    ) -> Lowered<Stmt> {
        let at = |kind, ty| ir::Expr { kind, ty, line };
        let func = self.check_function(t, invariants, line)?;
        let env = self.shapes_intern(Shape::Closure(Vec::new()));
        let fn_ty = self.shapes_intern(Shape::Func(vec![t], Type::I64, None));
        let check = at(
            K::Closure {
                func,
                env,
                captures: Vec::new(),
            },
            fn_ty,
        );
        let text = self.str_type();
        let texts = invariants
            .iter()
            .map(|inv| at(K::Str(self.text(inv.span).to_string()), text))
            .collect();
        let list = self.list_type(text);
        let args = vec![
            at(K::Int(i128::from(id)), Type::I64),
            check,
            at(K::List(texts), list),
        ];
        Ok(Stmt::Expr(at(
            K::Rt(RtOp::JsonCheckRegister, args),
            Type::Unit,
        )))
    }

    /// `(closure, value) -> Int`: bit `i` set when `value` breaks
    /// invariant `i`.
    fn check_function(
        &mut self,
        t: Type,
        invariants: &'a [ast::Expr],
        line: u32,
    ) -> Lowered<ir::FuncId> {
        let at = |kind, ty| ir::Expr { kind, ty, line };
        let saved = self.save_function(Type::I64);
        let env_ty = self.shapes_intern(Shape::Closure(Vec::new()));
        let (env, value, bits) = (
            self.new_local(env_ty),
            self.new_local(t),
            self.new_local(Type::I64),
        );
        let mut stmts = vec![Stmt::Let(bits, at(K::Int(0), Type::I64))];
        let outer = self.invariant_of.replace((value, t));
        let mut failed = None;
        for (i, inv) in invariants.iter().enumerate().take(64) {
            let cond = match self.expr(inv) {
                Ok(c) => c,
                Err(e) => {
                    failed = Some(e);
                    break;
                }
            };
            let set = at(
                K::Binary(
                    BinOp::BitOr,
                    Box::new(self.local_expr(bits, line)),
                    Box::new(at(K::Int(1i128 << i), Type::I64)),
                ),
                Type::I64,
            );
            let broken = at(K::Unary(UnOp::Not, Box::new(cond)), Type::Bool);
            let assign = at(K::Assign(bits, Box::new(set)), Type::Unit);
            let mark = Block {
                stmts: vec![Stmt::Expr(assign)],
                value: None,
            };
            stmts.push(Stmt::Expr(at(
                K::If(Box::new(broken), mark, Block::default()),
                Type::Unit,
            )));
        }
        self.invariant_of = outer;
        let f = ir::Function {
            name: "invariant check".into(),
            params: vec![env, value],
            ret: Type::I64,
            locals: self.locals.clone(),
            body: Block {
                stmts,
                value: Some(Box::new(self.local_expr(bits, line))),
            },
            line,
            fails: false,
            external: None,
        };
        self.restore_function(saved);
        if let Some(e) = failed {
            return Err(e);
        }
        Ok(self.add_function(f))
    }
}
