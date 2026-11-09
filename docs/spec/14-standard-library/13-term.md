# 14.13 std.term

`std.term` colors and styles text printed to a terminal, draws progress bars and asks questions. It needs `Console`. The color functions take text and give it back with the codes a terminal understands.

```kavach
use std.term

fn main() {
    print(term.red("error:") + " file not found")
    print(term.bold(term.green("done") + " in 2 s"))
}
```

| Function | Gives `s`… |
|---|---|
| `term.black(s)`, `term.red(s)`, `term.green(s)`, `term.yellow(s)`, `term.blue(s)`, `term.magenta(s)`, `term.cyan(s)`, `term.white(s)`, `term.gray(s)` | in that color |
| `term.bold(s)` | in bold |
| `term.dim(s)` | faint |
| `term.italic(s)` | in italics |
| `term.underline(s)` | underlined |

- **Colors are off when output is not a terminal.** When standard output goes to a file, a pipe or a test, or the environment variable `NO_COLOR` is set (`io.colors()` is `false`, 14.6), each function gives `s` unchanged. Logs and files never get escape codes.

  ```kavach
  // kavach run app.kav > out.txt
  print(term.red("error:"))      // out.txt holds: error:
  ```

- **Styles nest.** A style inside another of the same kind ends only itself: in `term.red("a" + term.green("b") + "c")`, `c` is red again.
- Each function ends with its own reset (the default color, or the end of bold, faint, italics or underline), never a full reset, so the outer styles stay on.

## Progress bars

A progress bar shows how much of a long job is done, on one line that is drawn again as the job goes on:

```kavach
let bar = term.progress(files.len())
for f in files {
    copy(f)
    bar = bar.step(1)
}
bar.done()
```

```
[############--------]  60%  6/10
```

| Function or method | Does |
|---|---|
| `term.progress(total) -> Progress` | A bar for a job of `total` steps, drawn at 0% |
| `bar.step(n) -> Progress` | `n` more steps are done: the bar is drawn again (only when its percentage changed) |
| `bar.done()` | The job is over: the bar is drawn full, and its line ends |

The bar is drawn only when output is a terminal. Otherwise these print nothing, so a log or a pipe gets no half-drawn lines.

## Prompts

| Function | Gives |
|---|---|
| `term.prompt(question) -> String?` | Prints `question` and a space, and gives the line typed, or `none` at the end of input |
| `term.prompt_int(question) -> Int?` | Asks for a whole number: an answer that isn't one gets `Please type a whole number.` and the question again; `none` at the end of input |
| `term.prompt_float(question) -> Float?` | The same for a number (`2.5`, `3`), with `Please type a number.` |
| `term.prompt_secret(question) -> Secret?` | Asks for a password or a token, the typing not shown (`io.read_hidden`), and gives `Secret(text: …)`; `none` at the end of input |
| `term.confirm(question, default: Bool) -> Bool` | Asks a yes-or-no question, shown with `[y/N]` (or `[Y/n]` when `default` is `true`): `y` or `yes` gives `true`, `n` or `no` gives `false`, anything else, an empty line or the end of input gives `default` |
| `term.choose(question, options: [String]) -> Int?` | Prints `question` and the options, numbered from 1, and asks for a number: gives the index (from 0) of the option chosen, or `none` when the answer is not one of them |

Prompts read standard input with `io.read_line` (14.6), whether or not it is a terminal, so a script can answer them through a pipe.

They are named `prompt`, not `ask`, which is the AI's call (8.4), so a question at the keyboard is never taken for a question to a model ([decision 0053](../../decisions/0053-terminal-questions-are-prompt.md)). A call to an old name (`term.ask`, `ask_int`, `ask_float`, `ask_secret`) is a compile error that names the new one.

`Secret` is `pub type Secret { secret text: String }`: its `text` is a `secret` field (2.11), so the compiler keeps it out of `print`, logs and AI prompts (8.7), and it prints as `<secret>`. Pass `s.text` to a `secret` parameter, or `reveal(s.text)` where the plain text is really needed.

```kavach
const token = term.prompt_secret("API token?") ?? return
connect(token.text)                 // `fn connect(secret token: String)`
```

```kavach
const name = term.ask("Your name?") ?? "friend"
const age = term.ask_int("Your age?") ?? 0
if term.confirm("Delete {count} files?", default: false) {
    remove_all()
}
```
