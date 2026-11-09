//! Every program in `tests/integration/` passes `kavach test`: its
//! `test` blocks, and the tests generated from its types and contracts.

use std::path::Path;
use std::process::Command;

#[test]
fn integration_programs_pass_their_tests() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/integration");
    let mut checked = 0;
    for file in std::fs::read_dir(&root)
        .unwrap()
        .flatten()
        .map(|f| f.path())
    {
        if file.extension().is_none_or(|e| e != "kav") {
            continue;
        }
        let out = Command::new(env!("CARGO_BIN_EXE_kavach"))
            .args(["test", &file.to_string_lossy(), "--seed", "7"])
            .env_remove("ANTHROPIC_API_KEY")
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}:\n{}{}",
            file.display(),
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        checked += 1;
    }
    assert!(checked >= 1, "found no integration programs");
}
