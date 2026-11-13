//! The conformance runner (tests/conformance/README.md): checks, runs or
//! tests every conformance file and compares the result with its header.
//! Also runs the examples, which must succeed.

use kavach_driver::{Options, compile};
use std::fs;
use std::path::{Path, PathBuf};

fn kav_files(dir: &Path, out: &mut Vec<PathBuf>) {
    // A project folder (with `kavach.toml` and `src/`) is one test: its
    // `src/main.kav`, compiled with the other files of `src/`.
    if dir.join("kavach.toml").is_file() && dir.join("src").is_dir() {
        out.push(dir.join("src").join("main.kav"));
        return;
    }
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            kav_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "kav") {
            out.push(path);
        }
    }
}

/// What a test's header says should happen.
#[derive(Default)]
struct Expect {
    output: Vec<String>,
    exit: Option<i32>,
    compile_error: bool,
    warning: bool,
    panic: Option<String>,
    stderr: Vec<String>,
    /// `// expect no stderr`: standard error must be empty.
    no_stderr: bool,
    test: Option<bool>,
    args: Vec<String>,
}

fn expectations(src: &str) -> Expect {
    let mut e = Expect::default();
    for line in src.lines() {
        if let Some(text) = line.strip_prefix("// expect output: ") {
            e.output.push(text.to_string());
        } else if let Some(code) = line.strip_prefix("// expect exit: ") {
            e.exit = code.trim().parse().ok();
        } else if line == "// expect compile error" {
            e.compile_error = true;
        } else if line == "// expect warning" {
            e.warning = true;
        } else if line == "// expect panic" {
            e.panic = Some(String::new());
        } else if let Some(text) = line.strip_prefix("// expect panic: ") {
            e.panic = Some(text.to_string());
        } else if let Some(text) = line.strip_prefix("// expect stderr: ") {
            e.stderr.push(text.to_string());
        } else if line == "// expect no stderr" {
            e.no_stderr = true;
        } else if line == "// expect test pass" || line == "// expect test fail" {
            e.test = Some(line.ends_with("pass"));
        } else if let Some(words) = line.strip_prefix("// args: ") {
            e.args = words.split_whitespace().map(String::from).collect();
        }
    }
    e
}

/// Runs one file and returns what went wrong, if anything.
fn check_file(path: &Path) -> Option<String> {
    let src = fs::read_to_string(path).unwrap();
    let e = expectations(&src);
    // A folder can set limits in its own kavach.toml (chapter 12).
    let options = kavach_driver::Config::for_file(path)
        .expect("a valid kavach.toml")
        .options_for(path);
    let mut compiled = match kavach_driver::project_sources(path) {
        Some(sources) => kavach_driver::compile_sources(sources, options),
        None => compile(&src, &path.to_string_lossy(), options),
    };
    // Tagged literals, checked by their tags' checkers, as `kavach` does
    // (spec 10.15).
    if !compiled.has_errors() {
        let found = kavach_interp::check_literals(&compiled);
        compiled.messages.extend(found);
    }
    if e.compile_error {
        return (!compiled.has_errors())
            .then(|| "expected a compile error, but it compiled".to_string());
    }
    if compiled.has_errors() {
        return Some(format!("unexpected compile errors:\n{}", compiled.render()));
    }
    if e.warning
        && let Some(wrong) = warnings_differ(&src, &compiled)
    {
        return Some(wrong);
    }
    if let Some(pass) = e.test {
        let out = kavach_interp::test(&compiled, &Default::default());
        let ok = (out.exit_code == 0) == pass;
        return (!ok).then(|| {
            format!(
                "expected tests to {}:\n{}",
                if pass { "pass" } else { "fail" },
                out.stdout
            )
        });
    }
    let out = interpret(&compiled, path, &src, &e.args);
    let mut problems = Vec::new();
    let got: Vec<&str> = out.stdout.lines().collect();
    if got != e.output {
        problems.push(format!("output\n  want: {:?}\n   got: {:?}", e.output, got));
    }
    let want_exit = e.exit.unwrap_or(if e.panic.is_some() { 101 } else { 0 });
    if out.exit_code != want_exit {
        problems.push(format!(
            "exit code: want {want_exit}, got {} (stderr: {:?})",
            out.exit_code, out.stderr
        ));
    }
    if let Some(text) = &e.panic
        && (!out.stderr.contains("panic:") || !out.stderr.contains(text.as_str()))
    {
        problems.push(format!(
            "expected a panic containing {text:?}, stderr was {:?}",
            out.stderr
        ));
    }
    // A place in the test file shows as its name alone: `name.kav:9`.
    let shown = kavach_driver::display_path(&path.to_string_lossy());
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let stderr = out
        .stderr
        .replace(&format!("{shown}:"), &format!("{name}:"));
    let got: Vec<&str> = stderr.lines().collect();
    if (e.no_stderr || !e.stderr.is_empty()) && got != e.stderr {
        problems.push(format!("stderr\n  want: {:?}\n   got: {:?}", e.stderr, got));
    }
    (!problems.is_empty()).then(|| problems.join("\n"))
}

/// With `// expect warning`: the lines that get a warning must be those
/// that end with `// warning: <why>`.
fn warnings_differ(src: &str, compiled: &kavach_driver::Compiled) -> Option<String> {
    let want: Vec<usize> = src
        .lines()
        .enumerate()
        .filter(|(_, l)| l.contains("// warning:"))
        .map(|(i, _)| i + 1)
        .collect();
    let mut got: Vec<usize> = compiled
        .messages
        .iter()
        .filter(|m| m.severity == kavach_driver::Severity::Warning)
        .map(|m| src[..m.span.start as usize].matches('\n').count() + 1)
        .collect();
    got.dedup();
    (got != want).then(|| {
        format!(
            "warnings on lines {got:?}, want {want:?}:\n{}",
            compiled.render()
        )
    })
}

#[test]
fn conformance_tests_pass() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    kav_files(&root.join("tests/conformance"), &mut files);
    assert!(files.len() > 100, "found only {} tests", files.len());
    let failures: Vec<String> = files
        .iter()
        .filter_map(|f| {
            check_file(f)
                .map(|why| format!("{}: {why}", f.strip_prefix(&root).unwrap_or(f).display()))
        })
        .collect();
    assert!(
        failures.is_empty(),
        "{} of {} conformance tests failed:\n{}",
        failures.len(),
        files.len(),
        failures.join("\n\n")
    );
}

/// Examples 01–04 run successfully, and their examples pass (roadmap Phase 1).
#[test]
fn examples_run() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    for name in [
        "01-hello/hello.kav",
        "02-basics/basics.kav",
        "03-errors/errors.kav",
        "04-contracts/bank.kav",
    ] {
        let path = root.join(name);
        let src = fs::read_to_string(&path).unwrap();
        let compiled = compile(&src, name, Options::default());
        assert!(!compiled.has_errors(), "{name}:\n{}", compiled.render());
        let run = kavach_interp::run(&compiled);
        assert_eq!(run.exit_code, 0, "{name} failed: {}", run.stderr);
        let tests = kavach_interp::test(&compiled, &Default::default());
        assert_eq!(
            tests.exit_code, 0,
            "{name} examples failed:\n{}",
            tests.stdout
        );
    }
}

/// The interpreter's run of the test file at `path`. A program that starts
/// itself again (`spawn process`, spec 9.8) runs through `kavach run`:
/// run inside this test, it would have only the test to start again.
fn interpret(
    compiled: &kavach_driver::Compiled,
    path: &Path,
    src: &str,
    args: &[String],
) -> kavach_interp::Output {
    if !src.contains("spawn process") {
        return kavach_interp::run_with_args(compiled, args);
    }
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_kavach"))
        .arg("run")
        .arg(path)
        .args(args)
        .output()
        .unwrap();
    kavach_interp::Output {
        stdout: String::from_utf8_lossy(&out.stdout).into(),
        stderr: String::from_utf8_lossy(&out.stderr).into(),
        exit_code: out.status.code().unwrap_or(-1),
    }
}
