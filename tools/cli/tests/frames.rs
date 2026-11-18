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

/// A GPU `map` with `KAVACH_TRACE` on: on a GPU that can time its work,
/// the trace gets a row `GPU` with the map's own time (spec 14.12).
const GPU_MAP: &str = "gpu fn step(x: Float32, k: Float32) -> Float32 {
    x * 2.0 + k
}

fn main() {
    let xs: [Float32] = []
    for i in 0..1000 {
        xs.push(Float32(i))
    }
    const k: Float32 = 0.5
    const ys = xs.map(x => step(x, k))
    print(`{ys[0]} {ys[999]}`)
}
";

#[test]
fn the_gpus_work_is_timed_in_the_trace() {
    let dir = std::env::temp_dir().join(format!("kavach-gpu-trace-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("main.kav"), GPU_MAP).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_kavach"))
        .args(["run", "--native", "main.kav"])
        .env("KAVACH_GPU", "on")
        .env("KAVACH_TRACE", dir.join("trace.json"))
        .current_dir(&dir)
        .output()
        .unwrap();
    let trace = std::fs::read_to_string(dir.join("trace.json")).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&dir);
    let said = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "0.5 1998.5\n",
        "{said}"
    );
    // A machine whose GPU can't time its work (or that has none) writes
    // no GPU row; one that can writes the row and the map's event.
    if trace.contains("\"name\":\"GPU\"") {
        let event = trace.lines().find(|l| l.contains("\"name\":\"gpu map\""));
        let event = event.unwrap_or_else(|| panic!("no `gpu map` event: {trace}"));
        assert!(
            event.contains("\"ph\":\"X\"") && event.contains("\"dur\":"),
            "{event}"
        );
    } else {
        eprintln!("this machine's GPU can't time its work: no GPU row to check");
    }
}

/// `kavach build --tracy` (spec 14.12, decision 0123): the program holds
/// Tracy's client and runs as before; built without it, it holds none.
/// Run, it listens for Tracy's viewer on this machine only, and goes on
/// without one.
#[test]
fn only_a_program_built_with_tracy_holds_it() {
    let dir = std::env::temp_dir().join(format!("kavach-tracy-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("main.kav"), PROGRAM).unwrap();
    let exe = |name: &str| dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
    for (name, flag) in [("plain", None), ("traced", Some("--tracy"))] {
        let mut build = Command::new(env!("CARGO_BIN_EXE_kavach"));
        build
            .arg("build")
            .args(flag)
            .args(["main.kav", "-o"])
            .arg(exe(name));
        let built = build.current_dir(&dir).output().unwrap();
        assert!(
            built.status.success(),
            "{}",
            String::from_utf8_lossy(&built.stderr)
        );
    }
    let holds = |name: &str| {
        let bytes = std::fs::read(exe(name)).unwrap();
        bytes.windows(14).any(|w| w == b"Tracy Profiler")
    };
    let (plain, traced) = (holds("plain"), holds("traced"));
    let ran = Command::new(exe("traced")).output().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(!plain, "a program built without `--tracy` holds Tracy");
    assert!(traced, "a program built with `--tracy` holds Tracy");
    let said = String::from_utf8_lossy(&ran.stderr);
    assert_eq!(String::from_utf8_lossy(&ran.stdout), "true\n", "{said}");
    assert!(said.is_empty(), "{said}");
}
