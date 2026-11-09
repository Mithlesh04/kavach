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
