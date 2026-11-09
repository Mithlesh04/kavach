//! `kavach proto` (spec 14.14) writes the `proto3` file for a program's
//! types, with the wire conventions `std.grpc` uses.

use std::fs;
use std::process::Command;

#[test]
fn proto_writes_messages_and_oneof_wrappers() {
    let dir = std::env::temp_dir().join(format!("kavach-proto-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("shop.kav");
    fs::write(
        &path,
        r#"type Status = Pending | Paid(amount: Float, by: String)

type Order {
    id: Int
    tags: [Int]
    note: String?
    status: Status
}

fn main() {
    print(Order(id: 1, tags: [], note: none, status: Status.Pending).id)
}
"#,
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_kavach"))
        .arg("proto")
        .arg(&path)
        .output()
        .unwrap();
    let _ = fs::remove_dir_all(&dir);
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    for needle in [
        "syntax = \"proto3\";",
        "message Order {",
        "int64 id = 1;",
        "repeated int64 tags = 2;",
        "optional string note = 3;",
        "Status status = 4;",
        "message Status {",
        "oneof value {",
        "Pending pending = 1;",
        "Paid paid = 2;",
        "message Pending {}",
        "double amount = 1;",
        "string by = 2;",
    ] {
        assert!(text.contains(needle), "missing {needle:?} in:\n{text}");
    }
}
