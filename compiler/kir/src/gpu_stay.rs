//! GPU results that stay on the GPU until the CPU reads them (spec 15.2,
//! decision 0055). A local that a GPU `map` sets (`xs = xs.map(step)`)
//! may hold a list whose items are only in GPU memory. Its `map`s become
//! `GpuMapStay`, and every other read of the local is a `GpuFetch` first,
//! which brings the items back once. A `map` of it reads it on the GPU,
//! with no fetch, so a loop of `map`s copies back only at the end. Before
//! a loop that reads such a local but doesn't set it, one fetch goes
//! ahead of the loop instead of one per round. A `map` whose result only
//! a GPU op reads (`xs.map(f).sum(g)`) stays on the GPU the same way.

use crate::ir::{Block, Expr, ExprKind as K, Function, Local, Stmt, Type};
use crate::rt_op::RtOp;
use crate::walk::{self, Part};
use std::collections::HashSet;

pub(crate) fn keep_on_gpu(f: &mut Function) {
    // Such a list is held only for the op, which brings it back itself
    // if the CPU must read it.
    let mut held_by_op = HashSet::new();
    walk::blocks_mut(&mut f.body, &mut |b| {
        if read_on_gpu(b)
            && let Some(Stmt::Let(held, list)) = b.stmts.first_mut()
            && gpu_map(list).is_some()
        {
            stay(list);
            held_by_op.insert(*held);
        }
    });
    let mut set_by_gpu = set_by_gpu(f);
    set_by_gpu.retain(|l| !held_by_op.contains(l));
    if set_by_gpu.is_empty() {
        return;
    }
    let types = f.locals.clone();
    block(&mut f.body, &set_by_gpu, &HashSet::new(), &types);
    // Each such `map` leaves its result on the GPU, and reads its own
    // list without fetching it.
    walk::exprs_mut(&mut f.body, &mut |e| {
        if let K::Assign(l, value) = &mut e.kind
            && set_by_gpu.contains(l)
        {
            stay(value);
        }
    });
    walk::blocks_mut(&mut f.body, &mut |b| {
        for s in &mut b.stmts {
            if let Stmt::Let(l, value) = s
                && set_by_gpu.contains(l)
            {
                stay(value);
            }
        }
        unfetch_receiver(b);
    });
}

/// The locals a GPU `map` or `filter` sets.
pub(crate) fn set_by_gpu(f: &mut Function) -> HashSet<Local> {
    let mut set_by_gpu = HashSet::new();
    walk::blocks_mut(&mut f.body, &mut |b| {
        for s in &mut b.stmts {
            if let Stmt::Let(l, e) = s
                && gpu_map(e).is_some()
            {
                set_by_gpu.insert(*l);
            }
        }
    });
    walk::exprs_mut(&mut f.body, &mut |e| {
        if let K::Assign(l, value) = &mut e.kind
            && gpu_map(value).is_some()
        {
            set_by_gpu.insert(*l);
        }
    });
    set_by_gpu
}

/// Whether `e` is a `map` or `filter` that may run on the GPU.
pub(crate) fn is_gpu_list(e: &Expr) -> bool {
    let K::Block(b) = &e.kind else { return false };
    matches!(
        b.stmts.get(1),
        Some(Stmt::Let(
            _,
            Expr {
                kind: K::Rt(
                    RtOp::GpuMap | RtOp::GpuMapStay | RtOp::GpuFilter | RtOp::GpuFilterStay,
                    _
                ),
                ..
            }
        ))
    )
}

/// The `GpuMap` of `e`, when `e` is a `map` that may run on the GPU, as
/// `lower_gpu.rs` writes it: the list held, then the GPU's try.
fn gpu_map(e: &mut Expr) -> Option<&mut RtOp> {
    let K::Block(b) = &mut e.kind else {
        return None;
    };
    match b.stmts.get_mut(1) {
        Some(Stmt::Let(
            _,
            Expr {
                kind:
                    K::Rt(
                        op @ (RtOp::GpuMap
                        | RtOp::GpuMapStay
                        | RtOp::GpuFilter
                        | RtOp::GpuFilterStay),
                        _,
                    ),
                ..
            },
        )) => Some(op),
        _ => None,
    }
}

fn stay(e: &mut Expr) {
    if let Some(op) = gpu_map(e) {
        *op = match op {
            RtOp::GpuFilter | RtOp::GpuFilterStay => RtOp::GpuFilterStay,
            _ => RtOp::GpuMapStay,
        };
    }
}

/// Whether block `b` is a GPU op with its list held first, as
/// `lower_gpu.rs` writes it: the list may be read there.
fn read_on_gpu(b: &Block) -> bool {
    matches!(
        b.stmts.get(1),
        Some(Stmt::Let(
            _,
            Expr {
                kind: K::Rt(
                    RtOp::GpuMap
                        | RtOp::GpuMapStay
                        | RtOp::GpuFilter
                        | RtOp::GpuFilterStay
                        | RtOp::GpuSum
                        | RtOp::GpuMin
                        | RtOp::GpuMax,
                    _
                ),
                ..
            }
        ))
    )
}

/// A `map`'s own list, read as it is: the GPU reads it there. So does a
/// `sum`, `min` or `max` that may run on the GPU.
fn unfetch_receiver(b: &mut Block) {
    let is_map = read_on_gpu(b);
    if let (true, Some(Stmt::Let(_, held))) = (is_map, b.stmts.first_mut()) {
        unfetch(held);
    }
    // A `map` done in place when the CPU does it (`lower_lambda.rs`): its
    // branch takes the list out of the local first. The runtime brought
    // the list back already when the GPU left it to the CPU.
    if let (true, Some(choose)) = (is_map, b.value.as_deref_mut())
        && let K::If(_, _, cpu) = &mut choose.kind
        && let Some(Stmt::Let(_, taken)) = cpu.stmts.first_mut()
    {
        unfetch(taken);
    }
}

/// `e` without the fetch around it, if it has one.
fn unfetch(e: &mut Expr) {
    if let K::Rt(RtOp::GpuFetch, args) = &mut e.kind
        && let Some(list) = args.pop()
    {
        *e = list;
    }
}

/// Wraps the reads of the locals in `set_by_gpu` in `block`, but not of
/// those in `fetched`, which a fetch before a loop brought back already.
fn block(b: &mut Block, set_by_gpu: &HashSet<Local>, fetched: &HashSet<Local>, types: &[Type]) {
    let mut out = Vec::with_capacity(b.stmts.len());
    for mut s in std::mem::take(&mut b.stmts) {
        let body = match &mut s {
            Stmt::While { cond, body } => {
                expr(cond, set_by_gpu, fetched, types);
                Some(body)
            }
            Stmt::ForRange {
                start, end, body, ..
            } => {
                expr(start, set_by_gpu, fetched, types);
                expr(end, set_by_gpu, fetched, types);
                Some(body)
            }
            Stmt::ForEach { list, body, .. } => {
                if set_by_gpu.contains(list) && !fetched.contains(list) {
                    out.push(fetch_stmt(*list, types));
                }
                Some(body)
            }
            _ => None,
        };
        match body {
            Some(body) => {
                let ahead: Vec<Local> = set_by_gpu
                    .iter()
                    .copied()
                    .filter(|l| {
                        !fetched.contains(l) && reads(body, *l) && !walk::sets_local(body, *l)
                    })
                    .collect();
                out.extend(ahead.iter().map(|l| fetch_stmt(*l, types)));
                let inside: HashSet<Local> = fetched.iter().copied().chain(ahead).collect();
                block(body, set_by_gpu, &inside, types);
            }
            None => stmt(&mut s, set_by_gpu, fetched, types),
        }
        out.push(s);
    }
    b.stmts = out;
    if let Some(v) = &mut b.value {
        expr(v, set_by_gpu, fetched, types);
    }
}

fn stmt(s: &mut Stmt, set_by_gpu: &HashSet<Local>, fetched: &HashSet<Local>, types: &[Type]) {
    match s {
        Stmt::Let(_, e) | Stmt::Expr(e) | Stmt::Return(Some(e)) => {
            expr(e, set_by_gpu, fetched, types)
        }
        Stmt::LetResult { call, .. } => expr(call, set_by_gpu, fetched, types),
        Stmt::Check { cond, .. } => expr(cond, set_by_gpu, fetched, types),
        Stmt::Print(parts) => {
            for p in parts {
                if let crate::ir::PrintPart::Value(e) = p {
                    expr(e, set_by_gpu, fetched, types);
                }
            }
        }
        _ => {}
    }
}

fn expr(e: &mut Expr, set_by_gpu: &HashSet<Local>, fetched: &HashSet<Local>, types: &[Type]) {
    match &e.kind {
        K::Local(l) | K::Move(l) if set_by_gpu.contains(l) && !fetched.contains(l) => {
            let read = std::mem::replace(
                e,
                Expr {
                    kind: K::Int(0),
                    ty: e.ty,
                    line: e.line,
                },
            );
            *e = Expr {
                ty: read.ty,
                line: read.line,
                kind: K::Rt(RtOp::GpuFetch, vec![read]),
            };
        }
        _ => walk::parts(e, &mut |p| match p {
            Part::Expr(x) => expr(x, set_by_gpu, fetched, types),
            Part::Block(b) => block(b, set_by_gpu, fetched, types),
        }),
    }
}

/// Whether `body` reads local `l`.
fn reads(body: &Block, l: Local) -> bool {
    walk::any_in_block(
        body,
        &mut |e| matches!(e.kind, K::Local(x) | K::Move(x) if x == l),
    )
}

/// `l = fetch(l)`: its items back from the GPU, once.
fn fetch_stmt(l: Local, types: &[Type]) -> Stmt {
    let ty = types[l as usize];
    let at = |kind| Expr { kind, ty, line: 0 };
    let fetched = at(K::Rt(RtOp::GpuFetch, vec![at(K::Local(l))]));
    Stmt::Expr(Expr {
        kind: K::Assign(l, Box::new(fetched)),
        ty: Type::Unit,
        line: 0,
    })
}

#[cfg(test)]
mod tests {
    use crate::ir::ExprKind;
    use crate::rt_op::RtOp;

    /// How many of each op the program's `main` has.
    fn ops(src: &str, op: RtOp) -> usize {
        let c = kavach_driver::compile(src, "t.kav", Default::default());
        let program = crate::lower(&c).expect("lowers");
        let main = &program.functions[program.entry as usize];
        let mut n = 0;
        crate::walk::any_in_block(&main.body, &mut |e| {
            n += usize::from(matches!(&e.kind, ExprKind::Rt(o, _) if *o == op));
            false
        });
        n
    }

    #[test]
    fn a_loop_of_maps_fetches_once_after_it() {
        let src = "gpu fn step(x: Int32) -> Int32 {\n    x + 1\n}\n\nfn main() {\n    let xs: [Int32] = [1, 2]\n    for _ in 0..9 {\n        xs = xs.map(step)\n    }\n    let t = 0\n    for i in 0..xs.len() {\n        t += Int(xs[i])\n    }\n    print(t)\n}\n";
        assert_eq!(ops(src, RtOp::GpuMapStay), 1);
        // One fetch for `xs.len()` and one before the second loop, none
        // for the `map`'s own list.
        assert_eq!(ops(src, RtOp::GpuFetch), 2);
    }

    #[test]
    fn a_map_only_a_sum_reads_stays_on_the_gpu() {
        let src = "gpu fn step(x: Int32) -> Int32 {\n    x + 1\n}\n\nfn main() {\n    const xs: [Int32] = [1, 2]\n    print(xs.map(step).sum(x => x))\n}\n";
        assert_eq!(ops(src, RtOp::GpuMapStay), 1);
        assert_eq!(ops(src, RtOp::GpuMap), 0);
        assert_eq!(ops(src, RtOp::GpuFetch), 0);
    }

    #[test]
    fn a_sum_min_or_max_reads_the_list_on_the_gpu() {
        let src = "gpu fn step(x: Int32) -> Int32 {
    x + 1
}

fn main() {
    let xs: [Int32] = [1, 2]
    for _ in 0..9 {
        xs = xs.map(step)
    }
    print(xs.sum(x => x))
    print(xs.max())
}
";
        assert_eq!(ops(src, RtOp::GpuSum), 1);
        assert_eq!(ops(src, RtOp::GpuMax), 1);
        assert_eq!(ops(src, RtOp::GpuFetch), 0);
    }

    #[test]
    fn a_filter_kept_in_a_variable_stays_on_the_gpu() {
        let src = "gpu fn step(x: Int32) -> Int32 {\n    x + 1\n}\n\nfn main() {\n    let xs: [Int32] = [1, 2]\n    xs = xs.map(step)\n    let ys = xs.filter(x => x > 1)\n    ys = ys.map(step)\n    print(ys)\n}\n";
        assert_eq!(ops(src, RtOp::GpuFilterStay), 1);
        assert_eq!(ops(src, RtOp::GpuMapStay), 2);
        // Only `print(ys)` reads a list on the CPU.
        assert_eq!(ops(src, RtOp::GpuFetch), 1);
    }

    #[test]
    fn a_map_of_a_value_not_kept_copies_back() {
        let src = "gpu fn step(x: Int32) -> Int32 {\n    x + 1\n}\n\nfn main() {\n    const xs: [Int32] = [1, 2]\n    print(xs.map(step))\n}\n";
        assert_eq!(ops(src, RtOp::GpuMapStay), 0);
        assert_eq!(ops(src, RtOp::GpuFetch), 0);
    }
}
