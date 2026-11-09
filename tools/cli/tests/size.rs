//! A program pays only for what it uses (decision 0024): "hello world"
//! links none of the runtime's features (TLS, regular expressions, JSON,
//! tasks, agents), so it stays small.

use std::process::Command;

/// The most "hello world" may weigh in this test build. The runtime the
/// tests link is built for development (with debug checks), so this is
/// more than the 300 KB the decision sets for release builds (285 KB on
/// Windows, 2026-11-01); it still fails when a feature creeps back in:
/// TLS alone adds 150 KB, regular expressions 1 MB. Every program now
/// catches Ctrl+C and `SIGTERM` (decision 0063): 16 KB on Linux.
const BUDGET: u64 = 720 * 1024;

#[test]
fn hello_world_stays_small() {
    let dir = std::env::temp_dir().join(format!("kavach-size-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let source = dir.join("hello.kav");
    std::fs::write(&source, "print(\"hi\")\n").unwrap();
    let exe = dir.join(format!("hello{}", std::env::consts::EXE_SUFFIX));
    let built = Command::new(env!("CARGO_BIN_EXE_kavach"))
        .arg("build")
        .arg(&source)
        .arg("-o")
        .arg(&exe)
        .output()
        .unwrap();
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let size = std::fs::metadata(&exe).unwrap().len();
    let bytes = std::fs::read(&exe).unwrap();
    let ran = Command::new(&exe).output().unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(String::from_utf8_lossy(&ran.stdout), "hi\n");
    assert!(
        size <= BUDGET,
        "hello world is {} KB, over the budget of {} KB",
        size / 1024,
        BUDGET / 1024
    );
    // It stands alone: no Visual C++ runtime to install first, which a
    // fresh Windows may lack (the Universal CRT comes with Windows 10).
    let named = |dll: &[u8]| {
        bytes
            .windows(dll.len())
            .any(|w| w.eq_ignore_ascii_case(dll))
    };
    assert!(
        !named(b"vcruntime140.dll"),
        "hello world needs VCRUNTIME140.dll"
    );
}
