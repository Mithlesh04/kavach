//! A printed line comes out whole (spec 9.8): the values of a `print`
//! that can wait (a wait, or a call that can wait, as `cancel.rs` finds)
//! are all worked out first, in order, and only then is the line written.
//! Otherwise its first pieces would be out while a later one waited, and
//! another task's line would land in the middle; the interpreter builds
//! the whole line before it prints. Other prints stay as they are, so a
//! GPU `map` printed at once still runs as one.

use crate::ir::*;
use crate::walk::{any_expr, blocks_mut};

/// For each `print` in `f` with a value for which `waits` holds of some
/// part of it.
pub fn print_whole_lines(f: &mut Function, waits: &mut impl FnMut(&Expr) -> bool) {
    let mut locals = std::mem::take(&mut f.locals);
    blocks_mut(&mut f.body, &mut |b| {
        let mut i = 0;
        while i < b.stmts.len() {
            let lets = hoist(&mut b.stmts[i], &mut locals, waits);
            let n = lets.len();
            b.stmts.splice(i..i, lets);
            i += n + 1;
        }
    });
    f.locals = locals;
}

/// For a `print` with a value that can wait: its values moved into
/// locals, set just before it, which are given back.
fn hoist(s: &mut Stmt, locals: &mut Vec<Type>, waits: &mut impl FnMut(&Expr) -> bool) -> Vec<Stmt> {
    let Stmt::Print(parts) = s else {
        return Vec::new();
    };
    let mut wait = |e: &Expr| any_expr(e, &mut *waits);
    if !parts
        .iter()
        .any(|p| matches!(p, PrintPart::Value(e) if wait(e)))
    {
        return Vec::new();
    }
    let mut lets = Vec::new();
    for p in parts.iter_mut() {
        let PrintPart::Value(e) = p else {
            continue;
        };
        let (ty, line) = (e.ty, e.line);
        let l = locals.len() as Local;
        locals.push(ty);
        let value = std::mem::replace(
            e,
            Expr {
                kind: ExprKind::Local(l),
                ty,
                line,
            },
        );
        lets.push(Stmt::Let(l, value));
    }
    lets
}

#[cfg(test)]
mod tests {
    use super::print_whole_lines;
    use crate::ir::*;

    fn e(kind: ExprKind) -> Expr {
        Expr {
            kind,
            ty: Type::I64,
            line: 1,
        }
    }

    fn printing(value: Expr) -> Function {
        Function {
            name: "f".into(),
            params: Vec::new(),
            ret: Type::Unit,
            locals: Vec::new(),
            body: Block {
                stmts: vec![Stmt::Print(vec![
                    PrintPart::Text("result ".into()),
                    PrintPart::Value(value),
                ])],
                value: None,
            },
            line: 1,
            fails: false,
            external: None,
        }
    }

    /// `print("result {f()}")`, `f` able to wait: the call is made first,
    /// then the line.
    #[test]
    fn a_print_that_calls_works_out_its_values_first() {
        let mut f = printing(e(ExprKind::Call(0, Vec::new())));
        print_whole_lines(&mut f, &mut |x| matches!(x.kind, ExprKind::Call(..)));
        assert!(
            matches!(f.body.stmts[0], Stmt::Let(0, _)),
            "{:?}",
            f.body.stmts
        );
        assert!(
            matches!(&f.body.stmts[1], Stmt::Print(p) if matches!(&p[1], PrintPart::Value(Expr { kind: ExprKind::Local(0), .. })))
        );
    }

    /// `print("result {n}")`, which can't wait, is left as it is.
    #[test]
    fn a_print_of_plain_values_is_left_alone() {
        let mut f = printing(e(ExprKind::Int(3)));
        print_whole_lines(&mut f, &mut |x| matches!(x.kind, ExprKind::Call(..)));
        assert_eq!(f.body.stmts.len(), 1);
    }
}
