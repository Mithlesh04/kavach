# Audit of `unsafe` Rust (2026-11-26)

Every `unsafe` block, function, `impl` and `extern` in the Rust code, read and checked against its `SAFETY:` comment. This is part of step 1 of the [to-do](../vision/todo.md), "Foundations audit and reliability". Each finding is fixed with a test; this file says which are done.

- **How:** read-only. Nothing was built, run or tested.
- **What was searched:** ripgrep over `compiler/`, `runtime/`, `tools/`, `interop/`, `stdlib/` and `editors/`, for `unsafe {`, `unsafe fn`, `unsafe impl`, `unsafe extern`, `#[unsafe(...)]` and `allow(unsafe_code)`.
- **Who reviewed what:** an AI agent read the code, runtime/process and runtime/native in parts. Every (a) finding was re-read against the code; (a) 1 was confirmed again by a maintainer's session. The (b) items are as found, each to be confirmed when fixed.

## Lint configuration

- `Cargo.toml` has `[workspace.lints.rust] unsafe_code = "deny"`. Every crate under compiler/, runtime/ and tools/ has `[lints] workspace = true`. No crate lacks it, and none overrides it.
- **runtime/native turns the lint off for the whole crate:** `#![allow(unsafe_code)]` at lib.rs:17. Every per-item `#[allow(unsafe_code)]` in the crate is therefore decorative, and new unsafe anywhere in the crate is never flagged. rt_map.rs:9 adds a redundant `#![allow]` of its own.
- **runtime/process allows it per file.** Every system-call file has its own `#![allow(unsafe_code)]`. That is fine.
- **runtime/gpu and runtime/matrix allow it per item**, on one item each. That is fine.
- **Hits in tools/cli/tests, tools/svd, compiler/checker, compiler/parser tests and compiler/backend/llvm/src/pointers.rs** are Kavach source text inside Rust strings: Kavach's own `unsafe { }` (spec 6.4). They are not Rust unsafe.

## Summary table

"Unsafe items" counts `unsafe {`, `unsafe fn`, `unsafe impl` and `unsafe extern`, including fn-pointer types and tests. The `#[unsafe(no_mangle)]` exports are counted separately.

| crate | unsafe items | `no_mangle` exports | allowed by AGENTS.md section 8? | blocks without `SAFETY:` | (a) | (b) |
|---|---|---|---|---|---|---|
| runtime/native | 189 | 96 | yes (decision 0016) | 0 blocks; 4 `extern` declaration blocks have no comment (unwind_tables.rs:22 and :29, rt_ptr.rs:16, crash_stack.rs:186); send.rs:94-105 has one comment for six impls | 4 | 22 |
| runtime/process | 104 | 0 | yes (decisions 0105 and 0110) | 1 (parts_windows_net.rs:97) | 2 | 7 |
| runtime/gpu | 1 | 0 | yes (decision 0077) | 0 | 0 | 0 |
| runtime/matrix | 1 | 0 | yes (decision 0111) | 0 | 0 | 1 |
| **compiler/interp** | **1** (`unsafe impl<T> Send for Baton<T>`, tasks.rs:83) | 0 | **no: a policy violation** | 0 | 1 | 1 |
| everything else | 0 | 0 | | | | |

**Policy issues:**

1. **compiler/interp/src/tasks.rs:82-83** has Rust `unsafe` outside the four allowed places, with no decision record. Either:
   - add a decision record and an exception in AGENTS.md section 8; or
   - remove it. This is preferred: wrapping in a struct whose Send comes from `Mutex` doesn't work for `Rc`, so the real fix is to make the cross-thread hand-off explicit. See (a) 5.
2. **runtime/native's crate-wide allow** means the lint enforces nothing there. See the closing note.
3. Decision 0077 says `unsafe` is allowed only in `runtime/native`, then makes the gpu exception; that is consistent with AGENTS.md. Decision 0111 says matrix has "one `unsafe` call", and that is true.

## (a) Likely bugs

Each is fixed with a test that fails before the fix. Each finding says when it was fixed; the others are open.

### 1. Packed lists with a limit: `Vec::from_raw_parts` and dealloc get the wrong capacity

- **Status:** Fixed 2026-11-26, "Keep packed list buffers at their real capacity", test `runtime/native/src/lists.rs` (`packed_lists_with_a_limit_free_what_they_took`, which watches the allocator through `rt_memory::watch`). No conformance test: a program can't see the wrong size, as `KAVACH_CHECK_LEAKS` counts objects, not bytes.
- **Where:** runtime/native/src/lists.rs:48-65 (`with_packed`) and rc.rs:23-27 (`free`).
- **Two formulas disagree:**
  - `free` rebuilds the buffer with `cap = l.cap * size / 8`, which rounds down.
  - `with_packed` uses `words_for(l.cap, size)`, which rounds up.
  - `with_packed` also cuts `l.cap` to the limit after `f` may have grown the Vec, so the next rebuild states a smaller capacity than the real allocation.
- **Scenario A:** `List<UInt8>` with `capacity(5)`. `room_for_limit` gives a 1-word buffer, `with_packed` sets `l.cap = min(8, 5) = 5`. `free` then computes 5 × 1 / 8 = 0, so `Vec::from_raw_parts(ptr, 0, 0)` is UB by the API contract, and the buffer leaks. Every such list leaks one allocation, and `KAVACH_CHECK_LEAKS` counts objects only, so it never shows. With `List<UInt8>.capacity(100)`, the 13-word (104-byte) buffer is freed as 100 / 8 = 12 words (96 bytes).
- **Scenario B:** `Ring<UInt8>` with `capacity(9)` and 11 pushes.
  1. `drop_first` → `with_slots` repacks the buffer to exactly 1 word, so `l.cap` = 8.
  2. The next push needs 2 words. `w.push(0)` grows the Vec to 4 words (RawVec's minimum), and `l.cap = min(32, 9) = 9`.
  3. The next `with_packed` rebuilds the Vec with capacity `words_for(9, 1)` = 2 although 4 were allocated. A later realloc or drop hands the global allocator a 16-byte layout for a 32-byte block.
- **Impact:** UB under the `GlobalAlloc` contract. With today's allocator (rt_memory.rs and small_blocks.rs), it files the block in a smaller class or leaks it. That is no corruption yet, but any allocator change makes it heap corruption.
- **Fix:**
  - Use one formula everywhere: `words_for(l.cap, size)` in `free` too.
  - In `with_packed`, when the limit cuts `l.cap`, rebuild the buffer at exactly `words_for(l.cap)` words.
  - Add `debug_assert_eq!(v.capacity(), words_for(l.cap, size))`.
  - Test a `Ring<UInt8>.capacity(9)` with 20 pushes, and `List<UInt8>.capacity(5)` made and dropped in a loop, checking the allocator's byte count.

### 2. `Shared`'s lock depth is read after the lock is released, so `update`'s panic list keeps stale cells

- **Where:** runtime/native/src/send.rs:198-212 (`shared_lock`), :71-78 (`unlock`) and :216-221 (`let_go_after_panic`).
- **What happens:**
  1. `shared_lock(p, false)` calls `c.unlock()`, which stores `owner = 0` once the depth reaches 0. Then it reads `c.depth.get()` (a plain `Cell`).
  2. With tasks on all cores (decision 0052), another worker can take the lock between the two and `depth.set(1)`. That is a data race on a non-atomic `Cell`, so formally UB already.
  3. The first thread then reads 1 and never removes the cell from its `UPDATING` list.
- **Consequences:**
  1. **Unbounded growth.** `UPDATING` grows by one `Arc` per lost race. Each later `update` pushes again.
  2. **Leaks.** The extra `Arc` makes `free_cell`'s `Arc::strong_count(c) == 1` false (send.rs:292), so the value the `Shared` holds is never released. `KAVACH_CHECK_LEAKS` then wrongly reports a cycle.
  3. **Broken lock after a panic.** If that task later panics (a task panic is caught, spec 9.1), `let_go_after_panic` sets `depth = 1` and `unlock()`s every stale cell, storing `owner = 0` while another worker holds it. Two workers are then inside the critical section, so `set_shared`'s `value.replace` and the release race, which can cause a double release and a use-after-free.
- **Scenario:** `spawn cores` tasks that all `counter.update(fn ...)` the same `Shared` in a loop, then any task panics.
- **Fix:**
  - Have `unlock()` return the new depth, computed before `owner` is stored. Use that value in `shared_lock` instead of reading `c.depth` again.
  - In `let_go_after_panic`, only unlock cells whose `owner == me + 1`.
  - Test: 8 core tasks doing 100k `update`s on one `Shared`, then a check that `UPDATING` is empty and the value is freed under `KAVACH_CHECK_LEAKS`.

### 3. Ctrl+C, SIGTERM or a panic runs end hooks that free heap objects while other threads still run

- **Where:** runtime/native/src/rt_stop.rs:34-43, lib.rs:247-250, and rt.rs:251 (panic). The hooks are `rt_drops::forget_all` (rt_drops.rs:81), `rt_display::forget_all` (rt_display.rs:42) and `rt_json_checks::forget_all` (rt_json_checks.rs:53).
- **What happens:**
  - `stop()` runs on a thread the sigwait thread spawns (Unix) or on the console-control thread (Windows), and calls `rt::run_at_exit()` without taking the turn.
  - The three hooks `release` closures. That is a non-atomic rc, and at 0 `kv_free` → `pool::give` writes into the global `kv_pool`, which generated code pops from inline with no lock.
  - This breaks the invariant behind `unsafe impl Sync for Global` (pool.rs:54-56) and `obj_mut`.
  - `rt_display::own_form` (:55-63) and `rt_drops::run` (:135-148) copy a closure out of the table and call it without retaining it, so a concurrent `forget_all` frees it under them.
- **Scenario:** a program with a `Display` or `Drop` type prints or drops in a loop, and the user presses Ctrl+C. The main thread reads freed memory, or follows a corrupted pool free list, and crashes instead of exiting cleanly with 130.
  - `let_holders_finish` (the grace period of decision 0088) widens the window.
  - `panic_with` on one core while `spawn cores` tasks run on the others is the same race.
- **Fix:** these hooks exist only for the leak check. Either:
  - make them return at once unless `kv_free_at_exit` is set (only on the normal path in `run_entry`); or
  - split `at_exit` into finishing hooks (OTLP, flush), which run everywhere, and freeing hooks, which run only from `run_entry`.
  - Test: SIGINT to a built program spinning on a `Display` print.

### 4. `fs.map` bytes are checked as UTF-8, then read a second time to copy

- **Status:** Fixed 2026-11-26, "Copy mapped bytes before checking them as text", test `tests/conformance/14-standard-library/fs-map-text-is-checked-utf8.kav`. The race itself has no test; the test checks that mapped text is still checked. The other users of mapped slices were checked: the font, image, audio and texture decoders, `display`, equality, hashing, slicing, base64 and the copies (`to_vec`) are safe Rust that trusts no earlier check, and JSON never reads `Bytes` as text. A change in the middle can give them wrong data, not broken values.
- **Where:** runtime/native/src/rt_bytes.rs:91-106 (`BytesTextRange`, `BytesText`), with rt_map.rs:29-36.
- **What happens:** `std::str::from_utf8(d)` validates the mapped bytes, then `new_text(t)` reads them again to copy them. Spec 14.3 says another program's writes show through, so a write between the two reads puts invalid UTF-8 into a Kavach `String`.
- **Why it matters:** downstream code assumes valid UTF-8. text.rs:26 and text_items.rs:85 use `from_utf8_unchecked`, and std's `chars()` uses unchecked reads, so a lead byte at the end reads past the buffer.
- **Scenario:** `const b = try fs.map("app.log")` while a logger appends, then `b.text()`. Something outside the program triggers it, though the window is narrow.
- **Fix:**
  - For `Data::Mapped`, copy first, then validate the copy: `String::from_utf8(d.to_vec())`. That is still one copy.
  - Check the other users of mapped slices (JSON, font, image and audio decoders) for the same check-then-reuse pattern.

### 5. compiler/interp: a task's last `TaskHandle` can drop on the task's thread after it gave up the turn

- **Where:** compiler/interp/src/tasks.rs:252-256 (the `Job` closure), :418-421, and task_errors.rs:38-61 (`Drop for TaskHandle`).
- **What happens:**
  1. `child.run_task(work, span, &handle)` ends with `sched.end(id)`, which passes the turn on.
  2. The closure's own `handle: Arc<TaskHandle>` drops only after that.
  3. If it is the last `Arc`, `Drop for TaskHandle` runs on a thread without the turn. It calls `e.display()` on a `Value`, and dropping `result` drops `Value`s that hold `Rc`s, whose non-atomic counts the turn holder may be changing at the same moment.
- **How the last `Arc` ends up there:** after `sched.end` marks the task done, the parent can prune `frame.spawned` (tasks.rs:265-267) and drop the program's task value.
- **Scenario:** a server loop of `spawn handle(x)` whose result shares an `Rc` (a `String` or list) with the parent's variables. Rc counts are corrupted, which leads to a leak or a use-after-free. The window is small but real.
- **Fix:**
  - Pass `handle` by value into `run_task` and drop it (and anything else holding `Value`s) before `sched.end(id)`.
  - Give `Baton` a bound or a comment that names every place a `Baton` may be dropped.
  - Resolve the policy issue above.

### 6. runtime/process (Unix): `killpg` after the group may be gone kills a stranger's group

- **Status:** Fixed 2026-11-26, "Never signal a program's group after its reap", tests `runtime/process/src/reaped.rs` (the lock's logic: no signal after the reap, and a signal waits for a reap under way; these run on every system) and `runtime/process/src/unix.rs` (`a_group_is_ended_while_it_runs_and_never_signalled_once_reaped`, Linux and macOS only). The second fix was taken: `wait` first waits with `waitid(WEXITED | WNOWAIT)`, which leaves the program unreaped, then reaps it with `wait4` under a lock that `kill` and `close` take too; `kill` sends nothing once the reap is done. No test failed before: the old code had no state to test, and pid reuse can't be forced in a test. Left for a maintainer's review: a program the tree started that outlives the first one and has closed its output is no longer ended by `close` on Linux and macOS, since its group's id can't be trusted once the first program is reaped (spec 14.23 says the whole tree ends; Windows' job still does that).
- **Where:** runtime/process/src/unix.rs:22-27 (`Tree::kill`), with lib.rs:181-194 (`kill`, `close`) and unix.rs:58-73 (`wait`).
- **What happens:**
  - `wait` reaps the leader with `wait4`, but `close(id)` still runs `killpg(group, SIGKILL)` when the Kavach `Child` is let go. A late `kill(id)` does the same.
  - The SAFETY comment's claim ("a group id isn't given to another group while any program is still in it") is true, but once every member has exited, the pgid is free.
  - Kavach starts each child with `process_group(0)`, so the next child to reuse that pid is a group leader by construction.
- **Scenario:** Sakshi-style tool driving. Child A (pid 1234) ends and is reaped, but its holder lives on. Thousands of spawns later, with pid_max 32768 in containers, child B gets 1234. Letting A go SIGKILLs B's whole tree, or any same-user group with that id.
- **Fix:**
  - Either keep the leader unreaped until `close`: wait with `waitid(P_PID, pid, WEXITED | WNOWAIT)`, `killpg` before the real reap, and take the rusage then;
  - or set a `reaped` flag, under a lock that `kill` also takes, and stop signalling once the group is empty.

### 7. runtime/process (Windows): the IP-helper table has no retry, so `free_port` falsely succeeds

- **Status:** Fixed 2026-11-26, "Ask again for IP helper tables that grew", test `runtime/process/src/parts_windows_procs.rs` (`a_table_that_grows_between_the_calls_is_asked_for_again`, `a_table_that_never_fits_is_an_error_not_an_empty_table`, with a fake IP helper). Before the fix they failed with `left: Ok(None) right: Ok(Some(30))` and `left: Ok([]) right: Err(122)`. A table that can't be read now makes `free_port` fail with `Failed`; `system.ports()` still lists nothing then.
- **Where:** runtime/process/src/parts_windows_procs.rs:82-104 (`table`).
- **What happens:**
  - The size comes from the first call, with one spare word.
  - If a connection opens before the second call, the call returns ERROR_INSUFFICIENT_BUFFER, and `table` returns an empty Vec.
  - `acts::free_port` then sees no holders and returns `Ok("ended:")` while the port is still held.
- **Scenario:** `free_port(443)` on a busy server.
- **Fix:** loop while the status is 122, growing to the returned size plus some slack, a few times. If it still fails, return an error, not an empty table.

## (b) Unclear safety arguments: need a fix, a stronger comment or a test

### runtime/matrix

- **Fixed 2026-11-26, "Check list sizes before the fast matrix product", test `runtime/matrix/src/tests.rs` (`fast_checks_the_sizes_before_reading`).** The check comes after the empty case, which reads nothing.
- **product.rs:96-127 (`fast_band`), with product.rs:23:** `product` is a safe `pub fn`, but `b`'s length is never checked against `m * p` before `gemm` reads `m × p` items from `b.as_ptr()`.
  - `lhs` is slice-checked, and the exact path panics on bad sizes; only `fast: true` reads out of bounds.
  - Today the sizes come from std.tensor (stdlib/tensor/ops.kav:175-186), and `matrix.product_id` is "only for the standard library". So it is safe only while `shapes.spread` and the Kavach slices always give exactly `m*k` and `k*p` items, and nothing checks that.
  - Fix, O(1): at the top of `product`, `assert!(a.len() >= n.checked_mul(m)? && b.len() >= m.checked_mul(p)?)`, with checked multiplications. That makes the safe function sound for any input.

### compiler/interp

- **tasks.rs:83:** `unsafe impl<T> Send for Baton<T>` has no bound on `T`, so any type can cross threads through it. Its argument ("only made, moved and opened by the task holding the turn") is broken by (a) 5. It also needs a decision record (see the policy issues).

### runtime/native: threads

- **heap.rs:308-311 (`obj_mut`), :340-344, :397-401:** the SAFETY text says "native code runs on one thread". That has not been true since decision 0020 (`rt::parallel`, tasks on all cores).
  - The real argument is that no object is reachable from two threads (send.rs's copy-on-spawn).
  - Also, these functions return unbounded-lifetime `&mut` that callers overlap. For example `set_limit` holds `obj_mut` and then `list_mut`, two live `&mut` to one object. That is UB under Stacked Borrows, though harmless in practice. Prefer raw-pointer field writes in such helpers.
- **send.rs:304-318 (`copy`):** the comment says two threads can copy one object at once ("seen under load"). That contradicts the module invariant at send.rs:5-8 that "two threads ... must never hold the same object".
  - If two threads really reach one object, its non-atomic `rc` races too. The unsafe read of the `RefCell` hides a symptom.
  - Find which path lets two threads hold one object. The test named in the comment is `06-memory/drop-runs-once-for-copies-on-other-cores.kav`, so a `Drop` value's copies and `rt_drops::copied` are the likely place. Then state the invariant precisely.
- **send.rs:90-105:** one SAFETY comment covers six impls (`SharedCell`, `TaskObj`, `ChanObj`) and argues "only the thread holding the turn".
  - With many workers there is no single turn: `SharedCell` is guarded by its own spin owner, and `TaskObj`'s `Cell`s are written by the task's worker and read by the waiter's.
  - Write the real happens-before argument per type: the scheduler mutex in `end` and `wake`, and the spin lock's Acquire/Release.
- **pool.rs:54-56 (`unsafe impl Sync for Global`):** depends on "one thread at a time while not parallel". (a) 3 breaks it. The JIT (`kavach run --native`) is never parallel, but its signal thread still runs.
- **rt_par.rs:240-242, 279-281, 351-410:** the SAFETY comments cover the ABI (capture count, the transmute to the chunk signature).
  - They don't cover that the chunks run on several threads at once over the same capture objects. Soundness rests on a compiler guarantee: chunk bodies never change a capture's rc and write disjoint slots.
  - Name that guarantee, and the compiler check that enforces it, in the comment.
- **rt_drops.rs:135-148:** the `drop` closure from the global `TABLE` is called from any thread without retaining it. It is sound only while `drop` closures capture nothing, and while nobody releases them concurrently (see (a) 3).
- **small_blocks.rs:166-179, and pool.rs `give`:** a thread that only frees, never cutting a span, never registers `LEAVING`. Its free lists are lost at thread exit. That is a leak, not UB. Fix: `LEAVING.try_with(|_| {})` on the first `give`.
- **rt_memory.rs:141-150 (`GlobalAlloc`):** the comment doesn't cover re-entry.
  - `refill` registers a thread-local destructor from inside `alloc`. That is sound only because std registers TLS destructors without the global allocator.
  - State that, and the thread-safety argument (a per-thread `Local`, and `SHARED` behind a mutex that never allocates). Name the existing test.

### runtime/native: FFI, platform and crash path

- **crash_stack.rs:121-148 (Windows stack walk):** every frame with no function entry is treated as a leaf, with `[Rsp]` as its return address. JIT frames whose table wasn't registered (jit_unwind.rs:117 skips code over 4 GB) make it walk wild memory inside the crash path.
  - Fix: allow a leaf only for the first frame, stop when Rsp doesn't grow, and bound Rsp with `GetCurrentThreadStackLimits`.
- **crash_stack.rs:96-105 (`prefix_number`):** reads 8 bytes before each function start. That is safe on PE. On ELF it assumes adjacent PT_LOAD segments; `-z separate-code` breaks that. Add a comment or a bound.
- **rt_ptr.rs:6-7, 185-193:** "C may free what Kavach allocated" holds on Windows only if the C library uses the same dynamic UCRT. A `/MT` library corrupts the heap.
  - Document the limit in spec 6.4 and 10.10, or export an allocator pair for C.
- **lib_calls.rs:64-74, 111-121:** a re-entrant library call (C callback → library) sets the stack limit again in `begin`, and `kv_lib_leave` never puts the outer one back. Deep recursion then hits the guard page instead of giving the overflow panic.
  - Fix: save the limit in `begin` and restore it on leave.
- **Safe `pub extern "C-unwind" fn`s that dereference raw pointers:** `kv_set_functions` (crash_stack.rs:47, which carries `#[expect(clippy::not_unsafe_ptr_arg_deref)]`), `kv_print_str` and `kv_panic` via `rt::text` (rt.rs:285, `from_utf8_unchecked`), `kv_lib_text_in` (rt_ptr.rs:118), `kv_main`, `kv_lib_enter*`, `kv_par_for_red` (rt_par.rs:227).
  - Safe Rust in the workspace (the compiler crates link this library) can cause UB through them.
  - Make them `pub unsafe extern "C-unwind" fn` with `# Safety` docs. The ABI doesn't change.
- **Closure-pointer transmutes:** rt_tasks.rs:354, rt_drops.rs:147, rt_display.rs:62, rt_json_checks.rs:87, rt_ai.rs:60 and rt_events.rs:32. All are right today, but nothing checks the closure's type against the expected signature. Use one helper with one SAFETY argument, plus a debug assertion on the closure's shape.

### runtime/native: data and text

- **rt_files.rs:46-58 (`read_words`):** builds a `&mut [u8]` over uninitialised Vec capacity and hands it to `Read::read`. That breaks `from_raw_parts_mut`'s "initialized" rule.
  - `vec![0u64; room]` uses `alloc_zeroed`, which gives fresh zero pages for large sizes without touching them. That removes both unsafe blocks and keeps the speed the comment is after.
- **rt_map.rs:29-36 (`Mapping::bytes`):** hands out `&[u8]` over memory another process may change, which breaks the rule that a shared reference's memory does not change. (a) 4 is the concrete case. Say so in the comment, and list which users read twice.
- **text_items.rs:174-221 (`split_at_byte`, `push_part`, both `pub`):** the fast path copies raw bytes into inline items that :85 later trusts as UTF-8. That is sound only when `sep` is ASCII, which only the caller checks (rt_text.rs:95).
  - Add `assert!(sep.is_ascii())`, or make them `pub(crate)` and document the requirement.
- **rt_json_tape.rs:92-97 (`Doc::str`):** its soundness rests on `Builder::string`'s pointer-range test (rt_json_build.rs:51) and on `finish` receiving a byte-identical `base`. `Doc`'s fields are `pub`.
  - Make the fields private, and `debug_assert` the base length in `finish`.
- **lists.rs:104-127 (`room_for_limit`):** relies on `shrink_to` and `reserve_exact` giving exactly the capacity asked for. Std doesn't promise that. Rebuild the buffer exactly instead (part of (a) 1). Fixed 2026-11-26 with (a) 1: the buffer is made again with `Vec::with_capacity`, which std promises is exact.
- **rt_bytes.rs:230-231:** `unsafe fn crc32c_sse` has no `# Safety` doc ("the caller checked sse4.2").

### runtime/process

- **windows.rs:50-61 (`Tree::query<T>`) and parts_windows_procs.rs:265-290 (`each<T>`):** safe generic functions that `mem::zeroed::<T>()` a struct and let the OS write bytes into it. They are sound only when `T` is the struct that `class` or `level` names, and nothing ties the two together.
  - Make them `unsafe fn`, or map each class to its struct with a private trait.
  - `each` also leaks the buffer when `status != 0`.
  - Fixed 2026-11-26, "Tie each system query's class to its struct", test none: a wrong pairing no longer compiles, and the leak needs a failing `NetUserEnum`, which a test can't cause. A private trait gives each struct its class (`JobInfo`) or level (`Level`); `each` frees the buffer whatever the status.
- **windows.rs:118-141 (`resume`):** if the thread snapshot or the open fails, the child stays suspended, and `wait` blocks forever. Kill the job and return an error.
  - Fixed 2026-11-26, "End a program whose thread can't be resumed", test `runtime/process/src/windows.rs` (`a_program_whose_thread_isnt_resumed_is_an_error`, with a process id no process has). It didn't fail before: `resume` gave nothing to check. `start` now ends the job and fails.
- **parts_windows_more.rs:139-140:** `*(geometry.as_ptr() as *const DISK_GEOMETRY_EX)` dereferences a `Vec<u8>` pointer, which only promises alignment 1, as a struct holding an `i64`. That is formally misaligned UB, and the returned byte count `got` is unchecked.
  - Read the 8 bytes at `offset_of!` with `from_le_bytes`, as the code just above does.
  - Fixed 2026-11-26, "Read a disk's size as bytes at its place", test `runtime/process/src/parts_windows_more.rs` (`a_disks_size_is_read_from_bytes_at_any_alignment`, which also checks a short `got`). It didn't fail before: the misaligned read is UB that x86 doesn't show.
- **unix.rs:62-72 (`wait`):** if `wait4` fails with anything but EINTR, `status` stays 0, and the result is reported as exit 0 (a false success). Report -1.
  - Fixed 2026-11-26, "Report a failed wait for a program as -1", test `runtime/process/src/unix.rs` (`a_wait_that_fails_is_not_a_success`, Linux and macOS only: before the fix it gives 0).
- **acts_windows.rs:143-151 and registry.rs:178 (`wide`):** an interior NUL is not rejected, and act 3 (`remove_user`) skips the `user_name` check, so `remove_user("guest\0x")` deletes "guest".
  - Validate the name in act 3, or have `wide` reject NUL.
  - Fixed 2026-11-26, "Refuse a NUL in text for system calls", tests `runtime/process/src/registry.rs` (`text_for_a_system_call_never_holds_a_zero_before_its_end`) and `runtime/process/src/acts.rs` (`a_user_to_remove_never_holds_a_nul`); both check only the text, and nothing removes a user. Both were done: `wide` gives `None` for text holding a NUL, and act 3 refuses such a name with `Failed` on every system before any call. Act 3 doesn't apply `user_name`, as spec 14.25 says `remove_user` takes any name, for old accounts. The functions are new, so no test failed before.
- **parts_windows_net.rs:45-60:** `GetAdaptersAddresses` gets a fixed 32 KB buffer and no retry on ERROR_BUFFER_OVERFLOW (111). Hosts with many virtual adapters silently report none. Retry with the size the call returns.
  - Fixed 2026-11-26, "Ask again for the adapter list when it grows", test `runtime/process/src/parts_windows_procs.rs` (`the_adapter_list_is_asked_for_again_too`, with a fake call; the list now goes through the same `filled` as the IP helper's tables of (a) 7). No test failed before: the old code had no function to give a fake.
- **parts_windows_net.rs:97:** an `unsafe { utf16_at(..) }` with no SAFETY comment. It is sound; the same call at :109 has a comment.
  - Fixed 2026-11-26, "Merge the zero-ended text helpers", test none: a comment only.

## (c) Fine, counted per file

| file | sites |
|---|---|
| runtime/native: lib.rs (2 entry transmutes, the SIGPIPE ignore) | 3 |
| heap.rs (allocation, `data`, `slots`; outside the (b) note) | 9 |
| rc.rs (`free`, apart from (a) 1) | 1 |
| pool.rs | 7 |
| rt.rs (`text` takes only code-generator constants: cranelift codegen.rs:312, llvm stmt.rs:365; stack-limit slot) | 3 |
| rt_channels.rs (`Spin`/`Held`) | 3 |
| rt_workers.rs (`Body`: copied values) | 1 |
| rt_coroutines.rs (yielder) | 1 |
| crash_stack.rs | 12 |
| unwind_tables.rs (leaked for the program's life) | 4 |
| rt_stop.rs (the sigwait thread design; no async-signal-unsafe work in a handler) | 5 |
| rt_ptr.rs | 10 |
| rt_memory.rs | 19 |
| rt_clib.rs (`CString` rejects NUL; libraries never closed) | 9 |
| opaque.rs (the kind is checked before unwrapping; `Send + Sync` enforced) | 17 |
| small_blocks.rs | 21 |
| small_blocks/tests.rs | 7 |
| rt_gpu.rs | 2 |
| text.rs | 1 |
| text_items.rs | 4 |
| rt_json_read.rs | 4 |
| rt_json_checks.rs | 1 |
| rt_display.rs | 1 |
| display.rs | 1 |
| rt_map.rs (mmap/munmap pairing, Send/Sync) | 8 |
| table.rs | 1 |
| rt_exact.rs | 3 |
| runtime/process | 92 |
| runtime/gpu/pipeline.rs:45 (the kernel text and its `// loops end` line come only from the compiler or runtime constants; slice.rs builds its kernel from numbers) | 1 |

## Patterns that safe code, or a lint, could replace

- **Turn on clippy's `undocumented_unsafe_blocks`** (and `missing_safety_doc`) in `[workspace.lints.clippy]`. The "every unsafe has a `SAFETY:` comment" rule is then checked mechanically, including inside runtime/native's crate-wide allow.
  - Also consider replacing `#![allow(unsafe_code)]` in runtime/native/src/lib.rs with per-module allows, so the lint still flags unsafe in new modules.
- **`read_words`** (rt_files.rs): `vec![0u64; n]`, with no unsafe and no uninitialised memory.
- **`int_text`** (display.rs:95): `from_utf8(..).unwrap()` on 21 bytes costs nothing measurable.
- **Six closure-pointer transmutes:** one `closure_fn::<F>()` helper.
- **Duplicates in runtime/process:**
  - `utf16_at` (parts_windows_net.rs:23) and `text_at` (parts_windows_procs.rs:243) are identical; keep one next to `wide` in registry.rs.
    Fixed 2026-11-26, "Merge the zero-ended text helpers": one `text_at` in registry.rs; test none, as the code didn't change.
  - The `DISK_GEOMETRY_EX` cast can become byte reads.
  - `query<T>` and `each<T>` should be typed per class.
- **Redeclared `extern` blocks:** `calloc`/`free` (rt_ptr.rs:16) and `RtlAddFunctionTable` (unwind_tables.rs:29) could come from `libc` and `windows-sys`, which are already dependencies. The signatures were checked and match today.
- **Kept on purpose:**
  - `words_as_bytes` and rt_gpu's `as_bytes`: `bytemuck` would be a new dependency, which needs a decision record.
  - The inline-text `from_utf8_unchecked` calls: keep them for speed, but add `debug_assert!(from_utf8(..).is_ok())` so tests catch bad bytes from split, append or mapped data.
- **runtime/matrix:** checking the length up front makes the one `gemm` call's argument local and complete.
