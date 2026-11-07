//! `kavach test --keep` (spec 11.4): a found case that now passes becomes
//! an `example` on its function and leaves the found file; one that still
//! fails stays.

use std::process::Command;

#[test]
fn passing_found_cases_become_examples() {
    let dir = std::env::temp_dir().join(format!("kavach-keep-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("tests/found")).unwrap();
    let file = dir.join("bank.kav");
    std::fs::write(
        &file,
        "type BankError = Insufficient(needed: Int)\n\n\
         fn half(n: Int) -> Int {\n    n / 2\n}\n\n\
         fn withdraw(balance: Int, amount: Int) -> Int ! BankError\n    ensure result >= 0\n{\n    \
         if amount > balance + 1 {\n        fail BankError.Insufficient(needed: amount - balance)\n    }\n    \
         balance - amount\n}\n",
    )
    .unwrap();
    let found = dir.join("tests/found/bank.kavf");
    std::fs::write(
        &found,
        "// Found by `kavach test`.\nhalf(4)\nwithdraw(0, 1)\nwithdraw(1, 5)\n",
    )
    .unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_kavach"))
        .current_dir(&dir)
        .args(["test", "bank.kav", "--keep"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let src = std::fs::read_to_string(&file).unwrap();
    assert!(
        src.contains("fn half(n: Int) -> Int\n    example half(4) == 2\n{"),
        "{src}"
    );
    assert!(
        src.contains("    example withdraw(1, 5) fails BankError.Insufficient(needed: 4)\n{"),
        "{src}"
    );
    // `withdraw(0, 1)` still breaks the `ensure`, so it stays found.
    let left = std::fs::read_to_string(&found).unwrap();
    assert!(
        left.contains("withdraw(0, 1)") && !left.contains("half(4)"),
        "{left}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
