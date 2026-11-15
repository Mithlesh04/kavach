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
| `src/parts.rs` | Every part of the machine (spec 14.25, [decision 0112](../../docs/decisions/0112-every-detail-of-the-machine.md)): each kind as rows of text |
| `src/parts_linux.rs`, `src/parts_linux_more.rs` | Linux: the parts from `/sys`, `/proc` and dpkg's list; volume sizes from `statvfs`, addresses from `getifaddrs` |
| `src/parts_windows.rs`, `src/parts_windows_more.rs`, `src/parts_windows_net.rs` | Windows: the registry, the SMBIOS tables, storage queries, the IP helper, the display configuration, performance counters and SetupAPI |
| `src/registry.rs` | Reading the Windows registry |
| `src/parts_linux_procs.rs`, `src/parts_windows_procs.rs` | Programs running, the ports they hold, the user and the accounts |
| `src/acts.rs`, `src/acts_unix.rs`, `src/acts_windows.rs` | What changes the machine: ending programs, freeing ports, adding and removing users, locking the session |
| `src/acts_tests.rs` | Those changes, tested so this machine never changes (see below) |
| `src/smbios.rs` | The SMBIOS tables: memory modules and ids |
| `src/windows.rs` | Windows: the program starts suspended and joins a job object that ends everything in it; the job's processor time and peak committed memory |
| `src/unix.rs` | Linux and macOS: a process group ended with `killpg`; `wait4` for the end, processor time and largest resident set |
| `src/tests.rs` | Output and errors as lines, the folder and environment, killing a tree, a missing program or folder, the end coming again; the machine's facts |

## Testing what changes the machine

`src/acts_tests.rs` ends only programs it started, and checks that adding and removing users is refused without the rights; it never locks the session. A real user is added and removed only where `KAVACH_ISOLATED=1` says the machine is a throwaway container run as root. On Windows with Docker Desktop, after building the Linux tests in WSL (`cargo test -p kavach-runtime-process --no-run`):

```
docker run --rm --network none -e KAVACH_ISOLATED=1 -v "<linux target>/debug/deps:/t:ro" ubuntu:24.04 /t/kavach_runtime_process-<hash> acts_tests --test-threads=1
```

The container has no network and is removed when the tests end. Windows has no such container on Windows 11 Home, so adding a Windows user is not yet tested with the rights (spec 14.25).
