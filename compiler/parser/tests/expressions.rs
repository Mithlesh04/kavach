//! Expressions: each case is source and the tree it must produce.

mod common;
use common::check_all;

#[test]
fn precedence_follows_spec_1_8() {
    check_all(&[
        ("1 + 2 * 3", "(+ 1 (* 2 3))"),
        ("(1 + 2) * 3", "(* (+ 1 2) 3)"),
        ("not a and b", "(and (not a) b)"),
        ("a or b and c", "(or a (and b c))"),
        ("a == b and c < d", "(and (== a b) (< c d))"),
        ("b == a == c", "(chain b == a == c)"),
        ("0 <= i < n + 1 and ok", "(and (chain 0 <= i < (+ n 1)) ok)"),
        ("(a == b) == c", "(== (== a b) c)"),
        ("2 ** 3 ** 2", "(** 2 (** 3 2))"),
        (
            "123456789012345678901234567890",
            "123456789012345678901234567890",
        ),
        ("-2 ** 2", "(- (** 2 2))"),
        ("a * b ** 2", "(* a (** b 2))"),
        ("2 ** -n", "(** 2 (- n))"),
        ("x ?? 1 + 1", "(?? x (+ 1 1))"),
        ("0..n + 1", "(.. 0 (+ n 1))"),
        ("a | b ^ c & d", "(| a (^ b (& c d)))"),
        ("1 << 2 + 3", "(<< 1 (+ 2 3))"),
        ("-x * y", "(* (- x) y)"),
        ("a - b - c", "(- (- a b) c)"),
    ]);
}

#[test]
fn literals_and_collections() {
    check_all(&[
        ("[1, 2, 3]", "(list 1 2 3)"),
        ("[]", "(list )"),
        (
            "[\"asha\": 31, \"ravi\": 27]",
            "(map ((str \"asha\") 31) ((str \"ravi\") 27))",
        ),
        ("[:]", "(map )"),
        ("(200, \"OK\")", "(tuple 200 (str \"OK\"))"),
        ("(x)", "x"),
        ("'क'\ntrue\nnone\nself\n2.5", "'क'\ntrue\nnone\nself\n2.5"),
    ]);
}

#[test]
fn strings_with_interpolation() {
    check_all(&[
        ("\"Hi {user.name}!\"", "(str \"Hi \" (. user name) \"!\")"),
        ("\"{a + b}\"", "(str (+ a b))"),
    ]);
}

#[test]
fn calls_fields_and_indexing() {
    check_all(&[
        (
            "greet(\"Asha\", greeting: \"Hi\")",
            "(call greet (str \"Asha\") greeting:(str \"Hi\"))",
        ),
        (
            "orders.filter(.status == Paid).sum(.amount)",
            "(call (. (call (. orders filter) (dot-fn (== (. it status) Paid))) sum) (dot-fn (. it amount)))",
        ),
        ("user?.name", "(?. user name)"),
        ("names[0]", "(index names 0)"),
        ("result.0.balance", "(. (. result 0) balance)"),
        (
            "ask<RefundDecision>(prompt)",
            "(call ask<RefundDecision> prompt)",
        ),
        (
            "json.decode<[Rates]>(body)",
            "(call (. json decode)<[Rates]> body)",
        ),
        ("a < b", "(< a b)"),
        (
            "Channel<Map<K, List<V>>>(capacity: 1)",
            "(call Channel<Map<K List<V>>> capacity:1)",
        ),
        (
            "asha with { age: asha.age + 1 }",
            "(with asha age:(+ (. asha age) 1))",
        ),
    ]);
}

#[test]
fn lambdas() {
    check_all(&[
        ("x => x * 2", "(fn (x) (* x 2))"),
        ("(a, b) => a + b", "(fn (a b) (+ a b))"),
        (
            "items.map(n => n + 1)",
            "(call (. items map) (fn (n) (+ n 1)))",
        ),
        ("f(x => {\n  a\n  b\n})", "(call f (fn (x) {a b}))"),
    ]);
}

#[test]
fn control_flow_expressions() {
    check_all(&[
        ("if a { 1 } else { 2 }", "(if a {1} {2})"),
        (
            "if a { 1 } else if b { 2 } else { 3 }",
            "(if a {1} (if b {2} {3}))",
        ),
        ("if let u = find(7) { u }", "(if-let u (call find 7) {u})"),
        (
            "match s {\n  Paid => 1\n  _ => 2\n}",
            "(match s (Paid => 1) (_ => 2))",
        ),
        ("{\n  const x = 21\n  x * 2\n}", "{(const x 21) (* x 2)}"),
    ]);
}

#[test]
fn errors_and_effects() {
    check_all(&[
        ("try load()", "(try (call load))"),
        (
            "try read(p) context \"loading\"",
            "(try (call read p) context (str \"loading\"))",
        ),
        ("parse(x) catch 0", "(catch (call parse x) 0)"),
        (
            "parse(x) catch e => {\n  0\n}",
            "(catch (call parse x) e => {0})",
        ),
        (
            "Int(t) catch _ => fail Bad(text: t)",
            "(catch (call Int t) _ => (fail (call Bad text:t)))",
        ),
        ("spawn fetch(id)", "(spawn (call fetch id))"),
        // How the task runs (spec 9.8): a word only before a name or a
        // block, so `spawn thread()` still calls `thread`.
        ("spawn thread fetch(id)", "(spawn thread (call fetch id))"),
        ("spawn cores fetch(id)", "(spawn cores (call fetch id))"),
        ("spawn thread()", "(spawn (call thread))"),
        ("spawn cores.run()", "(spawn (call (. cores run)))"),
        (
            "spawn process convert(f)",
            "(spawn process (call convert f))",
        ),
        ("spawn process()", "(spawn (call process))"),
        // A time limit (spec 9.6).
        (
            "spawn cores search(b) within 5.seconds",
            "(spawn cores (call search b) within (. 5 seconds))",
        ),
        ("arena { 1 }", "(arena {1})"),
    ]);
}
