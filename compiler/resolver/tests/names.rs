//! Name resolution: what each name refers to, and the errors for bad names.

use kavach_resolver::{DefKind, resolve};

fn line_of(src: &str, offset: usize) -> usize {
    src[..offset].matches('\n').count() + 1
}

/// Resolver errors for `src` as "line: message". Fails on syntax errors.
fn errors(src: &str) -> Vec<String> {
    let parsed = kavach_parser::parse(src);
    assert!(
        parsed.errors.is_empty(),
        "syntax errors in {src:?}: {:?}",
        parsed.errors
    );
    resolve(&parsed.file)
        .errors
        .iter()
        .map(|e| format!("{}: {}", line_of(src, e.span.start as usize), e.message))
        .collect()
}

/// What the `nth` (1-based) occurrence of `name` in `src` refers to, as
/// "kind@line" of its declaration.
fn refers(src: &str, name: &str, nth: usize) -> String {
    let parsed = kavach_parser::parse(src);
    let res = resolve(&parsed.file);
    let offset = src.match_indices(name).nth(nth - 1).expect("occurrence").0;
    match res.def_at(offset as u32) {
        Some(def) => {
            let line = def.span.map_or("builtin".to_string(), |s| {
                line_of(src, s.start as usize).to_string()
            });
            format!("{:?}@{line}", def.kind)
        }
        None => "unresolved".to_string(),
    }
}

#[test]
fn names_refer_to_the_right_declaration() {
    let src = "fn main() {\n  const x = 1\n  if true {\n    const x = 2\n    print(x)\n  }\n  print(x)\n}";
    assert_eq!(refers(src, "x)", 1), "Local { mutable: false }@4");
    assert_eq!(refers(src, "x)", 2), "Local { mutable: false }@2");
    assert_eq!(refers(src, "print", 1), "Builtin@builtin");

    let src = "fn main() {\n  print(later(1))\n}\nfn later(n: Int) -> Int {\n  n\n}";
    assert_eq!(refers(src, "later", 1), "Fn@4");
    assert_eq!(refers(src, "n\n", 1), "Param@4");

    let src = "type Status = Pending | Paid\nfn f(s: Status) -> Int {\n  match s {\n    Paid => 1\n    _ => 0\n  }\n}";
    assert!(refers(src, "Paid =>", 1).starts_with("Variant"));
    assert_eq!(refers(src, "Status)", 1), "Type@1");

    let src = "fn half(n: Int) -> Int\n  ensure result * 2 <= n\n{\n  n / 2\n}";
    assert_eq!(refers(src, "result", 1), "Result@builtin");

    let src = "type Account {\n  balance: Int\n  invariant balance >= 0\n}";
    assert_eq!(refers(src, "balance >=", 1), "Field@builtin");
}

#[test]
fn agent_rules_see_the_method_parameters() {
    let src = "capability Payments {\n  fn refund(order_id: Int, amount: Int)\n}\nagent Bot {\n  can Payments.refund\n  limit Payments.refund {\n    sum(amount) <= 100 per day\n  }\n  approve Payments.refund when amount > 5 by \"finance\"\n}";
    assert!(errors(src).is_empty(), "{:?}", errors(src));
    assert_eq!(refers(src, "amount) <=", 1), "Param@builtin");
    assert_eq!(refers(src, "sum", 1), "Builtin@builtin");
}

#[test]
fn errors_explain_the_fix() {
    let cases: &[(&str, &[&str])] = &[
        (
            "fn main() {\n  const total = 1\n  print(totl)\n}",
            &["3: `totl` is not defined; did you mean `total`?"],
        ),
        (
            "fn main() {\n  print(count)\n  const count = 1\n}",
            &["2: `count` is used before its `let`: move the declaration above this line"],
        ),
        (
            "fn main() {\n  const a = 1\n  const a = 2\n}",
            &["3: `a` is already declared in this scope"],
        ),
        (
            "fn main() {\n  const n = 1\n  n = 2\n}",
            &["3: `n` is a `const`, so it can't change: declare it with `let`"],
        ),
        ("fn main() {\n  let n = 1\n  n += 2\n}", &[]),
        (
            "fn f(size: Int) {\n  size = 2\n}",
            &["2: `size` is a parameter, so it can't change: copy it into a `let` first"],
        ),
        (
            "type U {\n  age: Int\n}\nfn main() {\n  const u = U(age: 1)\n  u.age = 2\n}",
            &["6: `u` is a `const`, so it can't change: declare it with `let`"],
        ),
        (
            "fn main() {\n  const orderCount = 1\n}",
            &["2: variables use snake_case: write `order_count`"],
        ),
        (
            "fn getTotal() {}",
            &["1: functions use snake_case: write `get_total`"],
        ),
        (
            "type order_status = Open | closed",
            &[
                "1: types use PascalCase: write `OrderStatus`",
                "1: variants use PascalCase: write `Closed`",
            ],
        ),
        (
            "fn f(x: Strng) {}",
            &["1: `Strng` is not a known type; did you mean `String`?"],
        ),
        (
            "fn f() {}\nfn f() {}",
            &["2: `f` is already declared in this scope"],
        ),
        (
            "fn f(s: Int) -> Int {\n  match s {\n    Missing => 1\n  }\n}",
            &["3: `Missing` is not a variant of any type"],
        ),
        (
            "type T = A | B\nfn f(t: T) -> Int {\n  match t {\n    T.C => 1\n    _ => 0\n  }\n}",
            &["4: `T` has no variant `C`"],
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
fn ambiguous_variants_are_left_to_the_type_checker() {
    let src =
        "type Door = Open | Closed\ntype Shop = Open | Shut\nfn main() {\n  const d = Open\n}";
    let parsed = kavach_parser::parse(src);
    let res = resolve(&parsed.file);
    assert!(res.errors.is_empty());
    let offset = src.rfind("Open").unwrap() as u32;
    assert_eq!(res.ambiguous[&offset].len(), 2);
}

#[test]
fn mixed_scripts_are_a_warning() {
    // The second letter is a Cyrillic `а` that looks like a Latin `a`.
    let src = "fn main() {\n  const pаy = 1\n  const नाम = 2\n  print(pаy + नाम)\n}";
    let parsed = kavach_parser::parse(src);
    let res = resolve(&parsed.file);
    assert!(res.errors.is_empty());
    assert_eq!(res.warnings.len(), 1, "{:?}", res.warnings);
    assert!(res.warnings[0].message.contains("Latin and Cyrillic"));
    assert!(matches!(
        res.defs.iter().find(|d| d.name == "नाम").map(|d| d.kind),
        Some(DefKind::Local { .. })
    ));
}

#[test]
fn unused_names_are_a_warning() {
    // `rest`, `never` and `i` are never read; `_kept` says so on purpose,
    // and `used`, `head` and `count` are used.
    let src = "fn main() {
  const used = 1
  const never = 2
  const [head, ...rest] = [1]
  const _kept = 3
  let count = 0
  count += 1
  for i in 0..2 {
    print(used + head)
  }
}";
    let parsed = kavach_parser::parse(src);
    let res = resolve(&parsed.file);
    assert!(res.errors.is_empty(), "{:?}", res.errors);
    let mut named: Vec<&str> = res
        .warnings
        .iter()
        .filter_map(|w| w.message.strip_prefix('`')?.split('`').next())
        .collect();
    named.sort_unstable();
    assert_eq!(named, ["i", "never", "rest"], "{:?}", res.warnings);
}
