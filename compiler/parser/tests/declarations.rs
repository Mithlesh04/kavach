// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Statements and declarations: source and the tree it must produce.

mod common;
use common::check_all;

#[test]
fn statements() {
    check_all(&[
        (
            "const x = 1\nlet y: Int = 2\ny += x",
            "(const x 1)\n(let y:Int 2)\n(+= y x)",
        ),
        (
            "const (code, message) = pair",
            "(const (tuple code message) pair)",
        ),
        ("const _ = add(1, 2)", "(const _ (call add 1 2))"),
        (
            "const [a, b = 0, _, ...rest] = xs",
            "(const (list a b=0 _ ...rest) xs)",
        ),
        (
            "const [\"asha\": a, host: server, port = 8080] = m",
            "(const (list (str \"asha\"):a host:server port=8080) m)",
        ),
        (
            "let { name, age: years, city = \"?\" } = user",
            "(let (record name:name age:years city:city=(str \"?\")) user)",
        ),
        ("user.age = 32", "(= (. user age) 32)"),
        (
            "for (i, n) in names.enumerate() {\n  print(n)\n}",
            "(for (tuple i n) (call (. names enumerate)) {(call print n)})",
        ),
        ("while n < 3 {\n  n += 1\n}", "(while (< n 3) {(+= n 1)})"),
        // Where a loop runs (spec 4.9): a label, then `on`.
        (
            "on cores for i in 0..n {\n  out[i] = i\n}",
            "(on cores) (for i (.. 0 n) {(= (index out i) i)})",
        ),
        (
            "rows: on threads(8) for p in paths {\n  out[0] = p\n}",
            "rows:(on threads 8) (for p paths {(= (index out 0) p)})",
        ),
        (
            "assert total(list) == 100",
            "(assert (== (call total list) 100))",
        ),
    ]);
}

#[test]
fn patterns() {
    check_all(&[(
        "match x {\n  0 => a\n  1..=9 => b\n  (0, y) => c\n  Circle(radius) => d\n  Rect(width: w, height: _) if w > 10 => e\n  Shape.Point => f\n  none => g\n  ok(v) => h\n  err(NotANumber(text)) => i\n  -1 => j\n}",
        "(match x (0 => a) ((..= 1 9) => b) ((tuple 0 y) => c) ((Circle radius) => d) ((Rect width:w height:_) if (> w 10) => e) (Shape.Point => f) (none => g) ((ok v) => h) ((err (NotANumber text)) => i) ((- 1) => j))",
    )]);
}

#[test]
fn functions_and_signatures() {
    check_all(&[
        (
            "fn add(a: Int, b: Int) -> Int {\n  a + b\n}",
            "(fn add (a:Int b:Int) -> Int {(+ a b)})",
        ),
        (
            "fn greet(name: String, greeting: String = \"Hello\") {}",
            "(fn greet (name:String greeting:String=(str \"Hello\")) {})",
        ),
        (
            "fn first<T>(items: [T]) -> T? {}",
            "(fn first <T> (items:[T]) -> T? {})",
        ),
        // Numbers in types (spec 2.9): types positional, numbers named.
        (
            "fn dot<n: Int, k: Int = 2>(a: [Float]<len: n>, b: List<Float, max: 16>) -> {String: Int}<max: 100> {}",
            "(fn dot <n:Int k:Int=2> (a:[Float]<len:n> b:List<Float max:16>) -> {String: Int}<max:100> {})",
        ),
        (
            "fn key(g: Grid<Float, width: 4, height: 3>, b: Bytes<len: 32>, r: Ring<Float, max: 100>?) {}",
            "(fn key (g:Grid<Float width:4 height:3> b:Bytes<len:32> r:Ring<Float max:100>?) {})",
        ),
        (
            "fn bad(a: Grid<Float, 2, 3>) {}",
            "(fn bad (a:Grid<Float (unnamed 2) (unnamed 3)>) {})",
        ),
        (
            "pub fn withdraw(a: Account, amount: Decimal) -> Account ! BankError uses Payments\n    require amount > 0\n    ensure result.balance == a.balance - amount\n    example withdraw(acct, 10) fails Low(needed: 5)\n{\n}",
            "(pub fn withdraw (a:Account amount:Decimal) -> Account ! BankError uses Payments (require (> amount 0)) (ensure (== (. result balance) (- (. a balance) amount))) (example (call withdraw acct 10) fails (call Low needed:5)) {})",
        ),
        (
            "fn transfer(a: Int)   // name\n    -> (Int, Int)\n{\n}",
            "(fn transfer (a:Int) -> (Int Int) {})",
        ),
        (
            "fn op() -> fn(Int, Int) -> Int {}",
            "(fn op () -> fn(Int Int) -> Int {})",
        ),
        (
            "fn route<P, R, E>(f: fn(P) -> R ! E uses Db, n: Int) -> R ! E {}",
            "(fn route <P R E> (f:fn(P) -> R ! E uses (Db) n:Int) -> R ! E {})",
        ),
        (
            "fn run(f: fn() ! Oops uses (Net, Fs.read)) {}",
            "(fn run (f:fn() ! Oops uses (Net Fs.read)) {})",
        ),
        (
            "fn main() ! Error {\n  print(1)\n}",
            "(fn main () ! Error {(call print 1)})",
        ),
    ]);
}

#[test]
fn types() {
    check_all(&[
        (
            "type Status = Pending | Paid | Refunded",
            "(type Status = Pending Paid Refunded)",
        ),
        (
            "type Shape = Circle(radius: Float) | Rect(w: Float, h: Float)",
            "(type Shape = (Circle radius:Float) (Rect w:Float h:Float))",
        ),
        (
            "type Account {\n  owner: String\n  balance: Decimal = 0\n  secret pin: String\n\n  invariant balance >= 0\n\n  fn empty(self) -> Bool { self.balance == 0 }\n}",
            "(type Account {owner:String balance:Decimal=0 secret pin:String (invariant (>= balance 0)) (fn empty (self) -> Bool {(== (. self balance) 0)})})",
        ),
        (
            "type Pair<A, B> {\n  left: A\n  right: B\n}",
            "(type Pair {left:A right:B})",
        ),
        (
            "type FakePayments: Payments {\n  fn refund(self) {}\n}",
            "(type FakePayments : Payments {(fn refund (self) {})})",
        ),
        (
            "interface Describe {\n  fn describe(self) -> String\n}",
            "(interface Describe (fn describe (self) -> String))",
        ),
        (
            "extend String {\n  fn shout(self) -> String { self }\n}",
            "(extend String (fn shout (self) -> String {self}))",
        ),
    ]);
}

#[test]
fn modules_and_tests() {
    check_all(&[
        ("use orders", "(use orders)"),
        (
            "use billing.invoice.{Invoice, create}",
            "(use billing.invoice {Invoice create})",
        ),
        (
            "use std.collections.Set as HashSet",
            "(use std.collections.Set as HashSet)",
        ),
        (
            "test \"adds\" {\n  assert 1 + 1 == 2\n}",
            "(test (str \"adds\") {(assert (== (+ 1 1) 2))})",
        ),
    ]);
}

#[test]
fn capabilities_and_agents() {
    check_all(&[
        (
            "capability Email {\n  fn send(to: String) ! EmailError\n}",
            "(capability Email (fn send (to:String) ! EmailError))",
        ),
        (
            "agent Bot {\n  can Ai, Payments.refund\n  limit Payments.refund {\n    amount <= 10_000\n    sum(amount) <= 50_000 per day\n  }\n  approve Payments.refund when amount > 5_000 by \"finance\"\n  on request(order: Order) ! Error {\n    print(order)\n  }\n}",
            "(agent Bot (can Ai Payments.refund) (limit Payments.refund (<= amount 10000) ((<= (call sum amount) 50000) per day)) (approve Payments.refund (> amount 5000) (str \"finance\")) (on request (order:Order) ! Error {(call print order)}))",
        ),
        // A request's deadline (decision 0072).
        (
            "agent Bot {\n  approve Payments.refund when amount > 5_000 by \"finance\" within 2.days\n}",
            "(agent Bot (approve Payments.refund (> amount 5000) (str \"finance\") (within (. 2 days))))",
        ),
    ]);
}

/// `pub(out)` (spec 10.1): offered to other packages, and `pub` too.
#[test]
fn pub_out_marks_what_a_library_offers() {
    let parsed = kavach_parser::parse("pub(out) fn a() {\n}\npub fn b() {\n}\nfn c() {\n}\n");
    assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
    let marks: Vec<(bool, bool)> = (parsed.file.items.iter())
        .map(|i| (i.is_pub, i.is_out))
        .collect();
    assert_eq!(marks, [(true, true), (true, false), (false, false)]);
}

/// `pub(read)` before a field (spec 10.1): read by other modules too.
#[test]
fn pub_read_marks_a_field_others_only_read() {
    let parsed =
        kavach_parser::parse("type A {\n    pub(read) n: Int\n    pub m: Int\n    k: Int\n}\n");
    assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
    let kavach_ast::ItemKind::Type(t) = &parsed.file.items[0].kind else {
        panic!("a type");
    };
    let kavach_ast::TypeBody::Record(r) = &t.body else {
        panic!("a record");
    };
    let marks: Vec<(bool, bool)> = r.fields.iter().map(|f| (f.is_pub, f.is_read)).collect();
    assert_eq!(marks, [(true, true), (true, false), (false, false)]);
}

/// A tag with `param`, and a tagged template whose `{…}` are its
/// parameters (spec 10.15).
#[test]
fn tags_and_tagged_templates() {
    check_all(&[
        (
            "literal sql: Query check check_sql param pg_param language \"sql\"",
            "(literal sql Query check check_sql param pg_param language \"sql\")",
        ),
        (
            "const q = sql`a = {x} and b = {y}`",
            "(const q (tagged sql (str \"a = \" x \" and b = \" y)))",
        ),
    ]);
}
