//! How each task runs, chosen where it is spawned (spec 9.8, decision
//! 0069): `spawn`, `spawn thread` and `spawn cores` print the same, alone
//! or mixed in one program; `tasks =` in `kavach.toml` says where the
//! choice went; `http.serve`'s handlers run on every core (spec 14.11).

use std::path::Path;
use std::process::Command;

const PROGRAM: &str = "use std.time

fn report(name: String, ms: Int) -> String {
    time.sleep_ms(ms)
    print(\"{name} done\")
    name
}

fn main() ! Error {
    const (send, receive) = Channel<Int>(capacity: 4)
    const producer = spawn PRODUCER {
        for i in 0..10 {
            send.put(i)
        }
        send.close()
    }
    let total = 0
    for n in receive {
        total += n
    }
    const slow = spawn SLOW report(\"slow\", 60)
    const fast = spawn FAST report(\"fast\", 20)
    print(\"both started, total {total}\")
    print(\"{try slow.wait()} and {try fast.wait()} are back\")
}
";

const PRINTS: &str = "both started, total 45\nfast done\nslow done\nslow and fast are back\n";

/// The program with each task's kind word (or none) in place.
fn program(producer: &str, slow: &str, fast: &str) -> String {
    let word = |w: &str| {
        if w.is_empty() {
            String::new()
        } else {
            format!("{w} ")
        }
    };
    PROGRAM
        .replace("PRODUCER ", &word(producer))
        .replace("SLOW ", &word(slow))
        .replace("FAST ", &word(fast))
}

/// Builds `program` in a project whose `kavach.toml` is `toml`, runs
/// it, and gives its output; `Err` with the error when it doesn't build.
fn build_and_run(name: &str, toml: &str, program: &str) -> Result<String, String> {
    let dir = std::env::temp_dir().join(format!("kavach-tasks-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("kavach.toml"), toml).unwrap();
    let main = dir.join("src").join("main.kav");
    std::fs::write(&main, program).unwrap();
    let exe = dir.join(format!("main{}", std::env::consts::EXE_SUFFIX));
    let built = Command::new(env!("CARGO_BIN_EXE_kavach"))
        .args(["build", main.to_str().unwrap(), "-o", exe.to_str().unwrap()])
        .output()
        .unwrap();
    let result = if built.status.success() {
        let ran = Command::new(Path::new(&exe)).output().unwrap();
        Ok(String::from_utf8_lossy(&ran.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&built.stderr).to_string())
    };
    let _ = std::fs::remove_dir_all(&dir);
    result
}

#[test]
fn every_kind_prints_the_same() {
    let toml = "[package]
name = \"t\"
";
    for (name, kinds) in [
        ("coroutines", ["", "", ""]),
        ("threads", ["thread", "thread", "thread"]),
        ("cores", ["cores", "cores", "cores"]),
        ("mixed", ["cores", "thread", ""]),
    ] {
        let got = build_and_run(name, toml, &program(kinds[0], kinds[1], kinds[2]));
        assert_eq!(got.as_deref(), Ok(PRINTS), "{name}");
    }
}

#[test]
fn the_kavach_toml_setting_moved_into_the_code() {
    let toml = "[package]
name = \"t\"
tasks = \"threads\"
";
    let err = build_and_run("moved", toml, &program("", "", ""));
    let err = err.expect_err("`tasks =` is no setting any more");
    assert!(
        err.contains("kavach.toml") && err.contains("spawn thread"),
        "{err}"
    );
}

#[test]
fn http_handlers_on_every_core_answer_right() {
    let toml = "[package]
name = \"t\"
";
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../tests/conformance/14-standard-library/http-serves-many-clients-on-every-core.kav",
    );
    let program = std::fs::read_to_string(path).unwrap();
    let got = build_and_run("http", toml, &program);
    assert_eq!(
        got.as_deref(),
        Ok("answers: 80
right: 80
counted: 80
"),
        "built with workers on every core"
    );
}
