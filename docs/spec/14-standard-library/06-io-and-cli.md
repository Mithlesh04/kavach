# 14.6 std.io and std.cli

| Function | Gives |
|---|---|
| `io.read_line() -> String?` | The next line typed (without its line ending), or `none` at the end of input. Needs `Console` |
| `io.read_hidden() -> String?` | A line typed without being shown on the screen, for a password, when standard input is a terminal; otherwise an ordinary line, so a pipe can answer it. `none` at the end of input. Needs `Console` ([decision 0036](../../decisions/0036-hidden-input-through-rpassword.md)) |
| `io.write(text)` | Prints `text` without a line end, shown at once (for a prompt, or a line drawn again with `\r`). Needs `Console` |
| `io.is_terminal() -> Bool` | Whether standard output is a terminal: `false` for a file, a pipe or a test. Needs `Console` |
| `io.colors() -> Bool` | Whether standard output shows colors: it is a terminal, and the environment variable `NO_COLOR` is not set ([no-color.org](https://no-color.org)). Needs `Console` |
| `cli.args() -> [String]` | The program's arguments, without its own name. Needs `Env` |
| `cli.exit(code)` | Ends the program at once with `code`. Needs `Env` |
| `cli.parse<T>() -> T ! CliError` | The arguments, parsed into record type `T`, or into union type `T` of commands. Needs `Env` |
| `cli.on_stop(grace: 10.seconds) -> StopHold` | A hold on the program's stop (5.8): while it is held, the first Ctrl+C, `SIGTERM` or closed terminal lets the program finish its work, for at most `grace`, instead of ending it at once |
| `hold.asked() -> Bool` | Whether a stop has been asked for |
| `hold.done()` | Lets go of the hold: once no hold is left after a stop was asked, the program ends |

`cli.parse` reads options from the field names and types of `T`, so a command line is described once, as a type:

```kavach
use std.cli

type Args {
    input: String            // --input data.csv   (required: no default)
    limit: Int = 10          // --limit 5
    verbose: Bool = false    // --verbose
}

fn main() {
    const args = cli.parse<Args>() catch e => {
        print(e)             // what is wrong, then the usage
        cli.exit(2)
        return
    }
}
```

- A field `name` is the option `--name` (an `_` in the name is written `-`).
- Fields may be `String`, numbers, `Bool` (a flag with no value) or optionals of these. A field without a default and not optional is required.
- `--help` gives an error whose text is the usage, listing every option with its type and default.

### Commands

When `T` is a union, each variant is a command, and the variant's fields are that command's options. The first argument names the command; the options follow it:

```kavach
use std.cli

type Command = AddItem(name: String, count: Int = 1) | Show(limit: Int = 10)

fn main() ! Error {
    match try cli.parse<Command>() {      // tool add-item --name milk
        Command.AddItem(name, count) => print("add {count} {name}")
        Command.Show(limit) => print("show {limit}")
    }
}
```

- A variant `AddItem` is the command `add-item`: lowercase, with a `-` before each word after the first.
- A command's options follow the rules for record fields above. A variant without fields is a command without options. Commands don't nest: an option can't be a union.
- A missing or unknown command, or `--help` in place of a command, gives an error whose text is the usage of every command with its options.
- A problem with a command's options, or `--help` after the command, gives that command's usage (`usage: tool add-item [options]`).
