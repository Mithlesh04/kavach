# 0065. A panic stops only its task: how native code gets there

- **Status:** Accepted (2026-11-05, Mithlesh chose the recommendation: option 1)
- **Date:** 2026-11-04
- **Changes:** spec 5.6, 9.1; `runtime/native`, both backends

## Context

Spec 5.6 says a panic stops the task it happened in, and [decision 0062](0062-crashes-restarts-and-clocks.md) holds to it, as Tokio and Java contain a task's or thread's panic. The task that waits for the panicking one gets an error from `.wait()`. A panic in the main task still ends the program with code 101.

The interpreter can do this today: a panic is already a value (`Flow::Panic`), so a task's run can turn it into an error for whoever waits. Native code can't. `panic_with` (`runtime/native/src/rt.rs`) prints the message and ends the whole process. To stop one task, the runtime would have to leave that task's frames, which are compiled code with runtime calls in between, and go back to where the task began.

The interpreter must not go first, since native code must behave as it does ([decision 0014](0014-development-interpreter.md)).

What was found (2026-11-04):

- LLVM marks every function `nounwind` (`compiler/backend/llvm/src/func.rs`).
- Cranelift writes unwind information for object files. Its JIT (`kavach run --native`, `cranelift-jit` 0.125) registers none: there is no `__register_frame` or `RtlAddFunctionTable` call.
- The runtime's 73 entry points are `extern "C"`, and Rust ends the process when a panic tries to unwind through one.
- `panic_with` is called from 59 places in the runtime, some of them while a lock is held.

## Options

1. **Unwind, as C++ and Rust do.** LLVM functions lose `nounwind` and gain `uwtable`. Cranelift's JIT code gets its unwind tables registered by the runtime (`__register_frame` on Unix, `RtlAddFunctionTable` on Windows). The entry points become `extern "C-unwind"`. `panic_with` unwinds, and the start of each task (`rt_tasks.rs`, `rt_coroutines.rs`) catches it.
   - Rust frames release their locks on the way out.
   - Values held in compiled frames leak, since generated code has no cleanup on the way out. That is acceptable for a panic.
   - This is the most work, but it covers tasks on threads and coroutines alike.
2. **Leave the coroutine.** A task that runs as a coroutine (most tasks, decision 0048) is set aside on a panic: the host marks it failed and never resumes it, and its stack is let go of without unwinding.
   - This is little work.
   - It is safe only if no Rust frame on that stack holds a lock or other state: all 59 call sites need checking, and so does every new one.
   - Tasks that run on threads of their own (`tasks = "parallel"`) aren't covered, and their panics still end the program.
3. **Keep a panic fatal in native code**, and change spec 5.6 to say so. A supervisor (systemd, Kubernetes) restarts the program, and durable agents resume their events (8.5).

## Recommendation

Option 1, done in steps:

1. Cranelift object files (`kavach build`), which already carry unwind information.
2. LLVM.
3. The JIT, whose tables must be registered.

Until all three are done, a panic stays fatal in native code and in the interpreter alike, so the two keep behaving the same.

## Decision

Mithlesh chose option 1 (2026-11-05), in the three steps above. Why:

- **It is the only option that keeps spec 5.6 everywhere.** Option 2 covers coroutines but not tasks on threads of their own (`spawn thread`, `spawn cores`, decision 0069), so whether a panic ends the program would depend on how a task runs. Option 3 drops the promise.
- **It is the proven way.** C++ exceptions, Rust's `catch_unwind` (which Tokio uses to contain a task's panic) and Java's threads all unwind through compiled frames with the platform's unwind tables. The tools are already there: Cranelift writes the tables, LLVM writes them once `nounwind` goes, and `extern "C-unwind"` is stable Rust.
- **It stays safe as the runtime grows.** Option 2 is safe only while no Rust frame on the coroutine's stack holds a lock, which every new runtime call would have to be checked for. Unwinding runs Rust's own cleanup, so a lock held at a panic is released.
- **A panic costs nothing until it happens.** Unwind tables are data beside the code; a program that never panics runs the same instructions. The cost is size: the tables add a few percent to an executable (measured when step 1 lands, against the 720 KB budget of the size test).

Two details, so the work can start:

- **`.wait()` on a panicked task fails with an `Error`**, as it does for a cancelled task (9.1): `.wait()` already gives `T ! Error`, so no new type is needed. The message is `panicked: <message>` with the place, `at <file>:<line>`.
- **The panic is printed when it happens**, as `panic: <message>` and its place, as Rust prints a panicking thread's message. A bug is never silent, even when nobody waits for the task.
