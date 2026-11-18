use crate::ir::{ExprKind, Program, Shape, Type};
use crate::rt_op::RtOp;

/// `body` ends `step` with enough `if`s that it is too big to inline.
fn padded(body: &str, var: &str) -> String {
    let ifs: String = (0..14)
        .map(|i| format!("    if by == {} {{\n        {var} += {i}\n    }}\n", 40 + i))
        .collect();
    body.replace("    // padding\n", &ifs)
}

fn lowered(src: &str) -> Program {
    let c = kavach_driver::compile(src, "t.kav", Default::default());
    crate::lower(&c).expect("lowers")
}

fn stashes(program: &Program) -> bool {
    program.functions.iter().any(|f| {
        crate::walk::any_in_block(&f.body, &mut |e| {
            matches!(e.kind, ExprKind::Rt(RtOp::InoutStash, _))
        })
    })
}

/// A function too big to inline, with an `inout` parameter and a result
/// besides, gives its value alone and builds no tuple (S11).
#[test]
fn a_result_besides_an_inout_comes_back_with_no_tuple() {
    let src = padded(
        r#"type R {
    count: Int
}

fn step(inout r: R, by: Int) -> Bool {
    r.count += 1
    // padding
    by % 2 == 0
}

fn main() {
    let r = R(count: 0)
    let even = 0
    for i in 0..10 {
        if step(r, i) {
            even += 1
        }
    }
    print(`{r.count} {even}`)
}
"#,
        "r.count",
    );
    let program = lowered(&src);
    for f in &program.functions {
        let tuples =
            crate::walk::any_in_block(&f.body, &mut |e| matches!(e.kind, ExprKind::Tuple(_)));
        assert!(!tuples, "{}: {:#?}", f.name, f.body);
        if let Type::Ref(id) = f.ret {
            assert!(
                !matches!(program.shapes[id as usize], Shape::Tuple(_)),
                "{}",
                f.name
            );
        }
    }
    assert!(
        stashes(&program),
        "the call was inlined: make `step` bigger"
    );
}

/// A function a `drop` method can call keeps its tuple: a `drop` may run
/// between the stash and the take and overwrite the slots.
#[test]
fn a_function_a_drop_calls_keeps_its_tuple() {
    let src = padded(
        r#"type Guard: Drop {
    name: String

    fn drop(self) {
        let k = 0
        const odd = step(k, 3)
        print(`drop {self.name} {k} {odd}`)
    }
}

fn step(inout n: Int, by: Int) -> Bool {
    n += by
    // padding
    n % 2 != 0
}

fn main() {
    let n = 0
    for i in 0..3 {
        const g = Guard(name: `g{i}`)
        if step(n, i) {
            print(n)
        }
    }
}
"#,
        "n",
    );
    assert!(
        !stashes(&lowered(&src)),
        "a function `drop` reaches used the slots"
    );
}
