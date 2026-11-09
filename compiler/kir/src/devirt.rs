//! Devirtualizing calls through stored function values (spec 3.4): when
//! a local is stored exactly once, from a record literal whose field `i`
//! is a closure of a known function, `h.on(x)` — a `CallValue` of
//! `GetField(h, i)` — becomes a direct call of that function. A direct
//! call is one LLVM inlines, so the callback costs what C++'s does.

use crate::ir::*;
use crate::walk::{any_in_block, exprs_mut};
use std::collections::HashMap;

pub fn devirt(f: &mut Function) {
    // One store per local, from a record literal: which function each
    // closure field holds.
    let mut stores: HashMap<Local, Vec<Option<FuncId>>> = HashMap::new();
    let mut stored_twice: Vec<Local> = Vec::new();
    for s in &f.body.stmts {
        let Stmt::Let(l, v) = s else { continue };
        if stores.contains_key(l) {
            stored_twice.push(*l);
            continue;
        }
        if let ExprKind::Record { variant: 0, fields } = &v.kind {
            stores.insert(
                *l,
                fields
                    .iter()
                    .map(|x| match &x.kind {
                        ExprKind::Closure { func, .. } => Some(*func),
                        _ => None,
                    })
                    .collect(),
            );
        }
    }
    // A local assigned anywhere, at any depth, is out.
    any_in_block(&f.body, &mut |e| {
        if let ExprKind::Assign(l, _) = &e.kind {
            stored_twice.push(*l);
        }
        false
    });
    for l in stored_twice {
        stores.remove(&l);
    }
    if stores.values().all(|fs| fs.iter().all(Option::is_none)) {
        return;
    }
    exprs_mut(&mut f.body, &mut |e| {
        let ExprKind::CallValue(callee, _) = &e.kind else {
            return;
        };
        let ExprKind::GetField(base, i) = &callee.kind else {
            return;
        };
        let ExprKind::Local(h) = base.kind else {
            return;
        };
        let Some(Some(func)) = stores.get(&h).and_then(|fs| fs.get(*i as usize)) else {
            return;
        };
        let func = *func;
        let ExprKind::CallValue(callee, args) = std::mem::replace(&mut e.kind, ExprKind::None)
        else {
            unreachable!()
        };
        // The function still takes its closure first (the convention of
        // `lower_closure.rs`); the call is just no longer indirect.
        let mut values = vec![*callee];
        values.extend(args);
        e.kind = ExprKind::Call(func, values);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_call_through_a_once_stored_field_becomes_direct() {
        let rec = Type::Ref(0);
        let fnty = Type::Ref(1);
        let e = |kind, ty| Expr { kind, ty, line: 1 };
        let closure = e(
            ExprKind::Closure {
                func: 7,
                env: Type::Ref(2),
                captures: vec![],
            },
            fnty,
        );
        let get = e(
            ExprKind::GetField(Box::new(e(ExprKind::Local(0), rec)), 0),
            fnty,
        );
        let call = e(
            ExprKind::CallValue(Box::new(get), vec![e(ExprKind::Int(3), Type::I64)]),
            Type::I64,
        );
        let mut f = Function {
            name: "main".into(),
            params: vec![],
            ret: Type::Unit,
            locals: vec![rec],
            body: Block {
                stmts: vec![
                    Stmt::Let(
                        0,
                        e(
                            ExprKind::Record {
                                variant: 0,
                                fields: vec![closure],
                            },
                            rec,
                        ),
                    ),
                    Stmt::Expr(call),
                ],
                value: None,
            },
            line: 1,
            fails: false,
            external: None,
        };
        devirt(&mut f);
        assert!(any_in_block(&f.body, &mut |x| matches!(
            x.kind,
            ExprKind::Call(7, _)
        )));
        assert!(!any_in_block(&f.body, &mut |x| matches!(
            x.kind,
            ExprKind::CallValue(..)
        )));
    }
}
