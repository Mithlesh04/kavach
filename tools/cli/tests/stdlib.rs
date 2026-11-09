//! The standard library modules written in Kavach (`stdlib/`) pass
//! `kavach test`: their examples, and tests generated from their types and
//! contracts (spec 11), like any other Kavach code.

use std::path::Path;
use std::process::Command;

#[test]
fn standard_library_modules_pass_their_own_tests() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../stdlib");
    let mut checked = 0;
    for module in std::fs::read_dir(&root).unwrap().flatten() {
        let Ok(files) = std::fs::read_dir(module.path()) else {
            continue;
        };
        for file in files.flatten().map(|f| f.path()) {
            if file.extension().is_none_or(|e| e != "kav") {
                continue;
            }
            let out = Command::new(env!("CARGO_BIN_EXE_kavach"))
                .args(["test", &file.to_string_lossy(), "--seed", "7"])
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
    }
    assert!(checked >= 4, "only {checked} modules");
}
