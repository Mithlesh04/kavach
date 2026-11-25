# 14.30 std.app

What a desktop program or game needs from the system it runs on ([decision 0118](../../decisions/0118-app-folders-crash-reports-and-dialogs.md)): its own folders for saves, settings and caches, a report when it crashes, and the system's dialogs.

```kavach
use std.app
use std.fs

fn main() ! Error uses Fs, Window {
    const saves = try app.data_dir("Skyfall")
    try app.crash_reports(saves + "/crashes")
    const file = app.open_file("Load a level") ?? ""
    if not file.is_empty() and app.confirm("Load", `Load {file}?`) {
        try fs.copy(file, saves + "/level.txt")
    }
}
```

**Folders.** Each is the folder of the program called `name` (a name, without `/` or `\`), made with its parents if missing. They make a folder, so they need `Fs.write`.

| Function | Gives |
|---|---|
| `app.data_dir(name) -> String ! AppError` | Where the program keeps its data, as saves: `%APPDATA%\name` on Windows, `$XDG_DATA_HOME/name` or `~/.local/share/name` on Linux, `~/Library/Application Support/name` on macOS |
| `app.config_dir(name) -> String ! AppError` | Where it keeps its settings: `%APPDATA%\name`, `$XDG_CONFIG_HOME/name` or `~/.config/name`, `~/Library/Application Support/name` |
| `app.cache_dir(name) -> String ! AppError` | Where it keeps what it can make again, which the system may clear: `%LOCALAPPDATA%\name\cache`, `$XDG_CACHE_HOME/name` or `~/.cache/name`, `~/Library/Caches/name` |

The folders are made if missing. On Linux and macOS they are made readable by their user alone (`0700`), as the XDG Base Directory spec asks, so others on the machine can't read a program's saves and settings. A relative `XDG_DATA_HOME`, `XDG_CONFIG_HOME` or `XDG_CACHE_HOME` is ignored, as that spec says.

**Crash reports.** `app.crash_reports(folder) ! AppError` makes the folder, and from then on a panic that ends the program (5.8) also writes a report into it: a file `crash-<time>.txt` with the message, where it happened, the functions that were running, the program, Kavach's version, the system and the time. It needs `Fs.write`.

The functions are listed innermost first, each with where it is defined, after the line that panicked:

```
panic: index 5 is out of range: the list has 2 items
  at shop.kav:14
  in price_of (shop.kav:12)
  in total (shop.kav:7)
  in main (shop.kav:2)
program: shop
kavach: 0.2.0
system: linux x86_64
time: 1794950605 (seconds since 1970, UTC)
```

The program is named by its main file without `.kav`, as `cli.parse`'s usage names it (14.6): `shop` for `shop.kav`, under `kavach run`, `kavach run --native` and in a built program alike, never `kavach` or the path of the executable.

A built program finds them by walking its stack with the system's unwinder and a table of its functions that `kavach build` writes ([decision 0118](../../decisions/0118-app-folders-crash-reports-and-dialogs.md)), so they cost nothing until a panic. A function the compiler inlined into its caller has no frame of its own, so it isn't listed.

**Dialogs.** Each shows the system's own dialog and waits for the person. They show a window, so they need `Window`.

| Function | Gives |
|---|---|
| `app.open_file(title) -> String?` | The file the person chose, or `none` when they cancelled |
| `app.save_file(title, name) -> String?` | Where the person chose to save, starting from the file name `name`, or `none` |
| `app.pick_folder(title) -> String?` | The folder the person chose, or `none` |
| `app.message(title, text)` | Shows `text` with an OK button |
| `app.confirm(title, text) -> Bool` | Asks `text` with Yes and No buttons: whether they chose Yes |

- `AppError` is `Unavailable(reason: String)`: the system gives no such folder (no home folder, as for some services), or it can't be made.
- A dialog gives a path the person chose; reading or writing it still needs `Fs` (8.1).
- On a machine with no screen a dialog gives `none`, or `false` for `confirm`.
- Under `kavach test`, files are fake (11.5): a folder is the path `/fake/data/name` (or `config`, `cache`) and none is made, and `crash_reports` writes nothing.
- Generated tests (11.5) skip functions that use `Window`, dialogs among them.

## Open questions

- The line of each call in the list, which needs line tables (debug info) in built programs.
- File type filters in `open_file`.
- The program's name in a crash report (chosen 2026-11-22, for a maintainer's review): its main file's name, as the usage of `cli.parse` gives it, so a project's report says `main`. Its package's `name` (10.4) would say more.
