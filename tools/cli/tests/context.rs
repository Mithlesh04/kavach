//! `kavach query` and `kavach context` need no file inside a project, and
//! answer from its index (spec 13.9).

use std::path::Path;
use std::process::{Command, Output};

fn kavach(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_kavach"))
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn query_and_context_work_on_the_project_without_a_file() {
    let base = std::env::temp_dir().join(format!("kavach-context-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    let outside = kavach(&base, &["query", "find", "x"]);
    assert_eq!(outside.status.code(), Some(2), "no file and no project");

    assert!(kavach(&base, &["new", "shop"]).status.success());
    let root = base.join("shop");
    std::fs::write(
        root.join("src/orders.kav"),
        "pub fn refund(amount: Int) -> Int {\n    amount * 2\n}\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/main.kav"),
        "use orders\n\nfn main() {\n    print(orders.refund(3))\n}\n",
    )
    .unwrap();

    let callers = kavach(&root.join("src"), &["query", "callers", "orders.refund"]);
    assert_eq!(
        String::from_utf8_lossy(&callers.stdout),
        "../src/main.kav:4  in main: print(orders.refund(3))\n"
    );
    assert!(root.join(".kavach/index").is_file());

    let context = kavach(&root, &["context", "refund", "--budget", "100"]);
    let text = String::from_utf8_lossy(&context.stdout);
    assert!(
        text.starts_with("// context for `refund` in src/main.kav"),
        "{text}"
    );
    assert!(text.contains("    amount * 2"), "{text}");
    let _ = std::fs::remove_dir_all(&base);
}
