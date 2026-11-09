//! Warnings: code that works but rarely means what it says (spec 1.8,
//! 3.3, 5.2), on the right line, and none where the code is fine.

use kavach_checker::{Options, check};

/// The checker's warnings for `src`, as "line: start of the message".
fn warnings(src: &str) -> Vec<String> {
    let parsed = kavach_parser::parse(src);
    let resolved = kavach_resolver::resolve(&parsed.file);
    let checked = check(&parsed.file, &resolved, Options::default());
    assert!(checked.errors.is_empty(), "{:?}", checked.errors);
    let line = |start: u32| src[..start as usize].matches('\n').count() + 1;
    checked
        .warnings
        .iter()
        .map(|w| {
            let words: Vec<&str> = w.message.split_whitespace().take(4).collect();
            format!("{}: {}", line(w.span.start), words.join(" "))
        })
        .collect()
}

#[test]
fn comparisons_that_rarely_mean_what_they_say() {
    let main = |body: &str| format!("fn main() {{\n{body}\n}}");
    let cases: &[(&str, &[&str])] = &[
        // `==` on floats worked out while running.
        (
            "let a = 0.1\nprint(a + 0.2 == 0.3)",
            &["3: `==` on floats compares"],
        ),
        (
            "let a = 0.1\nprint(a != 0.3)",
            &["3: `!=` on floats compares"],
        ),
        (
            "let a = 0.1\nprint(0.0 <= a == a)",
            &["3: `==` on floats compares"],
        ),
        // Fine: a whole number, two constants, `near`, other types.
        ("let a = 0.1\nprint(a == 0.0)", &[]),
        ("print(0.1 + 0.2 == 0.3)", &[]),
        ("let a = 0.1\nprint(a.near(0.3))", &[]),
        ("let s = \"a\"\nprint(s == \"a\")", &[]),
        // `x % 2 == 1` on a signed number.
        ("let n = -3\nprint(n % 2 == 1)", &["3: `x % 2 =="]),
        ("let n = -3\nprint(1 != n % 2)", &["3: `x % 2 !="]),
        ("let n = -3\nprint(n % 2 != 0)", &[]),
        ("const u: UInt8 = 3\nprint(u % 2 == 1)", &[]),
    ];
    let failures: Vec<String> = cases
        .iter()
        .filter_map(|(body, want)| {
            let got = warnings(&main(body));
            (got != *want).then(|| format!("body: {body:?}\n  want: {want:?}\n   got: {got:?}"))
        })
        .collect();
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn try_and_catch_on_what_cant_fail() {
    let src = "fn two() -> Int {\n  2\n}\n\nfn main() {\n  print(two() catch 0)\n}";
    assert_eq!(warnings(src), ["6: `catch` does nothing here:"]);
    let fails = "fn read(t: String) -> Int ! ParseError {\n  try Int(t)\n}\n\nfn main() {\n  print(read(\"4\") catch 0)\n}";
    assert_eq!(warnings(fails), Vec::<String>::new());
}

#[test]
fn changes_to_a_copy_inside_a_lambda_or_task() {
    let main = |body: &str| format!("fn main() {{\n{body}\n}}");
    let cases: &[(&str, &[&str])] = &[
        (
            "let count = 0\nconst f = () => {\ncount += 1\n}\nf()\nprint(count)",
            &["4: `count` is a copy"],
        ),
        (
            "let xs = [1]\nconst t = spawn {\nxs[0] = 2\nxs.len()\n}\nprint(t.wait() catch _ => 0)",
            &["4: `xs` is a copy"],
        ),
        // Fine: a variable of the lambda's own, and a change outside one.
        (
            "const f = () => {\nlet mine = 0\nmine += 1\nmine\n}\nprint(f())",
            &[],
        ),
        ("let count = 0\ncount += 1\nprint(count)", &[]),
    ];
    let failures: Vec<String> = cases
        .iter()
        .filter_map(|(body, want)| {
            let got = warnings(&main(body));
            (got != *want).then(|| format!("body: {body:?}\n  want: {want:?}\n   got: {got:?}"))
        })
        .collect();
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}
