// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! The command list `kavach --help` prints, and each command's own part
//! of it for `kavach <command> --help` (spec 10.5).

use std::process::ExitCode;

pub(crate) const HELP: &str = "kavach: the Kavach programming language

usage:
  kavach new <name>         create a project (spec 10.5); inside one, run, build,
                            test and check need no file: they use src/main.kav
  kavach run <file.kav> [args…]   check a file, then run it with these arguments
  kavach run --native <f>   compile it to machine code first (a growing subset)
  kavach run --dry-run <f>  run it without executing capability calls (spec 8.5)
  kavach run --watch <f>    run it again each time a file of the project changes (spec 10.5)
  kavach run --hot <f>      swap in changed functions while it runs, keeping its values (spec 10.5)
  kavach run --tracy <f>    show its spans and GPU work live in the Tracy profiler;
                            `kavach build --tracy` builds a program that does (spec 14.12)
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

/// `kavach <command> --help` (or `-h`): the lines of `HELP` about that
/// command, with the lines that go on under them; all of it for a
/// command it doesn't name.
pub(crate) fn for_command(args: &[String]) -> Option<ExitCode> {
    let [command, flag, ..] = args else {
        return None;
    };
    if !matches!(flag.as_str(), "--help" | "-h") {
        return None;
    }
    print!("{}", usage(command));
    Some(ExitCode::SUCCESS)
}

/// The lines of `HELP` about `command`, with the lines that go on under
/// them; all of it for a command it doesn't name.
fn usage(command: &str) -> String {
    let mut out = String::new();
    let mut inside = false;
    for line in HELP.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("kavach ") {
            let words = trimmed.split_whitespace().nth(1).unwrap_or("");
            inside = words == command;
        } else if !line.starts_with("    ") {
            inside = false;
        }
        if inside {
            out.push_str(line);
            out.push('\n');
        }
    }
    if out.is_empty() {
        out = HELP.to_string();
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_command_shows_only_its_own_lines() {
        let build = super::usage("build");
        assert!(build.contains("kavach build --lib"), "{build}");
        assert!(
            build.contains("for other languages (spec 10.10)"),
            "{build}"
        );
        assert!(!build.contains("kavach run"), "{build}");
        assert_eq!(super::usage("nosuch"), super::HELP);
    }
}
