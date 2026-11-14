# runtime/process/

Other programs, run for `std.process` (spec 14.23) with no shell, as trees that end together ([decision 0105](../../docs/decisions/0105-std-process.md)). The interpreter and native code both use it, by the child's number. Its system calls are the only `unsafe` code outside `runtime/native` and one call in `runtime/gpu`; each has a `SAFETY:` comment.

| File | Job |
|---|---|
| `Cargo.toml` | The crate `kavach-runtime-process`, over `windows-sys` and `libc` |
| `src/lib.rs` | Starting a program with its arguments, folder and environment; each child's output read line by line into its inbox, and its end last; `next`, `kill`, `close` and the program's own peak memory |
| `src/windows.rs` | Windows: the program starts suspended and joins a job object that ends everything in it; the job's processor time and peak committed memory |
| `src/unix.rs` | Linux and macOS: a process group ended with `killpg`; `wait4` for the end, processor time and largest resident set |
| `src/tests.rs` | Output and errors as lines, the folder and environment, killing a tree, a missing program or folder, the end coming again |
