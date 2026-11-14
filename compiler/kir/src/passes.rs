//! The passes after lowering, in order. Files and tasks first (they add
//! code), then inlining and dropping what no root reaches, then the
//! passes on each function, which make it faster without changing what
//! it does.

use crate::ir::{Expr, ExprKind, FuncId, Function, Shape};
use crate::lower::Lowerer;
use crate::rt_op::RtOp;
use kavach_driver::Compiled;

/// Runs the passes on `functions`. `roots` are what the program starts
/// from: its entry first, then a library's exports. Gives the functions
/// kept, and the roots' new numbers.
pub(crate) fn finish(
    l: &mut Lowerer<'_>,
    c: &Compiled,
    mut functions: Vec<Function>,
    roots: &[FuncId],
) -> (Vec<Function>, Vec<FuncId>, FuncId, crate::gpu_later::Notes) {
    let error_ty = l.error_type();
    let mut scoped: Vec<(usize, RtOp, String)> = Vec::new();
    for (i, (f, _)) in l.fns.iter().enumerate() {
        let Some(f) = f else { continue };
        if let Some(folder) = &f.fs_scope {
            scoped.push((i, RtOp::FsScopeEnter, folder.clone()));
        }
        if let Some(program) = &f.process_scope {
            scoped.push((i, RtOp::ScopeProgram, program.clone()));
        }
    }
    let text_ty = l.str_type();
    crate::lower_tasks::scope_files(&mut functions, &scoped, error_ty, text_ty);
    crate::lower_tasks::scope_blocks(&mut functions, error_ty);
    crate::lower_tasks::join_tasks(&mut functions, error_ty);
    for (i, f) in functions.iter_mut().enumerate() {
        crate::unrecurse::unrecurse(i as crate::ir::FuncId, f);
    }
    crate::inline::inline_calls(&mut functions, &l.shapes.list);
    let roots = crate::prune::keep_reachable(&mut functions, roots);
    // Before the passes that would free a `Drop` value at its last use.
    // A `Weak` can tell when its `Shared` is freed, so in a program that
    // makes one, a `Shared` too lives to the end of its block (spec 6.2).
    let mut kept = l.drop_records.clone();
    let weak = |e: &Expr| matches!(e.kind, ExprKind::Rt(RtOp::WeakMake, _));
    if functions
        .iter()
        .any(|f| crate::walk::any_in_block(&f.body, &mut { weak }))
    {
        let shared = |i: &u32| matches!(l.shapes.list[*i as usize], Shape::Shared(_));
        kept.extend((0..l.shapes.list.len() as u32).filter(shared));
    }
    // A sender can tell when every receiver is gone (spec 9.4), so in a
    // program that makes a channel, its ends live to the end of their
    // blocks too.
    let chan = |e: &Expr| matches!(e.kind, ExprKind::Rt(RtOp::ChanNew, _));
    if functions
        .iter()
        .any(|f| crate::walk::any_in_block(&f.body, &mut { chan }))
    {
        let ends = |i: &u32| matches!(l.shapes.list[*i as usize], Shape::Channel(_));
        kept.extend((0..l.shapes.list.len() as u32).filter(ends));
    }
    let holders = crate::drop_scopes::holders(&l.shapes.list, &kept);
    if !holders.is_empty() {
        let runs = !l.drop_records.is_empty();
        for f in &mut functions {
            crate::drop_scopes::scope_drops(f, &holders, runs);
        }
    }
    crate::lower_log::configure_log(c, &mut functions, roots[0], text_ty);
    crate::stop_points::add_stop_points(&mut functions);
    crate::cancel::cancel_checks(&mut functions);
    let mut notes = crate::gpu_later::Notes::new();
    for f in &mut functions {
        crate::gpu_stay::keep_on_gpu(f);
        crate::gpu_later::overlap_sums(f, &mut notes);
        crate::devirt::devirt(f);
        crate::unused::drop_unused(f);
        crate::sroa::scalarize(f, &l.shapes.list, &holders);
        crate::coalesce::coalesce(f);
        crate::jam::jam_loops(f);
        crate::fuse::fuse_loops(f, &l.shapes.list);
        crate::version::version_loops(f);
        crate::ranges::prove_ranges(f);
        crate::prefetch::prefetch_loops(f, &l.shapes.list);
        // After `coalesce`, which would merge its locals away again.
        crate::exits::keep_before_exits(f);
        crate::moves::mark_moves(f, &holders);
    }
    let quiet_from = crate::par::outline(&mut functions, &mut l.shapes.list);
    for f in functions.iter_mut().skip(quiet_from as usize) {
        crate::ranges::prove_ranges(f);
    }
    for f in &mut functions {
        crate::counters::count_remainders(f);
    }
    (functions, roots, quiet_from, notes)
}
