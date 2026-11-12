//! The `kavach` command (spec 10.5, 10.9).

mod approvals;
mod audit;
mod context;
mod deps;
mod env;
mod fmt;
mod keep;
mod library;
mod native;
mod pack;
mod project;
mod proto;
mod watch;

use kavach_driver::Compiled;
use std::io::Write;
use std::process::ExitCode;

const HELP: &str = "kavach: the Kavach programming language

usage:
  kavach new <name>         create a project (spec 10.5); inside one, run, build,
                            test and check need no file: they use src/main.kav
  kavach run <file.kav> [args…]   check a file, then run it with these arguments
  kavach run --native <f>   compile it to machine code first (a growing subset)
  kavach run --dry-run <f>  run it without executing capability calls (spec 8.5)
  kavach build [--compress[=max]] <file.kav> [-o <out>]   write an executable (named after the file), packed smaller if asked
  kavach build --lib <lib.kav> [-o <dir>]   write a shared library and a C header
                            for other languages (spec 10.10)
  kavach test <file.kav> [--seed N] [--deep]
                            run its examples and `test` blocks, then generated
                            tests for every function (spec 11)
  kavach test <file.kav> --keep   found cases that now pass become `example`s
  kavach check <file.kav>   only check it for errors
  kavach fmt [file.kav] [--check]   format a file, or every file of the project
  kavach audit                              list unsafe code, revealed secrets and capability use (spec 10.5)
  kavach approvals                          the requests the project's agents wait on (spec 8.3)
  kavach approve <id> --as <name>           approve one; `kavach deny <id> --as <name> [--reason <text>]` refuses it
  kavach proto file.kav                     print the .proto file for the program's types (spec 14.14)
  kavach check --format json <file.kav>   one compact JSON line per message
  kavach fix <files or folders>   apply the fixes that have one right answer and add no error (spec 12.7)

tools for AI assistants (spec 13), which save reading whole files:
  kavach outline <file.kav>                 declarations without function bodies
  kavach query callers <name> [file.kav]    every place a name is used
  kavach query uses <Capability> [file.kav] functions that can touch it
  kavach query fails <ErrorType> [file.kav] functions that can fail with it
  kavach query type <Name> [file.kav]       a type's fields, variants and methods
  kavach query find <text> [file.kav]       declarations whose name contains it
  kavach query tokens [--format json] <files or folders>   where each name comes from, one JSON line per file
  kavach context <name> [file.kav] [--budget N]   its code plus what it uses (default 2000 tokens)
                            query and context need no file inside a project,
                            and answer from its index, .kavach/index (spec 13.9)
  kavach mcp                                serve all of these to AI assistants over MCP (stdin/stdout)
  kavach lsp                                serve errors as you type to an editor (LSP, stdin/stdout)
  kavach --version                          the compiler's version and commit
  kavach version <program>                  what built an executable (spec 10.5)
";

fn main() -> ExitCode {
    let (args, env_flags) = match env::take_flags(std::env::args().skip(1).collect()) {
        Ok(taken) => taken,
        Err(e) => return fail(&e),
    };
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
    // The `.env` file and `--env` values (spec 10.13).
    if let ([command, ..], Some(path)) = (&args[..], args.iter().find(|a| a.ends_with(".kav")))
        && matches!(command.as_str(), "run" | "test" | "build")
        && let Err(e) = env::prepare(command, env_flags, path)
    {
        return fail(&e);
    }
    // Ctrl+C and SIGTERM end a program as `cli.exit` does (spec 5.8),
    // caught before any other thread starts (decision 0063).
    if args.first().map(String::as_str) == Some("run") {
        let hooks = match args.get(1).map(String::as_str) {
            Some("--native") => kavach_runtime_native::end_hooks,
            _ => kavach_interp::end_hooks,
        };
        kavach_runtime_native::rt_stop::catch_stops(hooks);
        // `spawn process` starts the program again as it was started (9.8).
        kavach_interp::set_restart(kavach_runtime_core::processes::this_command());
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
            command(path, |c| kavach_interp::run_live_with_args(c, &args))
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
    let src = std::fs::read_to_string(path).map_err(|e| {
        eprintln!("error: can't read {path}: {e}");
        ExitCode::from(2)
    })?;
    deps::fetch_for(path)?;
    let options = config(path)?.options_for(std::path::Path::new(path));
    // In a project, every file of `src/` is compiled with it (spec 10.1).
    if let Some(sources) = kavach_driver::project_sources(std::path::Path::new(path)) {
        return Ok(kavach_driver::compile_sources(sources, options));
    }
    Ok(kavach_driver::compile(&src, path, options))
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
    let compiled = read(path)?;
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
    if keep {
        return keep::keep(path, &opts);
    }
    command(path, |c| kavach_interp::test(c, &opts))
}

fn fail(message: &str) -> ExitCode {
    eprintln!("error: {message}");
    ExitCode::from(2)
}
