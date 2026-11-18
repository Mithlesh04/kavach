//! The `kavach` command (spec 10.5, 10.9).

mod approvals;
mod audit;
mod context;
mod deps;
mod env;
mod fmt;
mod help;
mod keep;
mod library;
mod native;
mod pack;
mod project;
mod proto;
mod watch;

use help::HELP;
use kavach_driver::Compiled;
use std::io::Write;
use std::process::ExitCode;

/// The stack the compiler runs on: it walks expressions recursively, and
/// spec 1.8 sets no limit on how long or deep one is.
const COMPILER_STACK: usize = 256 * 1024 * 1024;

fn main() -> ExitCode {
    // Ctrl+C and SIGTERM end a program as `cli.exit` does (spec 5.8),
    // caught before any other thread starts, the compiler's own included
    // (decision 0063): on Unix every thread then has them blocked.
    let args =
        env::take_flags(std::env::args().skip(1).collect()).map_or_else(|_| Vec::new(), |(a, _)| a);
    if args.first().map(String::as_str) == Some("run") {
        let hooks = match args.get(1).map(String::as_str) {
            Some("--native") => kavach_runtime_native::end_hooks,
            _ => kavach_interp::end_hooks,
        };
        kavach_runtime_native::rt_stop::catch_stops(hooks);
        // `spawn process` starts the program again as it was started (9.8).
        kavach_interp::set_restart(kavach_runtime_core::processes::this_command());
    }
    let work = std::thread::Builder::new()
        .name("kavach".into())
        .stack_size(COMPILER_STACK)
        .spawn(cli);
    match work.map(std::thread::JoinHandle::join) {
        Ok(Ok(code)) => code,
        // A panic in the compiler has printed its message already.
        Ok(Err(_)) => ExitCode::from(101),
        Err(_) => cli(),
    }
}

fn cli() -> ExitCode {
    let (args, env_flags) = match env::take_flags(std::env::args().skip(1).collect()) {
        Ok(taken) => taken,
        Err(e) => return fail(&e),
    };
    // `kavach build --help`: that command's usage (spec 10.5).
    if let Some(code) = help::for_command(&args) {
        return code;
    }
    // At a workspace's root, each member in turn (spec 10.14).
    if let Some(code) = project::workspace(&args) {
        return code;
    }
    // Run again on each change (spec 10.5).
    if let Some(code) = watch::watch(&args) {
        return code;
    }
    let args = project::expand(args);
    let (args, compress) = pack::take_flag(args);
    // `--tracy`: live in the Tracy profiler (spec 14.12, decision 0123).
    let args = native::take_tracy(args);
    // The `.env` file and `--env` values (spec 10.13).
    if let ([command, ..], Some(path)) = (&args[..], args.iter().find(|a| a.ends_with(".kav")))
        && matches!(command.as_str(), "run" | "test" | "build")
        && let Err(e) = env::prepare(command, env_flags, path)
    {
        return fail(&e);
    }
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["new", name] => project::new(name),
        ["add", rest @ ..] => deps::add(rest),
        ["update"] => deps::update(),
        ["audit", path] => audit::audit(path),
        ["approvals"] => approvals::list(),
        ["approve", id, rest @ ..] => approvals::answer(true, id, rest),
        ["deny", id, rest @ ..] => approvals::answer(false, id, rest),
        ["proto", path] => proto::proto(path),
        ["fmt", rest @ ..] => fmt::fmt(rest),
        // As without `--native`: a `--` before the program's arguments is Kavach's.
        ["run", "--native", path, rest @ ..] => {
            native::native(path, rest.strip_prefix(&["--"]).unwrap_or(rest))
        }
        ["run", "--hot", path, rest @ ..] => watch::hot(path, rest),
        ["run", "--dry-run", path, rest @ ..] => {
            let rest = rest.strip_prefix(&["--"]).unwrap_or(rest);
            let args: Vec<String> = rest.iter().map(|a| a.to_string()).collect();
            command(path, |c| kavach_interp::run_dry_with_args(c, &args))
        }
        ["build", "--lib", path] => library::build_lib(path, None),
        ["build", "--lib", path, "-o", dir] => library::build_lib(path, Some(dir)),
        ["build", path] => native::build(path, None, compress),
        ["build", path, "-o", out] => native::build(path, Some(out), compress),
        ["run", path, rest @ ..] if !path.starts_with("--") => {
            // `kavach run app.kav -- a b` and `kavach run app.kav a b` are the same.
            let rest = rest.strip_prefix(&["--"]).unwrap_or(rest);
            let args: Vec<String> = rest.iter().map(|a| a.to_string()).collect();
            command(path, |c| {
                native::give_shaders(c);
                kavach_interp::run_live_with_args(c, &args)
            })
        }
        ["test", path, rest @ ..] => test(path, rest),
        ["check", path] => match load(path) {
            Ok(c) => {
                gpu_notes(&c);
                let shown = kavach_driver::relative_path(std::path::Path::new(path));
                eprintln!("{shown}: no errors");
                ExitCode::SUCCESS
            }
            Err(code) => code,
        },
        ["check", "--format", "json", path] => match read(path) {
            Ok(c) => {
                print!("{}", kavach_ai::messages_json(&c));
                ExitCode::from(u8::from(c.has_errors()))
            }
            Err(code) => code,
        },
        ["outline", path] => match load(path) {
            Ok(c) => {
                let text = kavach_ai::outline(&c);
                print!("{text}");
                let (short, full) = (text.chars().count(), c.src.chars().count().max(1));
                eprintln!(
                    "outline: {short} of {full} characters ({}%)",
                    short * 100 / full
                );
                ExitCode::SUCCESS
            }
            Err(code) => code,
        },
        ["context", name] => context::context(name, None, "2000"),
        ["context", name, "--budget", budget] => context::context(name, None, budget),
        ["context", name, path] => context::context(name, Some(path), "2000"),
        ["context", name, path, "--budget", budget] => context::context(name, Some(path), budget),
        ["query", "tokens", rest @ ..] => query_tokens(rest),
        ["fix", rest @ ..] if !rest.is_empty() => fix(rest),
        ["query", kind, name] => context::query(kind, name, None),
        ["query", kind, name, path] => context::query(kind, name, Some(path)),
        ["mcp"] => match kavach_ai::serve(std::io::stdin().lock(), std::io::stdout()) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::from(1)
            }
        },
        ["lsp"] => match kavach_lsp::serve_stdio() {
            Ok(true) => ExitCode::SUCCESS,
            Ok(false) => ExitCode::from(1),
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::from(1)
            }
        },
        ["--version" | "-V"] => {
            println!("kavach {}", kavach_driver::compiler_version());
            ExitCode::SUCCESS
        }
        // What built an executable (spec 10.5).
        ["version", program] => match std::fs::read(program)
            .map_err(|e| e.to_string())
            .and_then(pack::readable)
        {
            Ok(bytes) => match kavach_driver::find_marked(&bytes) {
                Some(text) => {
                    println!("{text}");
                    ExitCode::SUCCESS
                }
                None => {
                    eprintln!(
                        "error: {program} wasn't built by `kavach build`: it carries no Kavach version (spec 10.5)"
                    );
                    ExitCode::from(1)
                }
            },
            Err(e) => {
                eprintln!("error: can't read {program}: {e}");
                ExitCode::from(1)
            }
        },
        ["help" | "--help" | "-h"] => {
            print!("{HELP}");
            ExitCode::SUCCESS
        }
        _ => {
            eprint!("{HELP}");
            ExitCode::from(2)
        }
    }
}

/// Reads and checks `path` without printing anything.
/// `kavach query tokens [--format json] <files or folders>…`: the semantic
/// marks of each file (spec 13.2), one JSON line each, for tools that color
/// Kavach outside an editor, as the docs site does.
fn query_tokens(args: &[&str]) -> ExitCode {
    let paths: Vec<&str> = args
        .iter()
        .copied()
        .filter(|a| !matches!(*a, "--format" | "json"))
        .collect();
    if paths.is_empty() {
        eprintln!("error: give files or folders: `kavach query tokens src/main.kav`");
        return ExitCode::from(2);
    }
    let mut files = Vec::new();
    for p in paths {
        kav_files(std::path::Path::new(p), &mut files);
    }
    for f in files {
        println!("{}", kavach_lsp::tokens_json_line(&f));
    }
    ExitCode::SUCCESS
}

/// `kavach fix <files or folders>…` (spec 10.4): the fixes that have one
/// right answer, applied, with a line for each.
fn fix(args: &[&str]) -> ExitCode {
    let mut files = Vec::new();
    for p in args {
        kav_files(std::path::Path::new(p), &mut files);
    }
    let mut any = false;
    for f in files {
        match kavach_lsp::fix_file(&f) {
            Ok(done) => {
                for title in &done {
                    println!("{f}: {title}");
                }
                any |= !done.is_empty();
            }
            Err(e) => {
                eprintln!("error: {f}: {e}");
                return ExitCode::from(1);
            }
        }
    }
    if !any {
        println!("nothing to fix");
    }
    ExitCode::SUCCESS
}

/// `path` if it is a file, or every `.kav` file under it, in order.
fn kav_files(path: &std::path::Path, out: &mut Vec<String>) {
    if !path.is_dir() {
        // `/` between folders on every system, as other commands print.
        out.push(path.display().to_string().replace('\\', "/"));
        return;
    }
    let mut entries: Vec<_> = std::fs::read_dir(path)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .collect();
    entries.sort();
    for e in entries {
        if e.is_dir() || e.extension().is_some_and(|x| x == "kav") {
            kav_files(&e, out);
        }
    }
}

fn read(path: &str) -> Result<Compiled, ExitCode> {
    read_as(path, false)
}

/// `path` compiled, as a library when `library` (spec 10.10).
pub(crate) fn read_as(path: &str, library: bool) -> Result<Compiled, ExitCode> {
    let src = std::fs::read_to_string(path).map_err(|e| {
        eprintln!("error: can't read {path}: {e}");
        ExitCode::from(2)
    })?;
    deps::fetch_for(path)?;
    let mut options = config(path)?.options_for(std::path::Path::new(path));
    options.library = library;
    // In a project, every file of `src/` is compiled with it (spec 10.1).
    let mut compiled = match kavach_driver::project_sources(std::path::Path::new(path)) {
        Some(sources) => kavach_driver::compile_sources(sources, options),
        None => kavach_driver::compile(&src, path, options),
    };
    // Tagged literals, checked by their tags' checkers (spec 10.15).
    if !compiled.has_errors() {
        let found = kavach_interp::check_literals(&compiled);
        compiled.messages.extend(found);
    }
    Ok(compiled)
}

/// The nearest `kavach.toml` for `path`, or the defaults (spec 10.4).
fn config(path: &str) -> Result<kavach_driver::Config, ExitCode> {
    kavach_driver::Config::for_file(std::path::Path::new(path)).map_err(|e| {
        eprintln!("error: {e}");
        ExitCode::from(2)
    })
}

/// Each loop that waits for the GPU each round, and why (spec 15.2). This
/// takes lowering the program, so only a program with GPU work pays.
fn gpu_notes(c: &Compiled) {
    if c.gpu_maps.is_empty() {
        return;
    }
    if let Ok(program) = kavach_kir::lower(c) {
        for (line, why) in &program.notes {
            let (file, line) = program.location(*line);
            eprintln!("{file}:{line}: note: this loop waits for the GPU each round: {why}");
        }
    }
}

/// Reads and checks `path`, printing any messages. Fails on compile errors.
pub(crate) fn load(path: &str) -> Result<Compiled, ExitCode> {
    shown(read(path)?)
}

/// `compiled`'s messages printed; it, unless one is an error.
pub(crate) fn shown(compiled: Compiled) -> Result<Compiled, ExitCode> {
    eprint!("{}", compiled.render());
    if compiled.has_errors() {
        return Err(ExitCode::from(1));
    }
    Ok(compiled)
}

fn command(path: &str, run: impl FnOnce(&Compiled) -> kavach_interp::Output) -> ExitCode {
    let compiled = match load(path) {
        Ok(c) => c,
        Err(code) => return code,
    };
    let output = run(&compiled);
    let _ = std::io::stdout().write_all(output.stdout.as_bytes());
    let _ = std::io::stderr().write_all(output.stderr.as_bytes());
    // `main` returned while a stop signal let it finish (decision 0088).
    let code = kavach_runtime_core::stop_holds::asked_code().filter(|_| output.exit_code == 0);
    ExitCode::from(u8::try_from(code.unwrap_or(output.exit_code)).unwrap_or(1))
}

/// Stops `kavach test` when one test, example or generated case runs past
/// sixty times the time budget (a minute by default), naming it (spec
/// 11.9): a test waiting forever would otherwise hang the run.
fn watchdog(budget_ms: u64) {
    let limit = std::time::Duration::from_millis(budget_ms.saturating_mul(60));
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(500));
            if let Some((name, took)) = kavach_interp::longest_running()
                && took > limit
            {
                eprintln!(
                    "error: {name} has run for {} s, past the limit of {} s, and was stopped (spec 11.9)",
                    took.as_secs(),
                    limit.as_secs()
                );
                std::process::exit(1);
            }
        }
    });
}

/// `kavach test`: examples, `test` blocks and generated tests (spec 11.9).
/// Found cases go to `tests/found/<file>.kavf` next to the file.
fn test(path: &str, flags: &[&str]) -> ExitCode {
    let settings = match config(path) {
        Ok(c) => c.test,
        Err(code) => return code,
    };
    let Some(budget_ms) = settings.budget_ms() else {
        eprintln!(
            "error: `time_budget = \"{}\"` in kavach.toml: write it like \"1s\" or \"500ms\"",
            settings.time_budget
        );
        return ExitCode::from(2);
    };
    let mut opts = kavach_interp::TestOptions {
        // Every run prints its seed, so any run can be repeated.
        seed: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(1, |d| d.as_nanos() as u64 % 1_000_000),
        cases: settings.cases,
        // About a million loop rounds or calls per second of budget.
        step_budget: budget_ms * 1_000,
        found_file: None,
    };
    let mut keep = false;
    let mut flags = flags.iter();
    while let Some(flag) = flags.next() {
        match (*flag, flags.clone().next()) {
            ("--keep", _) => keep = true,
            ("--seed", Some(n)) if n.parse::<u64>().is_ok() => {
                opts.seed = n.parse().unwrap_or(1);
                flags.next();
            }
            ("--deep", _) => opts.cases = settings.deep_cases,
            _ => {
                eprintln!(
                    "error: unknown option `{flag}` for `kavach test`
{HELP}"
                );
                return ExitCode::from(2);
            }
        }
    }
    opts.found_file = Some(project::found_file(std::path::Path::new(path)));
    // This process only runs tests: nothing may change the machine.
    kavach_interp::forbid_changes();
    if keep {
        return keep::keep(path, &opts);
    }
    watchdog(budget_ms);
    command(path, |c| kavach_interp::test(c, &opts))
}

fn fail(message: &str) -> ExitCode {
    eprintln!("error: {message}");
    ExitCode::from(2)
}
