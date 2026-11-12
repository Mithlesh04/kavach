use super::format;

/// (input, formatted) pairs.
const CASES: &[(&str, &str)] = &[
    // Indentation from braces, trailing spaces, final newline.
    (
        "fn main() {\nprint(1)   \n        print(2)\n}",
        "fn main() {\n    print(1)\n    print(2)\n}\n",
    ),
    // Blank lines: at most one, none just inside braces.
    (
        "fn a() {\n\n    x()\n\n\n\n    y()\n\n}\n\n\n\nfn b() {\n}\n",
        "fn a() {\n    x()\n\n    y()\n}\n\nfn b() {\n}\n",
    ),
    // Continued lines get one more level; closers line up with openers.
    (
        "const total = 1 +\n2 +\n3\nconst n = items\n.filter(.paid)\n.len()\nconst xs = [\n1,\n2,\n]\n",
        "const total = 1 +\n    2 +\n    3\nconst n = items\n    .filter(.paid)\n    .len()\nconst xs = [\n    1,\n    2,\n]\n",
    ),
    // Comments stay where they are, indented like code.
    (
        "fn a() {\n// note\n  x() // why\n}\n",
        "fn a() {\n    // note\n    x() // why\n}\n",
    ),
    // `\r\n` becomes `\n`.
    ("fn a() {\r\nx()\r\n}\r\n", "fn a() {\n    x()\n}\n"),
    // A line ending in a type's `>` doesn't continue; one ending in `a >` does.
    (
        "type Load {
live: Shared<Int>
stop: Int
}
const big = a >
b
",
        "type Load {
    live: Shared<Int>
    stop: Int
}
const big = a >
    b
",
    ),
    // A multi-line string is left exactly as it is.
    (
        "fn a() {\nconst s = \"\"\"\n   keep   \n  this\n   \"\"\"\n}\n",
        "fn a() {\n    const s = \"\"\"\n   keep   \n  this\n   \"\"\"\n}\n",
    ),
];

#[test]
fn formats_layout_only() {
    for (input, want) in CASES {
        assert_eq!(format(input).as_deref(), Ok(*want), "input: {input:?}");
    }
}

#[test]
fn formatting_twice_changes_nothing() {
    for (input, _) in CASES {
        let once = format(input).unwrap();
        assert_eq!(format(&once).unwrap(), once, "input: {input:?}");
    }
}

#[test]
fn every_kavach_file_in_the_repository_is_formatted() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    for dir in ["examples", "tests/conformance"] {
        collect(&root.join(dir), &mut files);
    }
    assert!(files.len() > 100);
    for f in files {
        let src = std::fs::read_to_string(&f).unwrap();
        if kavach_lexer::lex(&src).errors.is_empty() {
            // The repository follows its own formatter.
            let formatted = format(&src).unwrap_or_else(|e| panic!("{}: {e}", f.display()));
            assert_eq!(
                formatted,
                src,
                "{} is not formatted: run `kavach fmt` on it",
                f.display()
            );
        }
    }
}

fn collect(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            collect(&p, out);
        } else if p.extension().is_some_and(|x| x == "kav") {
            out.push(p);
        }
    }
}

/// Names in the wrong style are renamed with every use: variables,
/// parameters (and named arguments), functions, types, fields and
/// methods (through the values they are read from).
#[test]
fn names_are_fixed_with_their_uses() {
    let src = "type order_status = Open | Closed\n\ntype Order {\n    orderId: Int\n    status: order_status\n\n    fn isOpen(self) -> Bool {\n        self.orderId > 0\n    }\n}\n\nfn makeOrder(orderId: Int) -> Order {\n    Order(orderId: orderId, status: Open)\n}\n\nfn main() {\n    const firstOrder = makeOrder(orderId: 7)\n    const copy = firstOrder with { orderId: 8 }\n    print(\"{copy.orderId} {firstOrder.isOpen()}\")\n}\n";
    let want = "type OrderStatus = Open | Closed\n\ntype Order {\n    order_id: Int\n    status: OrderStatus\n\n    fn is_open(self) -> Bool {\n        self.order_id > 0\n    }\n}\n\nfn make_order(order_id: Int) -> Order {\n    Order(order_id: order_id, status: Open)\n}\n\nfn main() {\n    const first_order = make_order(order_id: 7)\n    const copy = first_order with { order_id: 8 }\n    print(\"{copy.order_id} {first_order.is_open()}\")\n}\n";
    let (out, left) = super::fix_names(src, "t.kav");
    assert_eq!(out, want);
    assert!(left.is_empty(), "{left:?}");
}

/// A `pub` name, and a rename whose new name is taken, are left as they
/// are and reported; the other renames still happen.
#[test]
fn names_that_cant_change_are_reported() {
    let src = "pub fn sharedName() -> Int {\n    1\n}\n\nfn main() {\n    const my_total = 1\n    const myTotal = 2\n    const otherOne = 3\n    print(my_total + myTotal + otherOne + sharedName())\n}\n";
    let (out, left) = super::fix_names(src, "t.kav");
    assert!(out.contains("const other_one = 3") && out.contains("myTotal + other_one"));
    assert!(out.contains("const myTotal = 2") && out.contains("fn sharedName"));
    assert_eq!(left.len(), 2, "{left:?}");
}
