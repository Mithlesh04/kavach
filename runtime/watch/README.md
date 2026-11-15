# runtime/watch/

Watching files and folders for `std.fs` (spec 14.3), through `notify` (decision 0116): watchers by number, the changes each has had, read one by one or all at once. Apart from `native/`, so a program that watches nothing doesn't link `notify` (decision 0024).

| File | Job |
|---|---|
| `Cargo.toml` | The crate `kavach-runtime-watch` and its one dependency, `notify` |
| `src/lib.rs` | Watchers by number and the calls the interpreter and native code make: `watch`, `next`, `poll`, `close`; a rename as a removal and a creation; paths under the watched path as written |
| `src/tests.rs` | Renames, paths, and a real folder in the system's temporary folder that tells of a new file |
