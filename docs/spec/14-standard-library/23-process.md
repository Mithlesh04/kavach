# 14.23 std.process

Other programs, run from Kavach: a compiler, a simulator, `git`, a tool with no library. The `Process` capability (8.1) allows it ([decision 0105](../../decisions/0105-std-process.md)).

```kavach
use std.process

fn lint(file: String) -> Int ! ProcessError uses Process("verilator") {
    const done = try process.run("verilator", ["--lint-only", file])
    print(done.out)
    done.code
}
```

| Function or method | Gives |
|---|---|
| `process.run(program, args, folder: "", env: [:]) -> Finished ! ProcessError` | Runs `program` to its end and gives its exit code, what it printed and what it cost |
| `process.start(program, args, folder: "", env: [:]) -> Child ! ProcessError` | Starts `program` and gives it back running, to read its output as it comes |
| `child.next() -> Output?` | The next line the program printed, waiting for it; `none` once it has ended and every line was read |
| `child.each(f)` | Calls `f(output)` with each line as it comes, until `f` gives `false` or the program has ended and every line was read |
| `child.wait() -> Exit` | Waits for the program's end, skipping what it still prints |
| `child.kill()` | Ends the program, and every program it started, at once |
| `process.peak_bytes() -> Int` | The most memory this program has held so far, in bytes. Needs no capability |

- `args` is a list, passed as it is, never through a shell: `"; rm -rf /"` is one odd argument, not a command. A program is a name looked up on the system's `PATH` (`"verilator"`), or a path (`"./tools/gen"`).
- `folder` is the folder the program runs in; `""` is the one this program runs in. `env` adds or changes environment variables for it; it gets this program's others too.
- Standard input is empty: the program reads nothing. Text sent to it is an open question.

The results:

| Type | Holds |
|---|---|
| `Finished` | `code: Int`, `out: String`, `err: String`, `cpu_ms: Int`, `peak_bytes: Int`; `out` and `err` are the lines printed, each ending in a line break |
| `Output` | `Out(line: String)` or `Err(line: String)`: a line of standard output or of standard error, without its line end |
| `Exit` | `code: Int`, `cpu_ms: Int`, `peak_bytes: Int` |
| `ProcessError` | `NotFound(program: String)`, `Denied(program: String)` or `Failed(reason: String)` |

- `code` is the program's exit code: 0 most often means it went well. A program ended by a signal on Linux or macOS gives 128 plus the signal's number, as shells say it.
- `cpu_ms` is the processor time the program and the programs it started spent, in milliseconds. `peak_bytes` is the most memory they held at once: on Windows the memory they committed, on Linux and macOS the largest resident set of any of them. Both are for budgets and reports, so a tool that runs too long or grows too big can be seen.
- A line that isn't valid UTF-8 is read with `�` in place of the bytes that aren't.

A program is a tree: what it starts is part of it. `kill()`, a `Child` whose last holder ends, a task cancelled while it waits for one (9.6), and a time limit (`.timeout`) all end the whole tree, through a job object on Windows and a process group on Linux and macOS. So a timeout never leaves a simulator running behind it.

```kavach
use std.process

fn simulate(top: String) -> Exit ! ProcessError uses Process("vsim"), Console {
    const sim = try process.start("vsim", ["-c", top], folder: "build")
    sim.each(out => match out {
        Output.Out(line) => {
            if line.contains("Error") {
                print(line)
            }
            true
        },
        Output.Err(line) => {
            print("! {line}")
            true
        },
    })
    sim.wait()
}
```

Waiting for a line or for the end lets other tasks take their turns (9.7), so a program can run several tools at once, each in its own task.

### Which programs

`Process` can be granted for one program, as `Fs` is for one folder (8.1):

| Written | Allows |
|---|---|
| `uses Process` | Running any program |
| `uses Process("verilator")` | Only `verilator` |

The name is checked when the program runs, since it is a value: while a function with `uses Process("verilator")` runs, every `run` and `start` in it, and in everything it calls, must name that program, or fails with `Denied(program)`. A name without a `/` matches only that name, looked up on `PATH`; a name with one matches only that path. As folders do, names nest: inside two of them, a program must match both, so a function that runs two tools has `uses Process`, or calls one function per tool. A task keeps the names of the function that spawned it.

- `process.start_id`, `next_id`, `exit_id`, `kill_id`, `close_id` and `peak_bytes_id` are what `run`, `start` and the methods are built on, by the child's number.
- Generated tests (11.5) skip functions that use `Process`: there is no fake yet.

## Open questions

- Text sent to a program's standard input, and reading its output as `Bytes`.
- A fake `Process` for `kavach test`, answering from a table of programs and their output.
- Limits on a program's memory and processor time, set when it starts.
- Environment values that are secrets (8.7): today `env` takes plain text, so a secret is passed only after `reveal`.
