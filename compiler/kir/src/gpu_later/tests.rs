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

/// A program with `body` in `main`.
fn program(body: &str) -> String {
    format!(
        "gpu fn step(x: Int32) -> Int32 {{\n    x + 1\n}}\n\nfn main() ! Error {{\n    const rounds: [[Int32]] = [[1, 2]]\n    let total = 0\n{body}\n    print(total)\n}}\n"
    )
}

fn waits(src: &str) -> bool {
    ops(src, RtOp::GpuSum) == 1 && ops(src, RtOp::GpuSumLater) == 0
}

#[test]
fn a_sum_only_its_statement_reads_doesnt_hold_up_the_loop() {
    let src =
        program("    for xs in rounds {\n        total += Int(xs.map(step).sum(x => x))\n    }");
    assert_eq!(ops(&src, RtOp::GpuSumLater), 1);
    assert_eq!(ops(&src, RtOp::GpuSum), 0);
    // One in the loop, one after it; and the map's result stays.
    assert_eq!(ops(&src, RtOp::GpuLaterNext), 2);
    assert_eq!(ops(&src, RtOp::GpuMapStay), 1);
    assert_eq!(ops(&src, RtOp::GpuFetch), 0);
}

#[test]
fn a_sum_kept_in_a_const_for_one_statement_is_the_same() {
    let src = program(
        "    for xs in rounds {\n        const s = xs.map(step).sum(x => x)\n        total += Int(s)\n    }",
    );
    assert_eq!(ops(&src, RtOp::GpuSumLater), 1);
}

#[test]
fn a_loop_that_reads_the_total_waits() {
    let src = program(
        "    for xs in rounds {\n        total += Int(xs.map(step).sum(x => x))\n        print(total)\n    }",
    );
    assert!(waits(&src));
}

#[test]
fn a_sum_of_a_list_on_the_cpu_is_left_alone() {
    let src = program("    for xs in rounds {\n        total += Int(xs.sum(x => x))\n    }");
    assert!(waits(&src));
}

#[test]
fn a_statement_that_does_more_than_numbers_waits() {
    let src = format!(
        "fn note(n: Int) -> Int {{\n    print(n)\n    n\n}}\n\n{}",
        program(
            "    for xs in rounds {\n        total += note(Int(xs.map(step).sum(x => x)))\n    }"
        )
    );
    assert!(waits(&src));
}

#[test]
fn a_captured_value_the_loop_changes_keeps_it_waiting() {
    let src = program(
        "    let k: Int32 = 1\n    for xs in rounds {\n        total += Int(xs.map(step).sum(x => x + k))\n        k = k + 1\n    }",
    );
    assert!(waits(&src));
}

#[test]
fn leaving_the_function_runs_what_is_left_first() {
    let src = program(
        "    for xs in rounds {\n        total += Int(xs.map(step).sum(x => x))\n        if xs.len() == 0 {\n            fail Error(\"empty\")\n        }\n    }",
    );
    assert_eq!(ops(&src, RtOp::GpuSumLater), 1);
    // In the loop, after it, and before the `fail`.
    assert_eq!(ops(&src, RtOp::GpuLaterNext), 3);
}
