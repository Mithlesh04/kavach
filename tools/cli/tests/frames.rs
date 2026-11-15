//! Frame timing (spec 14.4, 14.12): `time.monotonic_ns` and a precise
//! `time.sleep_ns`, and spans written as a Chrome trace with
//! `KAVACH_TRACE`, interpreted and in native code. Run in a folder of its
//! own in the system's temporary folder.

use std::process::Command;

const PROGRAM: &str = "use std.time
use std.log

fn main() uses Clock {
    const before = time.monotonic_ns()
    log.span(\"frame\") {
        time.sleep_ns(3_000_000)
    }
    const took = time.monotonic_ns() - before
    print(took >= 3_000_000 and took < 200_000_000)
}
";

#[test]
fn a_frame_is_timed_and_written_as_a_chrome_trace() {
    for native in [false, true] {
        let dir =
            std::env::temp_dir().join(format!("kavach-frames-{native}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("main.kav"), PROGRAM).unwrap();
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_kavach"));
        cmd.arg("run");
        if native {
            cmd.arg("--native");
        }
        let out = cmd
            .arg("main.kav")
            .env("KAVACH_TRACE", dir.join("trace.json"))
            .current_dir(&dir)
            .output()
            .unwrap();
        let trace = std::fs::read_to_string(dir.join("trace.json")).unwrap_or_default();
        let _ = std::fs::remove_dir_all(&dir);
        let text = String::from_utf8_lossy(&out.stdout);
        assert_eq!(
            text,
            "true\n",
            "native: {native}; {}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            trace.starts_with("[\n{\"name\":\"frame\",\"ph\":\"X\""),
            "native: {native}; {trace}"
        );
        let dur: u64 = (trace.split("\"dur\":").nth(1))
            .and_then(|r| r.split(',').next())
            .and_then(|d| d.parse().ok())
            .unwrap_or(0);
        assert!(dur >= 3_000, "the span took at least 3 ms: {dur} µs");
    }
}
