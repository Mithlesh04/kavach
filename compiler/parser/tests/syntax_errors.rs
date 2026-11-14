//! Syntax errors: clear messages on the right line, and parsing continues.

mod common;
use common::errors;

#[test]
fn messages_point_at_the_problem() {
    let cases: &[(&str, &[&str])] = &[
        (
            "fn f(x) -> Int {}",
            &["1: write the type of `x`: parameters always have a type"],
        ),
        (
            "fn f(a: Int) -> Int\n  require a > 0\n  ! E\n{}",
            &["3: `!` must come before `require`"],
        ),
        (
            "fn f() -> Int -> Int {}",
            &["1: a signature has only one `->`"],
        ),
        (
            "fn f() -> Int",
            &["1: expected `{` to start the body of `f`"],
        ),
        (
            "var count = 0",
            &[
                "1: `var` is not Kavach: write `let` for a variable, or `const` for a value that never changes",
            ],
        ),
        (
            "let mut count = 0",
            &["1: Kavach has no `let mut`: a `let` can already change, so write `let count`"],
        ),
        (
            "a = b = 1",
            &[
                "1: assignment is a statement, not an expression: put each assignment on its own line",
            ],
        ),
        (
            "f(1) = 2",
            &["1: only a variable, field or index can be assigned to"],
        ),
        (
            "if x = 4 {\n}",
            &["1: `=` sets a variable: to compare, write `==` (spec 4.1)"],
        ),
        (
            "deprecated fix b()\nfn a() {\n}",
            &["1: say what to use instead, in quotes: `deprecated \"use refund_order instead\"`"],
        ),
        (
            "while n = 0 {\n}",
            &["1: `=` sets a variable: to compare, write `==` (spec 4.1)"],
        ),
        (
            "0..5..9",
            &["1: ranges can't be chained: use `and` or parentheses"],
        ),
        (
            "greet(greeting: \"Hi\", \"Asha\")",
            &["1: positional arguments must come before named ones"],
        ),
        (
            "const x = .name",
            &["1: a leading `.` only works as a function argument, like `items.filter(.paid)`"],
        ),
        (
            "const x = 1 2",
            &["1: expected the end of the statement: start a new line"],
        ),
        (
            "pub let x = 1",
            &["1: `pub` only goes before a declaration such as `fn` or `type`"],
        ),
        ("}", &["1: unexpected `}`: nothing here to close"]),
        (
            "const x = 1;",
            &["1: Kavach has no semicolons: a statement ends at the end of its line"],
        ),
    ];
    let failures: Vec<String> = cases
        .iter()
        .filter_map(|(src, want)| {
            let got = errors(src);
            (got != *want).then(|| format!("source: {src:?}\n  want: {want:?}\n   got: {got:?}"))
        })
        .collect();
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn parsing_continues_after_an_error() {
    let src = "const a = 1 2\nconst b = ]\nfn ok() {}\nconst c = 3";
    let parsed = kavach_parser::parse(src);
    assert_eq!(parsed.errors.len(), 2, "{:?}", parsed.errors);
    assert_eq!(parsed.file.items.len(), 4);
}
