//! Semantic tokens of names reached through a module (`tokens.rs`).

/// `matrix.Matrix(…)` makes a record: its name is a type, not a function,
/// though it is called (to-do item 18).
#[test]
fn a_type_made_through_its_module_is_a_type() {
    let text = "use std.matrix\n\nfn main() {\n    const m = matrix.Matrix(rows: 1, cols: 1, items: [2.0])\n    print(matrix.of([[1.0]]).rows + m.rows)\n}\n";
    let c = kavach_driver::compile(text, "main.kav", Default::default());
    let marks = crate::tokens::marks(&c, "main.kav", text);
    let at = |line: u64, col: u64| {
        marks
            .iter()
            .find(|m| m.0 == line && m.1 == col)
            .map(|m| m.3)
    };
    // `Matrix` on line 3, column 21; `of` on line 4, column 17.
    assert_eq!(at(3, 21), Some(4), "{marks:?}");
    assert_eq!(at(4, 17), Some(1), "{marks:?}");
}
