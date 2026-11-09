//! The AI tools give exact, compact answers (spec chapter 13).

use kavach_ai::{Index, messages_json, outline, query};
use kavach_driver::{Compiled, Options, compile};

const SHOP: &str = r#"type Status = Pending | Paid

type Order {
    id: Int
    amount: Decimal
    status: Status = Status.Pending
}

type ShopError = Missing | Declined | Denied(reason: String)

capability Payments {
    fn charge(order_id: Int, amount: Decimal) ! ShopError
}

/// Total of all paid orders.
pub fn total_paid(orders: [Order]) -> Decimal
    example total_paid([]) == 0
{
    let total: Decimal = 0
    for order in orders {
        if order.status == Status.Paid {
            total += order.amount
        }
    }
    total
}

fn pay(order: Order) -> Order ! ShopError uses Payments {
    try Payments.charge(order.id, order.amount)
    order with { status: Status.Paid }
}

fn report(orders: [Order]) -> String {
    "paid: {total_paid(orders)}"
}
"#;

fn shop() -> Compiled {
    let c = compile(SHOP, "shop.kav", Options::default());
    assert!(!c.has_errors(), "{}", c.render());
    c
}

#[test]
fn outline_keeps_signatures_and_drops_bodies() {
    let text = outline(&shop());
    assert!(text.contains("/// Total of all paid orders.\npub fn total_paid(orders: [Order]) -> Decimal\n    example total_paid([]) == 0\n{ … }"));
    assert!(text.contains("fn pay(order: Order) -> Order ! ShopError uses Payments { … }"));
    assert!(text.contains("    fn charge(order_id: Int, amount: Decimal) ! ShopError"));
    assert!(
        !text.contains("total += order.amount"),
        "bodies must be left out"
    );
    assert!(
        text.len() * 100 / SHOP.len() < 70,
        "outline is {} of {} bytes",
        text.len(),
        SHOP.len()
    );
}

#[test]
fn fails_follows_try_into_any_error() {
    // `checkout` says `! Error`, but passes a `ShopError` on with `try`:
    // it can fail with one (spec 13.2). `tidy` handles its own.
    let src = format!(
        "{SHOP}
fn checkout(order: Order) -> Order ! Error uses Payments {{
    try pay(order)
}}

fn tidy(order: Order) -> Order uses Payments {{
    pay(order) catch _ => order
}}
"
    );
    let c = compile(&src, "shop.kav", Options::default());
    assert!(!c.has_errors(), "{}", c.render());
    let answer = query(&Index::build(&c, "shop.kav"), "fails", "ShopError").unwrap();
    assert!(
        answer.contains("fn checkout(order: Order) -> Order ! Error"),
        "{answer}"
    );
    assert!(!answer.contains("fn tidy"), "{answer}");
}

#[test]
fn queries_answer_exactly() {
    let c = Index::build(&shop(), "shop.kav");
    let ask = |kind: &str, name: &str| query(&c, kind, name).unwrap();
    assert_eq!(
        ask("callers", "total_paid"),
        "shop.kav:17  in total_paid: example total_paid([]) == 0\nshop.kav:34  in report: \"paid: {total_paid(orders)}\"\n"
    );
    assert_eq!(
        ask("uses", "Payments"),
        "shop.kav:28  fn pay(order: Order) -> Order ! ShopError uses Payments\n"
    );
    assert_eq!(
        ask("fails", "ShopError"),
        "shop.kav:12  fn charge(order_id: Int, amount: Decimal) ! ShopError\nshop.kav:28  fn pay(order: Order) -> Order ! ShopError uses Payments\n"
    );
    assert_eq!(
        ask("find", "pa"),
        "shop.kav:1  variant Paid\nshop.kav:11  capability Payments\nshop.kav:16  fn total_paid\nshop.kav:28  fn pay\n"
    );
    assert!(ask("type", "Order").starts_with("shop.kav:3\ntype Order {\n    id: Int"));
    assert_eq!(ask("uses", "Net"), "no results for `uses Net`\n");
    assert!(
        query(&c, "callers", "nothing")
            .unwrap_err()
            .contains("not declared")
    );
    assert!(
        query(&c, "everything", "x")
            .unwrap_err()
            .contains("unknown query")
    );
}

#[test]
fn messages_are_one_json_line_each() {
    let c = compile(
        "fn main() {\n  const total = 1\n  print(\"{totl}\")\n}",
        "a.kav",
        Options::default(),
    );
    assert_eq!(
        messages_json(&c),
        "{\"file\":\"a.kav\",\"line\":3,\"col\":11,\"severity\":\"error\",\"message\":\"`totl` is not defined; did you mean `total`?\"}\n"
    );
}

#[test]
fn context_gives_the_code_and_what_it_uses() {
    let c = Index::build(&shop(), "shop.kav");
    let text = kavach_ai::context(&c, "pay", 2000).unwrap();
    assert!(text.starts_with("// context for `pay` in shop.kav (~"));
    assert!(
        text.contains("    try Payments.charge(order.id, order.amount)"),
        "the target's full code"
    );
    let order = text.find("type Order {").expect("Order");
    let error = text.find("type ShopError").expect("ShopError");
    let payments = text.find("capability Payments").expect("Payments");
    let status = text
        .find("type Status")
        .expect("Status, for the `Paid` variant");
    assert!(
        order < error && error < payments && payments < status,
        "in order of first use"
    );
    assert!(!text.contains("fn total_paid"), "only what `pay` uses");

    let small = kavach_ai::context(&c, "pay", 60).unwrap();
    assert!(small.contains("// left out to fit 60 tokens: "), "{small}");
    assert!(kavach_ai::context(&c, "nope", 100).is_err());
}

#[test]
fn uses_counts_capabilities_inferred_and_main() {
    // `main` may use anything, but `query uses` still lists what it does:
    // here `Console`, through `greet` (spec 13.2).
    let src = "fn greet(name: String) {\n    print(\"hi {name}\")\n}\n\nfn main() {\n    greet(\"Asha\")\n}\n";
    let c = compile(src, "main.kav", Options::default());
    assert!(!c.has_errors(), "{}", c.render());
    let answer = query(&Index::build(&c, "main.kav"), "uses", "Console").unwrap();
    assert_eq!(
        answer,
        "main.kav:1  fn greet(name: String)\nmain.kav:5  fn main()\n"
    );
}
