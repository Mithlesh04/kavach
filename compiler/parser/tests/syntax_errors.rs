// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

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
            "fn f(xs: [Int, max: 2]) {}",
            &["1: a list's numbers go after its type: write `[Int]<max: 2>` (spec 2.2)"],
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
            "const m = [\"a\": 1]",
            &["1: a dict is written with braces: `{\"asha\": 31}` (spec 2.2)"],
        ),
        (
            "const m = [:]",
            &["1: an empty dict is written `{:}` (spec 2.2)"],
        ),
        (
            "const m: [String: Int] = {:}",
            &["1: a dict's type is written with braces: `{String: Int}` (spec 2.2)"],
        ),
        (
            "pub(crate) fn f() {\n}",
            &[
                "1: `pub(…)` takes only `out`: `pub(out)` offers a name to other packages (spec 10.1)",
            ],
        ),
        (
            "pub(out) test \"t\" {\n}",
            &[
                "1: `pub(out)` goes before `fn`, `type`, `const`, `interface`, `capability`, `agent`, `literal` or `use` (spec 10.1)",
            ],
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
            &["1: `pub` only goes before a declaration such as `fn`, `type` or `const`"],
        ),
        ("}", &["1: unexpected `}`: nothing here to close"]),
        (
            "const x = #1",
            &[
                "1: `#` counts from the end, and is written only inside `[ ]`: `xs[#1]` is the last item (spec 2.19)",
            ],
        ),
        (
            "const x = 1;",
            &["1: Kavach has no semicolons: a statement ends at the end of its line"],
        ),
        // A variant's name is one word; text goes in its value (spec 2.6).
        (
            "type Country = India | U.S | UnitedKingdom",
            &[
                "1: `U.S` is not a variant name: a name is one word, with no dots or spaces. To keep this text, write `US = \"U.S\"` (spec 2.6)",
            ],
        ),
        (
            "type Country = India | GREAT AMERICA",
            &[
                "1: `GREAT AMERICA` is not a variant name: a name is one word, with no dots or spaces. To keep this text, write `GreatAmerica = \"GREAT AMERICA\"` (spec 2.6)",
            ],
        ),
        (
            "type Country = \"INDIA\" | \"US\"",
            &[
                "1: a variant is a name, not text: to keep the text, write `India = \"INDIA\"` (spec 2.6)",
                "1: a variant is a name, not text: to keep the text, write `US = \"US\"` (spec 2.6)",
            ],
        ),
        (
            "type Status = Paid = `{x}`",
            &["1: a variant's text is fixed when compiling: write it without `{…}` (spec 2.6)"],
        ),
        // A keyword used as a name: one error at the name (spec 1.6).
        (
            "fn while(n: Int) -> Int {\n    n + 1\n}\nfn main() {\n    print(1)\n}",
            &["1: `while` is a keyword, so it can't name a function: choose another name"],
        ),
        (
            "fn area(type: Int) -> Int {\n    2\n}",
            &["1: `type` is a keyword, so it can't name a parameter: choose another name"],
        ),
        (
            "fn main() {\n    const if = 1\n    print(2)\n}",
            &["2: `if` is a keyword, so it can't name a variable: choose another name"],
        ),
        (
            "type Order {\n    agent: String\n}",
            &["2: `agent` is a keyword, so it can't name a field: choose another name"],
        ),
        (
            "type match {\n    id: Int\n}",
            &["1: `match` is a keyword, so it can't name a type: choose another name"],
        ),
        // `test` is a keyword only where a test block starts (spec 1.6).
        (
            "fn test() -> Int {\n    1\n}\nfn main() {\n    const test = test()\n    print(test)\n}\ntest \"one\" {\n    assert test() == 1\n}",
            &[],
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
