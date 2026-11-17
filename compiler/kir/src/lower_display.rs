//! `Display` (spec 1.11): at its start, a program registers each type's
//! `display` with the runtime (`DisplayRegister`), which shows the type's
//! values as it gives (`runtime/native/src/rt_display.rs`).

use crate::ir::{self, ExprKind as K, Shape, Stmt, Type};
use crate::lower::{Lowered, Lowerer, Owner};
use crate::rt_op::RtOp;
use kavach_ast::Span;
use kavach_checker::Ty;

impl Lowerer<'_> {
    /// The statements that register each `Display` type's `display`, for
    /// the start of the entry function; a generic type's are made per
    /// instance (`generic_display_registrations`).
    pub(crate) fn display_registrations(&mut self) -> Lowered<Vec<Stmt>> {
        let mut defs: Vec<_> = self.c.display_types.iter().copied().collect();
        defs.sort_unstable();
        let mut out = Vec::new();
        for def in defs {
            let key = (Owner::Adt(def), "display".to_string());
            let Some(&fid) = self.method_ids.get(&key) else {
                continue;
            };
            let span = Span::new(0, 0);
            let ty = self.kir_type(&Ty::Adt(def, Vec::new()), span)?;
            let Type::Ref(id) = ty else { continue };
            self.display_records.push(id);
            let text = self.str_type();
            let fn_ty = self.shapes_intern(Shape::Func(vec![ty], text, None));
            let closure = self.function_value(fid, fn_ty, span)?;
            let code = ir::Expr {
                kind: K::Int(i128::from(id)),
                ty: Type::I64,
                line: 1,
            };
            out.push(Stmt::Expr(ir::Expr {
                kind: K::Rt(RtOp::DisplayRegister, vec![code, closure]),
                ty: Type::Unit,
                line: 1,
            }));
        }
        Ok(out)
    }

    /// The registrations of each instance of a generic `Display` type the
    /// program made (`Box<Int>`) and that isn't registered yet: its
    /// `display`, made for those type arguments. The caller asks again
    /// until none is left, as making one may make more instances.
    pub(crate) fn generic_display_registrations(&mut self) -> Lowered<Vec<Stmt>> {
        let mut found: Vec<(kavach_resolver::DefId, Vec<Type>, u32)> = (self.shapes.adts.iter())
            .filter(|((def, args), _)| !args.is_empty() && self.c.display_types.contains(def))
            .map(|((def, args), id)| (*def, args.clone(), *id))
            .filter(|(_, _, id)| !self.display_records.contains(id))
            .collect();
        found.sort_by_key(|f| f.2);
        let mut out = Vec::new();
        for (def, args, id) in found {
            self.display_records.push(id);
            let key = (def, "display".to_string());
            let Some(f) = self.generic_methods.get(&key).copied() else {
                continue;
            };
            let subst = (self.type_generics(def).into_iter())
                .zip(args.iter().copied())
                .collect();
            let fid = self.instance(def, f, args, subst, Some(Owner::Adt(def)));
            let text = self.str_type();
            let fn_ty = self.shapes_intern(Shape::Func(vec![Type::Ref(id)], text, None));
            let closure = self.function_value(fid, fn_ty, Span::new(0, 0))?;
            let code = ir::Expr {
                kind: K::Int(i128::from(id)),
                ty: Type::I64,
                line: 1,
            };
            out.push(Stmt::Expr(ir::Expr {
                kind: K::Rt(RtOp::DisplayRegister, vec![code, closure]),
                ty: Type::Unit,
                line: 1,
            }));
        }
        Ok(out)
    }
}
