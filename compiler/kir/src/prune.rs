//! Only functions reachable from the entry are kept (decision 0024): a
//! program pays in compile time and size only for the functions it calls,
//! not for every function of every module it has.

use crate::ir::{ExprKind, FuncId, Function};
use crate::walk::{any_in_block, exprs_mut};

/// Drops the functions no call or function value reaches from `roots`,
/// renumbering the rest; gives the roots' new numbers.
pub(crate) fn keep_reachable(functions: &mut Vec<Function>, roots: &[FuncId]) -> Vec<FuncId> {
    let reached = reached(functions, roots);
    // The new number of each function kept.
    let mut new_id = vec![FuncId::MAX; functions.len()];
    let mut next = 0;
    for (old, keep) in reached.iter().enumerate() {
        if *keep {
            new_id[old] = next;
            next += 1;
        }
    }
    let mut old = 0;
    functions.retain(|_| {
        old += 1;
        reached[old - 1]
    });
    for f in functions.iter_mut() {
        exprs_mut(&mut f.body, &mut |e| {
            if let ExprKind::Call(id, _) | ExprKind::Closure { func: id, .. } = &mut e.kind {
                *id = new_id[*id as usize];
            }
        });
    }
    roots.iter().map(|r| new_id[*r as usize]).collect()
}

/// Whether each function is reached from `roots` by calls and function
/// values.
pub(crate) fn reached(functions: &[Function], roots: &[FuncId]) -> Vec<bool> {
    let mut reached = vec![false; functions.len()];
    let mut todo = roots.to_vec();
    for r in roots {
        reached[*r as usize] = true;
    }
    while let Some(f) = todo.pop() {
        any_in_block(&functions[f as usize].body, &mut |e| {
            if let ExprKind::Call(id, _) | ExprKind::Closure { func: id, .. } = &e.kind
                && !reached[*id as usize]
            {
                reached[*id as usize] = true;
                todo.push(*id);
            }
            false
        });
    }
    reached
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{Block, Expr, Stmt, Type};

    fn calling(name: &str, callees: &[FuncId]) -> Function {
        let stmts = callees
            .iter()
            .map(|c| {
                Stmt::Expr(Expr {
                    kind: ExprKind::Call(*c, Vec::new()),
                    ty: Type::Unit,
                    line: 1,
                })
            })
            .collect();
        Function {
            name: name.into(),
            params: Vec::new(),
            ret: Type::Unit,
            locals: Vec::new(),
            body: Block { stmts, value: None },
            line: 1,
            fails: false,
            external: None,
        }
    }

    #[test]
    fn unreached_functions_go_and_calls_are_renumbered() {
        // 0 is never called; 1 calls 3; 2 is the entry and calls 1 and
        // itself; 4 calls 0 but nothing calls 4.
        let mut fs = vec![
            calling("unused", &[]),
            calling("a", &[3]),
            calling("main", &[1, 2]),
            calling("b", &[]),
            calling("dead", &[0]),
        ];
        let entry = keep_reachable(&mut fs, &[2])[0];
        let names: Vec<&str> = fs.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["a", "main", "b"]);
        assert_eq!(entry, 1);
        let calls = |f: &Function| -> Vec<FuncId> {
            f.body
                .stmts
                .iter()
                .filter_map(|s| match s {
                    Stmt::Expr(Expr {
                        kind: ExprKind::Call(id, _),
                        ..
                    }) => Some(*id),
                    _ => None,
                })
                .collect()
        };
        assert_eq!(calls(&fs[0]), [2]);
        assert_eq!(calls(&fs[1]), [0, 1]);
    }
}
