//! `Drop` (spec 6.1): at its start, a program registers each type's
//! `drop` with the runtime (`DropRegister`), which runs it when a value's
//! last holder ends (`runtime/native/src/rt_drops.rs`). `drop_scopes.rs`
//! decides where holders end.

use crate::ir::{self, ExprKind as K, Shape, Stmt, Type};
use crate::lower::{Lowered, Lowerer, Owner};
use crate::rt_op::RtOp;
use kavach_ast::Span;
use kavach_checker::Ty;

impl<'a> Lowerer<'a> {
    /// What the start of the entry function sets up, before the
    /// program's own code: a worker per core when the program has a
    /// `spawn cores` (spec 9.8), and each `Drop` type's `drop`.
    pub(crate) fn program_start(&mut self) -> Lowered<Vec<Stmt>> {
        let mut out = Vec::new();
        if spawns_on_cores(&self.c.file) {
            out.push(Stmt::Expr(ir::Expr {
                kind: K::Rt(RtOp::TaskParallel, Vec::new()),
                ty: Type::Unit,
                line: 1,
            }));
        }
        out.extend(self.drop_registrations()?);
        Ok(out)
    }

    /// The statements that register each `Drop` type's `drop`, for the
    /// start of the entry function; notes the types (`drop_records`).
    pub(crate) fn drop_registrations(&mut self) -> Lowered<Vec<Stmt>> {
        let mut defs: Vec<_> = self.c.drop_types.iter().copied().collect();
        defs.sort_unstable();
        let mut out = Vec::new();
        for def in defs {
            // A generic type's methods are made per use; not yet here.
            let key = (Owner::Adt(def), "drop".to_string());
            let Some(&fid) = self.method_ids.get(&key) else {
                continue;
            };
            let span = Span::new(0, 0);
            let ty = self.kir_type(&Ty::Adt(def, Vec::new()), span)?;
            let Type::Ref(id) = ty else { continue };
            let fn_ty = self.shapes_intern(Shape::Func(vec![ty], Type::Unit));
            let closure = self.function_value(fid, fn_ty, span)?;
            self.drop_records.push(id);
            let code = ir::Expr {
                kind: K::Int(i128::from(id)),
                ty: Type::I64,
                line: 1,
            };
            out.push(Stmt::Expr(ir::Expr {
                kind: K::Rt(RtOp::DropRegister, vec![code, closure]),
                ty: Type::Unit,
                line: 1,
            }));
        }
        Ok(out)
    }
}

/// Whether `file` has a `spawn cores` anywhere.
fn spawns_on_cores(file: &kavach_ast::File) -> bool {
    struct Find(bool);
    impl kavach_ast::visit::VisitMut for Find {
        fn expr(&mut self, e: &mut kavach_ast::Expr) {
            let cores = kavach_ast::SpawnKind::Cores;
            self.0 |= matches!(e.kind, kavach_ast::ExprKind::Spawn(k, ..) if k == cores);
            kavach_ast::visit::walk_expr(self, e);
        }
    }
    let mut find = Find(false);
    kavach_ast::visit::walk_file(&mut find, &mut file.clone());
    find.0
}
