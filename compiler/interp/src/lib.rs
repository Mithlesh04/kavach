//! A development interpreter for Kavach (decision 0014). It runs a checked
//! program straight from its syntax tree, for `kavach run`, `kavach test`
//! and the conformance tests. Shipped programs are compiled to native code.

mod agents;
mod ai;
mod bigint;
mod builtins;
mod bytes;
mod calls;
mod channels;
mod cli;
mod contracts;
mod destructure;
mod drops;
mod enums;
mod eval;
mod fake_net;
mod fakes;
mod files;
mod generics;
mod guard;
mod interp;
mod journal;
mod json;
mod json_decode;
mod literal_checks;
mod map;
mod net;
mod numbers;
mod ops;
mod parallel_map;
mod processes;
mod run;
mod stack;
mod std_agents;
mod std_base64;
mod std_compress;
mod std_crypto;
mod std_devices;
mod std_fs;
mod std_lib;
mod std_time;
mod std_window;
mod stmts;
mod task_errors;
mod tasks;
mod testing;
mod timeouts;
mod value;

use kavach_driver::Compiled;
pub use processes::set_restart;
pub use run::longest_running;
pub use testing::{Kept, TestOptions};
use value::Flow;

/// What a run printed, and how it ended.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Output {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

/// Interpreted code recurses on the native stack, so it gets a large one.
pub(crate) const STACK_SIZE: usize = 512 * 1024 * 1024;

/// Runs `main` (or the top-level statements) of a program that has no
/// compile errors. Exit codes follow spec 5.8: 1 when `main` returns an
/// error, 101 after a panic.
pub fn run(program: &Compiled) -> Output {
    run_with_args(program, &[])
}

/// Like [`run`], with the program's command-line arguments (spec 14.6).
pub fn run_with_args(program: &Compiled, args: &[String]) -> Output {
    run_main(program, args, false, false)
}

/// Like [`run_with_args`], with what the program prints shown as it
/// comes (`kavach run`): a question appears before the program waits for
/// the answer. The `Output` then holds only the exit code.
pub fn run_live_with_args(program: &Compiled, args: &[String]) -> Output {
    run_main(program, args, false, true)
}

/// `kavach run --dry-run` (spec 8.5): capability calls are checked and
/// logged, but answered with generated values instead of running.
pub fn run_dry_with_args(program: &Compiled, args: &[String]) -> Output {
    run_main(program, args, true, false)
}

fn run_main(program: &Compiled, args: &[String], dry_run: bool, live: bool) -> Output {
    on_big_stack(program, live, |i| {
        i.args = args.to_vec();
        i.dry_run = dry_run;
        // With `[build] version_flag`, `--version` alone prints what
        // builds the program, before `main` runs, as native code does
        // (spec 10.5).
        let (version, answers) = kavach_driver::program_version(program.path.as_ref());
        if answers && args == ["--version"] {
            i.out.push_str(&format!("{version}\n"));
            return 0;
        }
        // A child started by `spawn process` runs only its call (9.8).
        let result = match kavach_runtime_core::processes::site() {
            Some(site) => i.run_as_child(site as u32),
            None => i.run_main(),
        };
        let mut exit_code = 0;
        match result {
            // `main` is never cancelled.
            Ok(())
            | Err(Flow::Return(_) | Flow::Break(_) | Flow::Continue(_) | Flow::Cancelled) => {}
            Err(Flow::Exit(code)) => exit_code = code,
            Err(Flow::Fail(e)) => {
                i.err.push_str(&format!("error: {}\n", e.display()));
                exit_code = 1;
            }
            Err(Flow::Panic(message, span)) => {
                let (path, line) = i.c.shown_location(span.start);
                let shown = format!("panic: {message}\n  at {path}:{line}\n");
                i.err.push_str(&shown);
                exit_code = 101;
            }
        }
        // The last log events go to the collector, if there is one.
        // `std.log` events and agents' audit entries still waiting.
        kavach_runtime_net::otlp::finish();
        exit_code
    })
}

/// What runs before `kavach run` ends on Ctrl+C or `SIGTERM`: the last
/// log events and audit entries are sent, and output is flushed.
pub fn end_hooks() {
    kavach_runtime_net::otlp::finish();
    let _ = std::io::Write::flush(&mut std::io::stdout());
}

/// Runs each tag's checker on each tagged literal (spec 10.15): the
/// problems found, as compile errors and warnings.
pub fn check_literals(program: &Compiled) -> Vec<kavach_driver::Message> {
    if program.literal_uses.is_empty() {
        return Vec::new();
    }
    let found = std::sync::Mutex::new(Vec::new());
    on_big_stack(program, false, |i| {
        let problems = i.literal_problems();
        *found.lock().unwrap_or_else(|e| e.into_inner()) = problems;
        0
    });
    found.into_inner().unwrap_or_default()
}

/// Runs every `example` and `test` block, then generated tests for every
/// function (spec 11). Exits with 1 if anything failed.
pub fn test(program: &Compiled, opts: &TestOptions) -> Output {
    on_big_stack(program, false, |i| {
        let failed = i.run_tests(opts.seed) + i.generated_tests(opts);
        i32::from(failed != 0)
    })
}

/// `kavach test --keep` (spec 11.4): the found cases that now pass, as
/// `example` clauses for their functions.
pub fn keep(program: &Compiled, opts: &TestOptions) -> Kept {
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(STACK_SIZE)
            .spawn_scoped(scope, || interp::Interp::new(program).keep_found(opts))
            .expect("start the interpreter thread")
            .join()
            .expect("the interpreter thread finished")
    })
}

/// Runs `body` on a thread with a big stack. Tasks the program spawns
/// (spec 9) run on threads of their own, started here when asked.
fn on_big_stack(
    program: &Compiled,
    live: bool,
    body: impl FnOnce(&mut interp::Interp<'_>) -> i32 + Send,
) -> Output {
    let (spawner, jobs) = std::sync::mpsc::channel::<tasks::Job<'_>>();
    let made = std::sync::Arc::default();
    let for_main = std::sync::Arc::clone(&made);
    let mut output = std::thread::scope(|scope| {
        let main = std::thread::Builder::new()
            .stack_size(STACK_SIZE)
            .spawn_scoped(scope, move || {
                let mut i = interp::Interp::new(program);
                let mut shared = tasks::Tasks::new(spawner);
                shared.live = live;
                shared.made = for_main;
                i.tasks = Some(shared.clone());
                let exit_code = body(&mut i);
                i.flush();
                drop(i);
                shared.sched.end(0);
                let take = |s: &std::sync::Mutex<String>| {
                    std::mem::take(&mut *s.lock().unwrap_or_else(|e| e.into_inner()))
                };
                Output {
                    stdout: take(&shared.out),
                    stderr: take(&shared.err),
                    exit_code,
                }
            })
            .expect("start the interpreter thread");
        // Every task thread holds a sender, so this ends when the last
        // task has ended.
        for job in jobs {
            std::thread::Builder::new()
                .stack_size(STACK_SIZE)
                .spawn_scoped(scope, job)
                .expect("start a task thread");
        }
        main.join().expect("the interpreter thread finished")
    });
    // Every thread has ended, so every value but those in a cycle is
    // gone. A program that panicked or exited stopped where it was.
    if matches!(output.exit_code, 0 | 1) {
        let report = leak_report(program, &tasks::leaked_shared(&made));
        if live {
            eprint!("{report}");
        } else {
            output.stderr.push_str(&report);
        }
    }
    // The program's agents end with it: their folders are free again
    // (decision 0073), for the next program this process runs.
    kavach_runtime_core::agents::unlock_all();
    output
}

/// The `Shared` values made at `offsets` that were never freed, one line
/// per place (spec 6.2).
fn leak_report(program: &Compiled, offsets: &[u32]) -> String {
    let mut places: Vec<(String, usize)> = offsets
        .iter()
        .map(|&at| program.shown_location(at))
        .collect();
    places.sort();
    let mut report = String::new();
    for (i, place) in places.iter().enumerate() {
        if i > 0 && places[i - 1] == *place {
            continue;
        }
        let n = places.iter().filter(|p| *p == place).count();
        let shown = format!("{}:{}", place.0, place.1);
        report.push_str(&kavach_runtime_core::leak_message(n, &shown));
        report.push('\n');
    }
    if !report.is_empty() {
        report.push_str(kavach_runtime_core::LEAK_HINT);
        report.push('\n');
    }
    report
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    /// Lists are shared until changed: `push` changes a variable's list in
    /// place, and reading a list (`items.len()`, `items[i]`, passing it
    /// to a function) doesn't copy it. Both used to copy the whole list,
    /// so this program took minutes.
    #[test]
    fn lists_are_not_copied() {
        let src = "fn total(items: [Int]) -> Int {
    let sum = 0
    let i = 0
    while i < items.len() {
        sum += items[i]
        i += 1
    }
    sum
}

let items = []
for i in 0..100000 {
    items.push(i)
}
print(total(items))
";
        let compiled = kavach_driver::compile(src, "push.kav", Default::default());
        let start = Instant::now();
        let output = super::run(&compiled);
        assert_eq!(output.stdout, "4999950000\n");
        assert!(start.elapsed() < Duration::from_secs(10));
    }

    /// Finding a key in a map doesn't scan the map. It used to, so this
    /// program took minutes.
    #[test]
    fn map_keys_are_indexed() {
        let src = "let m: [Int: Int] = [:]\nfor i in 0..50000 {\n    m[i] = i\n}\nlet total = 0\nfor i in 0..50000 {\n    total += m[i] ?? 0\n}\nprint(total)\n";
        let compiled = kavach_driver::compile(src, "map.kav", Default::default());
        let start = Instant::now();
        let output = super::run(&compiled);
        assert_eq!(output.stdout, "1249975000\n");
        assert!(start.elapsed() < Duration::from_secs(10));
    }

    /// A method of a generic type is skipped with that reason, not
    /// reported as a `require` no input met (values of `Boxed<S>` aren't
    /// generated).
    #[test]
    fn methods_of_generic_types_say_why_they_are_skipped() {
        let method = |t: &str| {
            format!(
                "    fn sized(self, n: Int) -> {t}
    require n > 0
    {{
        self with {{ size: n }}
    }}
"
            )
        };
        let src = format!(
            "type Boxed<S> {{
    state: S
    size: Int

{}}}

type Plain {{
    size: Int

{}}}
",
            method("Boxed<S>"),
            method("Plain")
        );
        let compiled = kavach_driver::compile(&src, "gen.kav", Default::default());
        let output = super::test(&compiled, &Default::default());
        assert!(
            output
                .stdout
                .contains("not generated: Boxed.sized: its type is generic"),
            "{}",
            output.stdout
        );
        assert!(output.stdout.contains("✓ Plain.sized"), "{}", output.stdout);
        assert!(
            !output.stdout.contains("no generated input"),
            "{}",
            output.stdout
        );
    }

    /// A value holding a `Shared` can't be generated, so a function taking
    /// one is listed as not generated, not shown as passing with 0 cases;
    /// one whose `require` no input met but whose examples pass says so.
    #[test]
    fn functions_that_get_no_generated_input_say_why() {
        let src = "type Keys {
    current: Shared<String>

    fn now(self) -> String {
        self.current.get()
    }
}

fn key(text: String) -> Int
    require text.starts_with(\"kv1:\") and text.len() == 6
    example key(\"kv1:ab\") == 6
{
    text.len()
}
";
        let compiled = kavach_driver::compile(src, "keys.kav", Default::default());
        let out = super::test(&compiled, &Default::default()).stdout;
        let skipped = "not generated: Keys.now: takes values that can't be generated yet";
        assert!(out.contains(skipped), "{out}");
        let examples = "✓ key  checked by its examples only: no generated input met its `require`";
        assert!(out.contains(examples), "{out}");
        assert!(!out.contains("0 cases"), "{out}");
    }
}
