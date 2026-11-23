// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Semantic tokens of names reached through a module (`tokens.rs`).

/// `tensor.Tensor(…)` makes a record: its name is a type, not a function,
/// though it is called (to-do item 18).
#[test]
fn a_type_made_through_its_module_is_a_type() {
    let text = "use std.tensor\n\nfn main() {\n    const m = tensor.Tensor(shape: [1], items: [2.0])\n    print(tensor.of([1.0]).len() + m.len())\n}\n";
    let c = kavach_driver::compile(text, "main.kav", Default::default());
    let marks = crate::tokens::marks(&c, "main.kav", text);
    let at = |line: u64, col: u64| {
        marks
            .iter()
            .find(|m| m.0 == line && m.1 == col)
            .map(|m| m.3)
    };
    // `Tensor` on line 3, column 21; `of` on line 4, column 17.
    assert_eq!(at(3, 21), Some(4), "{marks:?}");
    assert_eq!(at(4, 17), Some(1), "{marks:?}");
}

/// `a @ b` and `m[0, 1]` call a method whose name isn't written: nothing is
/// marked at the operator or the bracket (the docs session's report).
#[test]
fn an_operator_is_not_marked_as_its_method() {
    let text = "use std.tensor\n\nfn main() {\n    const m = tensor.of_rows([[1.0, 2.0], [3.0, 4.0]])\n    print((m @ m + m)[0, 1])\n}\n";
    let c = kavach_driver::compile(text, "main.kav", Default::default());
    let marks = crate::tokens::marks(&c, "main.kav", text);
    // Line 4 holds only `print`, `m` and operators: no method is marked.
    let methods: Vec<_> = marks.iter().filter(|m| m.0 == 4 && m.3 == 2).collect();
    assert!(methods.is_empty(), "{marks:?}");
}
