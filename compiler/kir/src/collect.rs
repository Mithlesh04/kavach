//! Lowering every function of the program. A function native code can't
//! lower yet stops the build only when the program can reach it: a
//! standard library method no one calls (`Gen.pick`, generic) doesn't
//! keep a program from building (decision 0024).

use crate::Unsupported;
use crate::ir::{self, FuncId, Type};
use crate::lower::{Lowered, Lowerer};

/// Lowers every function in `l.fns` not lowered yet, in order, into
/// `functions`, including those lowering adds on the way. One that can't
/// be lowered gets an empty stand-in, and its reason goes in `failed`.
pub(crate) fn collect_functions(
    l: &mut Lowerer<'_>,
    functions: &mut Vec<ir::Function>,
    failed: &mut Vec<(FuncId, Unsupported)>,
) -> Lowered<()> {
    while functions.len() < l.fns.len() {
        let id = functions.len() as FuncId;
        if let Some(done) = l.ready.remove(&id) {
            functions.push(done);
            continue;
        }
        let lowered = match l.agents.synth.remove(&id) {
            Some(s) => l.synth_function(s),
            None => {
                let (f, owner) = l.fns[id as usize].clone();
                let f = f.expect("a declared function");
                l.subst = l.fn_subst.get(&id).cloned().unwrap_or_default();
                l.function(f, owner)
            }
        };
        match lowered {
            Ok(f) => functions.push(f),
            Err(why) => {
                failed.push((id, why));
                functions.push(stand_in());
            }
        }
    }
    l.subst.clear();
    Ok(())
}

fn stand_in() -> ir::Function {
    ir::Function {
        name: "not lowered".into(),
        params: Vec::new(),
        ret: Type::Unit,
        locals: Vec::new(),
        body: ir::Block::default(),
        line: 0,
        fails: false,
        external: None,
    }
}

/// The reason of the first function in `failed` the program reaches from
/// `roots`, if any.
pub(crate) fn none_reached(
    functions: &[ir::Function],
    roots: &[FuncId],
    failed: Vec<(FuncId, Unsupported)>,
) -> Lowered<()> {
    let reached = crate::prune::reached(functions, roots);
    match failed.into_iter().find(|(id, _)| reached[*id as usize]) {
        Some((_, why)) => Err(why),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use crate::Unsupported;
    use crate::ir::{Block, Expr, ExprKind, Function, Stmt, Type};

    /// A function whose body calls `calls`.
    fn function(calls: &[u32]) -> Function {
        let stmts = calls
            .iter()
            .map(|f| {
                Stmt::Expr(Expr {
                    kind: ExprKind::Call(*f, Vec::new()),
                    ty: Type::Unit,
                    line: 1,
                })
            })
            .collect();
        Function {
            name: "f".into(),
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
    fn only_reached_functions_must_lower() {
        // Function 1 couldn't be lowered: a program whose `main` (0)
        // never calls it still builds, one that calls it doesn't.
        let failed = || {
            vec![(
                1,
                Unsupported {
                    feature: "something".into(),
                    line: 1,
                },
            )]
        };
        let unused = [function(&[]), function(&[])];
        assert!(super::none_reached(&unused, &[0], failed()).is_ok());
        let used = [function(&[1]), function(&[])];
        assert!(super::none_reached(&used, &[0], failed()).is_err());
    }
}
