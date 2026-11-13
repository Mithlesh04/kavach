//! Type checking: valid programs pass, and mistakes get clear messages on
//! the right line.

use kavach_checker::{Options, check};

/// All errors from parsing, resolving and checking `src`, as "line: message".
fn errors(src: &str) -> Vec<String> {
    let parsed = kavach_parser::parse(src);
    let resolved = kavach_resolver::resolve(&parsed.file);
    let checked = check(&parsed.file, &resolved, Options::default());
    let line = |start: u32| src[..start as usize].matches('\n').count() + 1;
    let syntax = parsed.errors.iter().map(|e| (e.span.start, &e.message));
    let later = resolved
        .errors
        .iter()
        .chain(&checked.errors)
        .map(|e| (e.span.start, &e.message));
    syntax
        .chain(later)
        .map(|(start, message)| format!("{}: {message}", line(start)))
        .collect()
}

fn check_all(cases: &[(&str, &[&str])]) {
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
fn valid_programs_have_no_errors() {
    check_all(&[
        (
            "fn main() {\n  const price: Decimal = 0.1\n  const n: UInt8 = 200\n  const f: Float = 1e3\n}",
            &[],
        ),
        (
            "fn first<T>(items: [T]) -> T? {\n  items.get(0)\n}\nfn main() {\n  const n: Int = first([1, 2]) ?? 0\n}",
            &[],
        ),
        (
            "type Pair<A, B> {\n  left: A\n  right: B\n}\nfn main() {\n  const p = Pair(left: 1, right: \"one\")\n  const s: String = p.right\n}",
            &[],
        ),
        (
            "fn main() {\n  const total = [1, 2, 3].filter(n => n > 1).map(n => n * 2).sum(n => n)\n  print(total + 1)\n}",
            &[],
        ),
        (
            "type T = A | B(x: Int)\nfn f(t: T) -> Int {\n  match t {\n    T.A => 0\n    T.B(x) => x\n  }\n}",
            &[],
        ),
        (
            "fn f(x: Int?) -> Int {\n  if let v = x {\n    v\n  } else {\n    0\n  }\n}",
            &[],
        ),
        (
            "fn g() -> Int ! Error {\n  const n = try Int(\"4\") context \"reading n\"\n  n\n}",
            &[],
        ),
        (
            "fn main() {\n  for (i, name) in [\"a\"].enumerate() {\n    print(\"{i} {name}\")\n  }\n}",
            &[],
        ),
    ]);
}

#[test]
fn type_mismatches() {
    check_all(&[
        (
            "fn main() {\n  const n: Int = \"text\"\n}",
            &["2: expected `Int`, found `String`"],
        ),
        (
            "fn main() {\n  const a: Int = 1\n  const b: Float = a\n}",
            &[
                "3: there are no implicit number conversions: expected `Float`, found `Int`; convert with `Float(…)`",
            ],
        ),
        (
            "fn main() {\n  const x: Int = none\n}",
            &["2: `Int` can't hold `none`: write `Int?` for a value that may be missing"],
        ),
        (
            "fn f() -> Int {\n  const x = 1\n}",
            &["1: `f` must return `Int`, but its last line has no value"],
        ),
        (
            "fn main() {\n  const b = true + 1\n}",
            &["2: expected a number, found `Bool`"],
        ),
        (
            "fn main() {\n  const s = Set([1, 2])\n}",
            &[
                "2: `Set` can't be called like a function: make a set with `Set.of(1, 2, 3)`, or `Set.with_capacity(n)` and `add` (spec 2.2)",
            ],
        ),
        (
            "fn main() {\n  const xs = [1] + [2]\n}",
            &[
                "2: expected a number, found `[Int]`: `+` doesn't join lists; copy one into a `let` (`let all = xs`) and `push` the other's items in a `for` loop (spec 2.2)",
            ],
        ),
        (
            "fn main() {\n  const s = \"a\" + 1\n}",
            &["2: expected `String`, found `Int`"],
        ),
        (
            "fn main() {\n  const x = 5\n  const y = x ?? 1\n}",
            &[
                "3: `??` needs an optional value on the left, but this is `Int`, which is never `none`",
            ],
        ),
        (
            "fn main() {\n  if 1 {\n  }\n}",
            &[
                "2: conditions must be `Bool`, not `Int`: there is no truthiness, so compare explicitly (for example `!= 0`)",
            ],
        ),
    ]);
}

#[test]
fn records_and_fields() {
    let user = "type User {\n  name: String\n  age: Int = 0\n}\n";
    let cases = [
        (
            format!("{user}fn main() {{\n  const u = User(name: \"A\", agee: 3)\n}}"),
            vec!["6: `User` has no field `agee`"],
        ),
        (
            format!("{user}fn main() {{\n  const u = User(name: \"A\")\n  print(u.nme)\n}}"),
            vec!["7: `User` has no field `nme`"],
        ),
        (
            format!(
                "{user}fn main() {{\n  const u = User(name: \"A\")\n  const v = u with {{ age: \"x\" }}\n}}"
            ),
            vec!["7: expected `Int`, found `String`"],
        ),
        (
            format!("{user}fn f(u: User?) -> String {{\n  u.name\n}}"),
            vec!["6: `User?` might be `none`: use `?.name` or `if let`"],
        ),
        (
            "type Box<I> {
  n: Int
}
fn main() {
  const b: Box<String> = Box<Int>(n: 1)
  const c = Box<Int, Int>(n: 2)
}"
            .to_string(),
            vec![
                "5: expected `Box<String>`, found `Box<Int>`",
                "6: `Box` takes 1 type argument, not 2",
            ],
        ),
    ];
    for (src, want) in cases {
        assert_eq!(errors(&src), want, "{src}");
    }
}

#[test]
fn values_must_be_used_and_must_exist() {
    check_all(&[
        (
            "fn add(a: Int, b: Int) -> Int {\n  a + b\n}\nfn main() {\n  add(1, 2)\n  print(1)\n}",
            &["5: the result of `add` is unused: use it, or write `const _ = …` to throw it away"],
        ),
        (
            "fn f() {\n  print(1)\n}\nfn main() {\n  const x = f()\n}",
            &["5: `f` returns nothing, so there is no value to keep"],
        ),
        (
            "fn helper(n: Int) {\n  n * 2\n}",
            &[
                "2: `helper` has no `->`, so it returns nothing, but its last line is a value: write `-> Int`",
            ],
        ),
        (
            "fn f(n: Int) -> Int {\n  n\n}\nfn helper() {\n  f(1)\n}",
            &[
                "5: the result of `f` is unused: use it, write `const _ = …`, or add `-> Int` if `helper` should return it",
            ],
        ),
    ]);
}

#[test]
fn errors_must_be_handled() {
    let defs = "type E = Bad | Worse(n: Int)\ntype Other = Nope\nfn risky(n: Int) -> Int ! E {\n  if n < 0 {\n    fail E.Bad\n  }\n  n\n}\n";
    let cases = [
        (
            format!("{defs}fn main() {{\n  const x = risky(1)\n}}"),
            vec!["10: `risky` can fail: handle the error with `try`, `catch` or `match`"],
        ),
        (
            format!("{defs}fn main() {{\n  const xs = [1].filter(n => risky(n) > 0)\n}}"),
            vec![
                "10: `risky` can fail: write `try` before it to pass the error on from this lambda, which the call that takes the lambda then passes on (spec 3.3), or handle it here with `catch` or `match`",
            ],
        ),
        (
            format!("{defs}fn f() -> Int ! Other {{\n  try risky(1)\n}}"),
            vec!["10: `try` can't pass on `E`: `f` only returns `! Other`"],
        ),
        (
            format!("{defs}fn f() -> Int {{\n  fail E.Bad\n}}"),
            vec![
                "10: `fail` returns an error, but `f` has no `!` in its signature: add `! ErrorType`",
            ],
        ),
        (
            format!(
                "{defs}fn main() {{\n  match risky(1) {{\n    ok(v) => print(v)\n    err(E.Bad) => print(0)\n  }}\n}}"
            ),
            vec!["10: this `match` does not cover `err(E.Worse)`"],
        ),
        (
            format!(
                "{defs}fn main() {{\n  const x = risky(1) catch 0\n  const y = risky(2) catch e => {{\n    print(\"{{e}}\")\n    1\n  }}\n}}"
            ),
            vec![],
        ),
    ];
    for (src, want) in cases {
        assert_eq!(errors(&src), want, "{src}");
    }
}

#[test]
fn match_must_cover_every_case() {
    check_all(&[
        (
            "fn f(b: Bool) -> Int {\n  match b {\n    true => 1\n  }\n}",
            &["2: this `match` does not cover `false`"],
        ),
        (
            "fn f(n: Int) -> Int {\n  match n {\n    0 => 1\n  }\n}",
            &["2: this `match` does not cover `every other value (add `_ => …`)`"],
        ),
        (
            "fn f(x: Int?) -> Int {\n  match x {\n    none => 0\n  }\n}",
            &["2: this `match` does not cover `a present value (`value => …`)`"],
        ),
        (
            "type S = A | B\nfn f(s: S) -> Int {\n  match s {\n    S.A => 1\n    S.B if false => 2\n  }\n}",
            &["3: this `match` does not cover `S.B`"],
        ),
    ]);
}

#[test]
fn unsafe_is_allowed_when_the_package_opts_in() {
    let src = "fn main() {\n  unsafe {\n    const p = alloc(8)\n  }\n}";
    let parsed = kavach_parser::parse(src);
    let resolved = kavach_resolver::resolve(&parsed.file);
    assert_eq!(
        check(&parsed.file, &resolved, Options::default())
            .errors
            .len(),
        1
    );
    assert!(
        check(
            &parsed.file,
            &resolved,
            Options {
                allow_unsafe: true,
                ..Options::default()
            }
        )
        .errors
        .is_empty()
    );
}

/// An implementing method has the interface's types, read with `Self` as
/// the type and the interface's type arguments put in (spec 2.8).
#[test]
fn implementing_methods_keep_the_interface_types() {
    let iface =
        "interface Shape<T> {\n  fn scale(self, by: T) -> Self\n  fn area(self) -> T ! String\n}\n";
    let ok = format!(
        "{iface}type Sq: Shape<Int> {{\n  side: Int\n  fn scale(self, by: Int) -> Sq {{ Sq(side: self.side * by) }}\n  fn area(self) -> Int ! String {{ self.side }}\n}}\nfn main() {{}}"
    );
    let bad = format!(
        "{iface}type Sq: Shape<Int> {{\n  side: Int\n  fn scale(self, by: Float) -> Int {{ 1 }}\n  fn area(self) -> Int ! Int {{ self.side }}\n}}\nfn main() {{}}"
    );
    check_all(&[
        (&ok, &[]),
        (
            &bad,
            &[
                "7: `by` of `Sq.scale` is `Float`, and in `Shape<Int>.scale` it is `Int`: they must match",
                "8: `Sq.area` fails with `Int`, and `Shape<Int>.area` with `String`: they must match",
            ],
        ),
        (
            &bad.replace("by: Float", "by: Int"),
            &[
                "7: `Sq.scale` gives `Int`, and `Shape<Int>.scale` gives `Sq`: they must match",
                "8: `Sq.area` fails with `Int`, and `Shape<Int>.area` with `String`: they must match",
            ],
        ),
    ]);
}

#[test]
fn a_bad_number_gives_one_error() {
    // Not also "`print` takes one argument", as when the number was dropped.
    check_all(&[(
        "fn main() {
    print(0o8)
}
",
        &["2: invalid digit in octal number"],
    )]);
}
