//! A project's index (spec 13.9): saved in `.kavach/index`, used while
//! no file changes, rebuilt when one does; names may carry their module.

use kavach_ai::{NoIndex, context, index_for, query};
use std::path::{Path, PathBuf};

fn project(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("kavach-index-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    write(&root, "kavach.toml", "[package]\nname = \"shop\"\n");
    write(
        &root,
        "src/main.kav",
        "use orders\nuse billing\n\nfn main() {\n    print(orders.refund(3))\n    print(billing.refund(4))\n}\n",
    );
    write(
        &root,
        "src/orders.kav",
        "pub type Order {\n    pub id: Int\n}\n\npub fn refund(amount: Int) -> Int {\n    amount * 2\n}\n",
    );
    write(
        &root,
        "src/billing.kav",
        "use orders.{Order}\n\npub fn refund(amount: Int) -> Int {\n    const o = Order(id: amount)\n    o.id\n}\n",
    );
    root
}

fn write(root: &Path, file: &str, text: &str) {
    let path = root.join(file);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn entry(root: &Path) -> String {
    root.join("src/main.kav").to_string_lossy().to_string()
}

#[test]
fn the_index_is_saved_used_and_rebuilt() {
    let root = project("cache");
    let main = entry(&root);
    let callers = |main: &str| query(&index_for(main).unwrap(), "callers", "refund").unwrap();
    let first = callers(&main);
    assert!(
        first.contains("main.kav:5  in main: print(orders.refund(3))"),
        "{first}"
    );
    let saved = root.join(".kavach/index");
    assert!(saved.is_file(), "the first call saves the index");

    // While no file changes, answers come from the saved index.
    let text = std::fs::read_to_string(&saved).unwrap();
    std::fs::write(
        &saved,
        text.replace("print(orders.refund(3))", "from the index"),
    )
    .unwrap();
    assert!(callers(&main).contains("in main: from the index"));

    // A changed file means checking again.
    write(
        &root,
        "src/orders.kav",
        "pub fn refund(amount: Int) -> Int {\n    amount\n}\n",
    );
    write(
        &root,
        "src/billing.kav",
        "pub fn refund(amount: Int) -> Int {\n    amount\n}\n",
    );
    assert_eq!(callers(&main), first);

    // Deleting the index changes no answer.
    std::fs::remove_file(&saved).unwrap();
    assert_eq!(callers(&main), first);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn names_can_carry_their_module() {
    let root = project("names");
    let index = index_for(&entry(&root)).unwrap();
    let orders = context(&index, "orders.refund", 2000).unwrap();
    assert!(orders.contains("    amount * 2"), "{orders}");
    let billing = context(&index, "billing.refund", 2000).unwrap();
    assert!(
        billing.contains("// used by `billing.refund`:\n\npub type Order {"),
        "{billing}"
    );
    let plain = context(&index, "refund", 2000).unwrap_err();
    assert!(
        plain.contains("write billing.refund or orders.refund"),
        "{plain}"
    );
    assert!(
        query(&index, "type", "orders.Order")
            .unwrap()
            .contains("orders.kav:1")
    );
    let callers = query(&index, "callers", "billing.refund").unwrap();
    assert!(
        callers.contains("main.kav:6") && !callers.contains("main.kav:5"),
        "{callers}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_program_with_errors_gets_no_index() {
    let root = project("errors");
    write(
        &root,
        "src/orders.kav",
        "pub fn refund(amount: Int) -> Int {\n    amont\n}\n",
    );
    match index_for(&entry(&root)) {
        Err(NoIndex::Errors(messages)) => assert!(messages.contains("amont"), "{messages}"),
        other => panic!("expected errors, got {other:?}"),
    }
    assert!(!root.join(".kavach/index").exists());
    let _ = std::fs::remove_dir_all(&root);
}
