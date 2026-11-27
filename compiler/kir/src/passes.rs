// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! The passes after lowering, in order. Files and tasks first (they add
//! code), then inlining and dropping what no root reaches, then the
//! passes on each function, which make it faster without changing what
//! it does.

use crate::ir::{Expr, ExprKind, FuncId, Function, Shape};
use crate::lower::Lowerer;
use crate::pass_set::PassSet;
use crate::rt_op::RtOp;
use kavach_driver::Compiled;

/// Runs the passes on `functions`. `roots` are what the program starts
/// from: its entry first, then a library's exports. Gives the functions
/// kept, and the roots' new numbers. Of the passes that only make the
/// program faster, only those `passes` has on run.
pub(crate) fn finish(
    l: &mut Lowerer<'_>,
    c: &Compiled,
    mut functions: Vec<Function>,
    roots: &[FuncId],
    passes: &PassSet,
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
    // A panic in the standard library points at the program's call (5.6).
    let std_from =
        (c.files.iter().find(|f| c.in_std(f.start))).map_or(u32::MAX, |f| f.first_line as u32);
    crate::caller_lines::mark_calls(&mut functions, std_from);
    // The NaN watch (spec 5.9), before inlining and the loop passes, and
    // never in a `gpu fn`, which the GPU runs as it is.
    if kavach_driver::watch_nan(std::path::Path::new(&c.path)) {
        for (f, (decl, _)) in functions.iter_mut().zip(&l.fns) {
            if !matches!(decl, Some(d) if d.is_gpu) {
                crate::nan_watch::watch_nan(f, std_from);
            }
        }
    }
    if passes.on("unrecurse") {
        for (i, f) in functions.iter_mut().enumerate() {
            crate::unrecurse::unrecurse(i as crate::ir::FuncId, f);
        }
    }
    if passes.on("inline") {
        crate::inline::inline_calls(&mut functions, &l.shapes.list);
    }
    // Functions with `inout` parameters that can't fail, and how many.
    let inout = (l.fns.iter().enumerate())
        .filter_map(|(i, (f, _))| {
            let f = (*f)?;
            let n = Lowerer::inout_params(f).len();
            (n > 0 && f.errors.is_none()).then_some((i as FuncId, n))
        })
        .collect();
    if passes.on("inout_stash") {
        crate::inout_stash::stash_inout_tuples(&mut functions, &l.shapes.list, roots, &inout);
    }
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
    let shapes = &l.shapes.list;
    let on = |name| passes.on(name);
    for f in &mut functions {
        if on("checked_fits") {
            crate::checked_fits::checked_fits(f, shapes);
        }
        if on("gpu_stay") {
            crate::gpu_stay::keep_on_gpu(f);
        }
        if on("gpu_later") {
            crate::gpu_later::overlap_sums(f, &mut notes);
        }
        if on("devirt") {
            crate::devirt::devirt(f);
        }
        if on("unused") {
            crate::unused::drop_unused(f);
        }
        if on("tuple_lets") {
            crate::tuple_lets::split_tuple_lets(f, shapes);
        }
        if on("loop_fields") {
            crate::loop_fields::keep_fields_in_locals(f, shapes);
        }
        if passes.sroa_in(&f.name) {
            crate::sroa::scalarize(f, shapes, &holders);
        }
        if on("coalesce") {
            crate::coalesce::coalesce(f);
        }
        if on("loop_lens") {
            crate::loop_lens::hoist_loop_lens(f);
        }
        if on("jam") {
            crate::jam::jam_loops(f);
        }
        if on("fuse") {
            crate::fuse::fuse_loops(f, shapes);
        }
        if on("version") {
            crate::version::version_loops(f);
        }
        if on("ranges") {
            crate::ranges::prove_ranges(f);
        }
        if on("prefetch") {
            crate::prefetch::prefetch_loops(f, shapes);
        }
        // After `coalesce`, which would merge its locals away again.
        crate::exits::keep_before_exits(f);
        if on("moves") {
            crate::moves::mark_moves(f, &holders);
        }
    }
    let quiet_from = if passes.on("par") {
        crate::par::outline(&mut functions, &mut l.shapes.list, &l.on_cores)
    } else {
        functions.len() as FuncId
    };
    if passes.on("ranges") {
        for f in functions.iter_mut().skip(quiet_from as usize) {
            crate::ranges::prove_ranges(f);
        }
    }
    if passes.on("counters") {
        for f in &mut functions {
            crate::counters::count_remainders(f);
        }
    }
    (functions, roots, quiet_from, notes)
}
