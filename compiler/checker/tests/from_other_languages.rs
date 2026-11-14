//! Code written the way other languages write it gets one clear error
//! that says what Kavach does instead, on the right line.

/// The first error compiling `src`, as "line: message".
fn first_error(src: &str) -> String {
    let c = kavach_driver::compile(src, "main.kav", Default::default());
    let line = |start: u32| src[..start as usize].matches('\n').count() + 1;
    c.messages
        .iter()
        .find(|m| m.severity == kavach_driver::Severity::Error)
        .map(|m| format!("{}: {}", line(m.span.start), m.text))
        .unwrap_or_default()
}

#[test]
fn habits_from_other_languages_get_clear_errors() {
    let main = |body: &str| format!("fn main() {{\n{body}\n}}\n");
    let cases: &[(String, &str)] = &[
        (main("const a = 1\nprint(a === 1)"), "3: Kavach has no `===`: write `==`"),
        (main("const a = 1\nprint(a !== 2)"), "3: Kavach has no `!==`: write `!=`"),
        (
            "type Animal {\n    name: String\n}\n\ntype Dog: Animal {\n    bark: Bool\n}\n\nfn main() {}\n".into(),
            "5: `Dog` can't extend `Animal`: `Animal` is a type, not an interface, and Kavach has no inheritance",
        ),
        (main("const match = 3"), "2: `match` is a keyword, so it can't be a name"),
        (main("const 2nd_place = \"Ravi\""), "2: a name can't start with a digit (spec 1.5): start it with a letter, as `place_2nd`"),
        (main("const f = (inout n: Int) => n + 1"), "2: a lambda can't have `inout` parameters (spec 3.9)"),
        (main("let x: String? = null\nprint(x ?? \"a\")"), "2: Kavach has no `null`: a value that may be missing has an optional type"),
        (main("const prices = [1, 2]\nprint(prices.sum())"), "3: `sum` takes a function that gives each item's number"),
        (main("print(\"abcd\".slice(-4))"), "2: `slice` takes where to start and where to end"),
        (main("const ready = false\nprint(!ready)"), "3: Kavach has no `!` for \"not\": write `not`, as in `not ready` (spec 1.8)"),
        (
            "fn total(prices: [Int]) -> Int {\n    prices.sum(x => x)\n}\nfn main() {\n    print(total(4, 8, 15))\n}\n".into(),
            "5: `total` takes one list, `prices`, not separate values: put them in a list, as in `total([…])` (spec 3.1)",
        ),
        (main("print(true && false)"), "2: Kavach has no `&&`: write `and` (spec 1.8)"),
        (main("print(true || false)"), "2: Kavach has no `||`: write `or` (spec 1.8)"),
        (main("print([2, 3, [4, 5], 5])"), "2: expected `Int`, found `[Int]`: a list holds one type; to mix numbers and lists, use a union (2.6) or `Json` (2.12)"),
        // Found by the AI-writability benchmark (benchmarks/ai-writes).
        (main("const xs = [1]\nprint(len(xs))"), "3: Kavach has no `len` function: it is a method, so write `xs.len()`"),
        (main("const xs = [1]\nprint(sorted(xs))"), "3: Kavach has no `sorted` function: it is a method, so write `xs.sort()`"),
        (main("print(str(5))"), "2: Kavach has no `str` function: write `String(x)`"),
        (main("const xs = [1]\nprint(xs.count())"), "3: `count` takes a test and counts the items it holds for"),
        (main("print(\"ab\".length)"), "2: `String` has no field `length`: Kavach's is the method `len()`"),
        (main("let xs = [1]\nxs.append(2)"), "3: `[Int]` has no method `append`: Kavach's is `push(x)`"),
        (main("print(\"Ab\".to_lowercase())"), "2: `String` has no method `to_lowercase`: Kavach's is `lower()`"),
        (main("let xs = [1]\nprint(xs.pop())"), "3: `[Int]` has no method `pop` yet"),
        (main("const xs = [1]\nfor i in 0..xs.max() ?? 0 {\n}"), "3: `??` binds looser than `..`"),
        (
            "fn one() -> Int {\n    return 1\n}\nfn main() {\n    const t = spawn one()\n    print(t.join())\n}\n".into(),
            "6: `Task<Int>` has no method `join`: Kavach's is `wait()`",
        ),
        (
            "use std.log\n\nfn main() {\n    log.info(\"order placed\")\n}\n".into(),
            "4: `String` does not implement `LogEvent`: log events are records",
        ),
    ];
    let failures: Vec<String> = cases
        .iter()
        .filter_map(|(src, want)| {
            let got = first_error(src);
            (!got.starts_with(want))
                .then(|| format!("source: {src:?}\n  want: {want}\n   got: {got}"))
        })
        .collect();
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}
