# runtime/process/

Other programs, run for `std.process` (spec 14.23) with no shell, as trees that end together ([decision 0105](../../docs/decisions/0105-std-process.md)). The interpreter and native code both use it, by the child's number. It also says what machine the program runs on, for `std.system` (spec 14.25, [decision 0110](../../docs/decisions/0110-std-system.md)). Its system calls are the only `unsafe` code outside `runtime/native` and one call in `runtime/gpu`; each has a `SAFETY:` comment.

| File | Job |
|---|---|
| `Cargo.toml` | The crate `kavach-runtime-process`, over `windows-sys` and `libc` |
| `src/lib.rs` | Starting a program with its arguments, folder and environment; each child's output read line by line into its inbox, and its end last; `next`, `kill`, `close`, and the program's own peak memory and processor time |
| `src/machine.rs` | What machine the program runs on (spec 14.25, [decision 0110](../../docs/decisions/0110-std-system.md)): each fact by its number, as text or a number |
| `src/machine_linux.rs` | Linux: the facts from `/proc`, with no `unsafe` |
| `src/machine_windows.rs` | Windows: the version and processor from the registry, memory, cores, hostname and uptime from system calls |
| `src/machine_macos.rs` | macOS: the facts from `sysctl` (not yet checked on a Mac) |
| `src/windows.rs` | Windows: the program starts suspended and joins a job object that ends everything in it; the job's processor time and peak committed memory |
| `src/unix.rs` | Linux and macOS: a process group ended with `killpg`; `wait4` for the end, processor time and largest resident set |
| `src/tests.rs` | Output and errors as lines, the folder and environment, killing a tree, a missing program or folder, the end coming again; the machine's facts |
