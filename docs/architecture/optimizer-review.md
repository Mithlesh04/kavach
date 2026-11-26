# Optimizer review

A review of what native code still does that it could prove unneeded: step 1, item 5 of the [to-do](../vision/todo.md) (foundation 5). It covers inlining, scalar replacement, reference counts, bounds and overflow checks, vectorization, binary size and start-up, and the flags each backend gives its code generator. Each finding names the test that would show it done.

It sits beside [compiler-review.md](compiler-review.md), [checker-review.md](checker-review.md) and [tooling-review.md](tooling-review.md) and does not repeat them. The pass order, and which passes are needed for correctness, are in the compiler review's section 6.

**How it was made (2026-11-27, at commit `823261b4b`).** Source files were read. Small probe programs were built with the installed release, `kavach 0.2.0 (commit 369e1dbb)`, on Windows, with `KAVACH_LLVM_IR=1` (the IR before LLVM optimizes it) and `KAVACH_SROA_LOG=1` / `KAVACH_FUSE_LOG=1`. The printed IR was then run through the release's own `opt -O3` (LLVM 23.1.1) to see what survives, with `-pass-remarks` for the vectorizer. Nothing was built from the repository. The probes are tiny and given in each section. A finding marked **confirmed** was seen in a probe's IR; **from reading** was not run. File sizes were measured. **No speed was measured**: the machine was loaded, so every benefit below is an estimate.

## Summary

1. **The KIR inliner goes one level deep.** It copies each callee's body as it was *before* inlining (`inline.rs:60-68`), so in `a → b → c` the call to `c` stays in `a`. A record passed down two calls is then allocated and freed every round, even though LLVM later inlines the call and folds the loop's result to a constant (confirmed).
2. **Reading a list parameter by index takes and drops a reference per read** when the index is not "plain" (`xs[i % xs.len()]`): six reads cost six retain/release pairs, each release with a path to `kv_free`. Spec 6.1 says "the compiler removes most count updates"; here it removes none (confirmed). Arguments are passed owned: one more pair per call.
3. **Every remaining overflow check blocks vectorization.** `for x in xs { s += x }` stays scalar, as does a fused `map`. Bounds checks are not the problem: LLVM removed every one in the probes once the list was "held steady".
4. **Records stay on the heap** when chosen by `if`/`match`, when they hold text, or when passed past one level of calls (confirmed). Each read of a text constant allocates a new text object (confirmed: once per round in a loop).
5. **Runtime calls are opaque to LLVM.** No runtime declaration carries attributes (except `kv_bytes_data`/`kv_bytes_count`), and every pointer is an `i64` with `inttoptr`. So LLVM can't delete an allocation nobody reads, or keep values in registers across a runtime call.
6. **One `push` doubles the executable.** "hello world" is 540 KB on Windows (release 0.2.0; decision 0024 sets 300 KB). A program whose `push` can take the slow path calls `kv_rt`, the core dispatcher, and is 1,300 KB (measured).
7. **LLVM builds target plain x86-64 (SSE2)**: no `-mcpu`, no `target-cpu` (`backend/llvm/src/lib.rs:166-176`). **Cranelift builds target the build machine's CPU** (`cranelift_native::builder()`, `cranelift/src/lib.rs:152`), also for executables, so a Cranelift-built program may not run on an older machine (from reading).
8. **A stack check precedes every call**, also calls to functions that can never recurse, and it stays in the loop after LLVM inlines the call (confirmed).

## 1. Inlining

**What exists.** `kir/src/inline.rs` runs once over the whole program, after `unrecurse` and before `prune`:

- A function is inlined when it has at most 40 expressions (`MAX_SIZE`, line 29), or 120 when it takes or returns a record or tuple (`MAX_FIXED_SIZE`, line 35).
- It must not fail (`! E`), call C, or keep a `return` after `lift_returns` (lines 276-295). A `return` inside a loop stays, so such a function is never inlined.
- A recursive function is inlined into itself two levels deep, up to 400 expressions (lines 41-42). A recursion that `unrecurse` turned into a loop is inlined as its loop, up to 120 (line 48).
- Arguments that are numbers or locals are written in place of the parameter; others get a `let` (`copy_in`, line 198).
- Lambdas written into list methods are inlined at lowering (`lower_lambda.rs`), and `devirt.rs` turns a call through a once-stored callback into a direct call.

After that, LLVM's `opt -O3` inlines again: every Kavach function is `define internal` (`backend/llvm/src/func.rs:142`), so LLVM inlines freely. Cranelift does no inlining of its own.

**The gap.** LLVM's second round recovers the call's cost, but not what only KIR passes can do. `sroa`, `ranges`, `moves`, `fuse` and `version` work inside one function. What is still a call when they run stays a boundary for them. LLVM can't remove what crosses it: an allocation or a reference count is a call into the runtime, which LLVM can't see through (section 3).

1. **One level only (confirmed).** `bodies` is taken before any inlining (`inline.rs:60`), and `exprs_mut` visits children before their parent (`walk.rs:175-181`), so an inlined copy is not visited again. Probe:

   ```kavach
   fn inner(p: P) -> Int { p.x * 2 + p.y }
   fn outer(p: P) -> Int { inner(p) + 1 }
   // main: for i in 0..1000 { t += outer(P(x: i, y: 3)) }
   ```

   KIR inlines `outer` into `main`, but `main` still calls `inner`. So `sroa` can't split the record. After `opt -O3`, LLVM has inlined `inner` and folded the whole result to `kv_print_i64(i64 1003000)`, yet the loop still runs 1,000 rounds of:

   ```llvm
   %v13 = tail call i64 @kv_record(i64 100, i64 0, i64 2)
   ...  ; stack check, kv_stack_overflow
   tail call void @kv_free(i64 %v13)
   ```

2. **Size limits count KIR expressions, not cost.** A checked `a + b` and its line count as one expression. 40 is small next to LLVM's threshold (a cost of 225 at `-O2` and 250 at `-O3`, counted in LLVM instructions). There is no bonus for a function called from one place only, which C++ compilers always inline.
3. **What C++ would inline and Kavach doesn't:**
   - functions that can fail (`! E`): C++ inlines functions returning `std::expected` or throwing;
   - functions with an early `return` inside a loop (a search loop);
   - a function called once, whatever its size;
   - calls through interface values (`lower_iface.rs`: a box with one closure per method), where the concrete type is known at the call (C++ devirtualizes after construction);
   - callbacks stored more than once (`devirt.rs` handles once-stored ones only).

**Benefit (not measured).** Fixing (1) removes an allocation, a free and a call per round wherever a record goes two calls deep: helper-calls-helper code, builder chains, small geometry and money types. On the `everyday` benchmarks it affects `copies`, `nested` and `builderchain` when their helpers nest. Inlining functions that fail matters for every program that uses `try` in a loop.

**Test that shows it done.**
- A unit test in `inline/tests.rs`: `a → b → c`, all small; after `inline_calls`, `a` has no `Call`.
- An IR check (section 8): the probe above has no `kv_record` in `main` after `opt -O3`.
- Benchmark rows: `everyday` `copies` and `nested`, `chains` `builderchain`.

## 2. Scalar replacement and escape (`sroa.rs`)

**What exists.** `sroa.rs` splits a record local into one local per field when every field is a number, `Bool` or `Char` (lines 46-52), and every use is a build, a copy between locals, `SetField` or `GetField` (lines 11-17). A fixed-length list read at constant indexes is split the same way, up to 16 items. Parameters are never split (line 75). `Drop` types are kept whole. `loop_fields.rs` keeps a record's plain fields in locals while a loop runs. `inout_stash.rs` and `tuple_lets.rs` avoid tuples for `inout` results. Records of plain fields inside lists are flat (`list_layout.rs`), so they are not objects at all.

**Probe** (`KAVACH_SROA_LOG=1` printed `sroa: main splits [7, 11, 8, 0]`):

```kavach
type P { x: Int, y: Int }
type Named { name: String, n: Int }
fn add(a: P, b: P) -> P { P(x: a.x + b.x, y: a.y + b.y) }
fn pick(c: Bool, a: P, b: P) -> P { if c { a } else { b } }
// three loops of 1,000 rounds in main:
acc = add(acc, P(x: i, y: 1))          // split: no allocation (confirmed)
q = pick(i % 2 == 0, P(x: i, y: 2), q) // kv_record + kv_free every round
let r = Named(name: "a", n: i); t += r.n  // kv_str_new + kv_record + kv_free every round
```

**Gaps (all confirmed by the probe, except where marked).**

1. **A record chosen by `if` or `match` stays an object.** `scan_store` (line 335) knows `Record`, copies, `SetField` and blocks, but an `If` value falls to "keep" (lines 386-389). Both branches here are splittable values; the store could be split as one `if` per field.
2. **A record with a text, list or other heap field stays an object**, even when only a number field is read (`r.n` above). Splitting needs each heap field to be its own local that holds its reference, and a retain when the record is rebuilt.
3. **A record returned from a function that is not inlined** is always an object: the return value is one `i64`. LLVM can return a small struct in registers (`{i64, i64}`). `inout_stash.rs` already does something like it for `inout` values.
4. **A record passed to a function that is not inlined** is always an object, because parameters are never split. With section 1's fix, most small callees are inlined and this shrinks.
5. **Each read of a text constant allocates** (`kv_str_new`, which calls `heap::new_text`, `runtime/native/src/rt_strings.rs:16-17`). Inside a loop that is one allocation and one free per round. `inline.rs:233` notes this as a known cost.

**Benefit (not measured).** (1) and (5) are common in ordinary code: a "pick the best so far" loop, a default record, a label in a loop. Each costs an allocation and a free per round, where C++ uses registers or a pointer to constant data. The `records` benchmark (records in a list) is already flat, so it would not move; new benchmark rows are needed.

**Tests.**
- Unit tests in `sroa/tests.rs` for an `if`-chosen record and a record with a `String` field read by its number field.
- IR checks: no `kv_record` in the probe's loops; no `kv_str_new` inside a loop that reads a text constant.
- A chapter 6 conformance test for each new split (copy, change one side through the split path, check the other), as step 2 item 2 asks.

## 3. Reference counting

**What exists.**
- `moves.rs` turns each heap variable's last read into a `Move`: no retain, and the new owner may change it in place.
- `borrows.rs` lets a closure's function borrow its closure and the captures it never changes, so a call through a function value changes no count.
- Both backends borrow a *place* instead of retaining it: a local, or an item or field reached from one, when the index is "plain" (`backend/llvm/src/heap.rs:88-112`, `plain` at line 277: integers, locals, and arithmetic on them).
- A `for x in xs` loop and a `for i in 0..xs.len()` loop that reads `xs[i]` load the buffer once and touch no count (`hold_steady`, `backend/llvm/src/lists.rs`). Confirmed: no count updates in either loop.
- At exit, objects are not freed one by one unless leaks are checked (`kv_free_at_exit`).

**Probe:**

```kavach
fn weigh(xs: [Int], i: Int) -> Int {
    let a = xs[i % xs.len()]
    // … five more reads like it …
    a * 3 + b * 5 + c * 7 + d * 11 + e * 13 + f * 17
}
// main: for i in 0..1000 { t += weigh(xs, i) }
```

`weigh` is over the 40-expression limit, so KIR doesn't inline it. Before `opt`, each read is:

```llvm
  %v6 = load i64, ptr %v5        ; retain xs
  %v7 = add i64 %v6, 1
  store i64 %v7, ptr %v8
  ...                            ; the read
  %v40 = sub i64 %v39, 1         ; release xs
  store i64 %v40, ptr %v41
  %v42 = icmp eq i64 %v40, 0
  br i1 %v42, label %L35, label %L36   ; L35: call void @kv_free
```

The caller also retains `xs` before each call, and `weigh` releases it at its end. After `opt -O3`, LLVM has inlined `weigh` into the loop, and the loop body still holds 7 paths to `kv_free` and 11 count updates per round. LLVM can't remove them: it doesn't know the count is at least 1, and `kv_free` is opaque.

**Gaps.**

1. **Borrowing needs a "plain" index.** `xs[i % xs.len()]` is not plain because of `xs.len()`; neither is `xs[f(i)]` or `xs[xs.len() - 1]`. What borrowing needs is weaker: the index expression must not set, move or pass `inout` the base local. `walk::sets_local` already answers most of that.
2. **Arguments are passed owned** (`backend/llvm/src/calls.rs:50-55`: `self.expr(a)` retains unless it is a `Move`). Every call with a heap argument still used afterwards costs a retain in the caller and a release with a free check in the callee. Swift's default ("guaranteed" parameters) and C++'s `const&` cost nothing here. `borrows.rs` already does this for closures' functions, so both backends and the runtime's callers know how to keep to it.
3. **Counts are not paired up.** A retain followed by a release of the same local, with no store or call between, cancels. KIR knows the local is alive across the pair, LLVM doesn't. A small KIR or backend peephole could drop such pairs.
4. **Count updates are plain loads and stores** of an `i64` reached through `inttoptr`. That is correct, since values crossing threads are copied (spec 9), but it means LLVM's alias analysis rests only on the TBAA tags (`header` and `item`, `module.rs`). Section 5 shows where that stops.

**Benefit (not measured).** Gap 1 removes 2 count updates and a branch per indexed read of a list parameter in a loop: lookups in tables, ring buffers, `xs[i % n]`. Gap 2 removes 2 updates and a branch per heap argument per call: every non-inlined call that takes a list, map, text or record. The `lookups` and `everyday` `callbacks`/`nested` rows would move most.

**Risk to spec 6.1.** Gap 1 is low risk when the rule is written as "the index expression never sets, moves or passes `inout` the base local". Gap 2 is a calling convention, and the risky part is in-place change: a callee that borrows a parameter sees a count that does not include its own reference. Two rules keep it safe:
- reading a borrowed parameter as a whole value retains it;
- a `Move` of a borrowed parameter is a plain read, as `borrows.rs` does for closures (`heap.rs:101-103`).

Then `let ys = xs; ys.push(1)` sees a count of 2 and copies.

**Tests.**
- IR check: the probe's loop holds no `kv_free` after `opt -O3`.
- A chapter 6 conformance test: a function that copies its borrowed list parameter, changes the copy, and the caller's list is unchanged.
- `KAVACH_CHECK_LEAKS=1` stays at zero on the native cross-check (`tools/cli/tests/native.rs`).

## 4. Bounds and overflow checks

**What exists.**
- **Bounds.** The LLVM backend versions a counted loop: one check before it that every `list[var]` stays in range, then a copy of the loop with no checks (`backend/llvm/src/stmt.rs:128-215`). It covers only an index that *is* the loop's variable (`known_in_range`, `lists.rs:33-38`). `jam.rs` marks its reads `InRange`. Cranelift has neither.
- **Overflow.** `ranges.rs` finds each integer local's range and removes checks that can't fail. It turns `% c` into a compare and a subtract, and divides in 32 bits what fits. `ranges/accumulate.rs` bounds a counted loop's accumulators by its rounds. `version.rs` makes a checked-free copy of a loop that multiplies by a local in `0..2³¹`. `checked_fits.rs` handles `checked_*`. `counters.rs` turns `i % 7` into a counter that wraps.

**Probes.**
- `for i in 0..xs.len() { s += xs[i] }`: no bounds check is left. The overflow check `llvm.sadd.with.overflow.i64` stays in every round (confirmed).
- `xs[i + 1] - xs[i]` over `0..xs.len() - 1`, and a `while i > 0 { i -= 1; s += xs[i] }` loop: 4 `kv_bad_index` calls before `opt`, **0 after**. LLVM proved them, because the buffer and length were loaded once before the loop (confirmed).
- In `weigh` (section 3), `xs[i % xs.len()]` keeps its division-by-zero check and its `srem`. That is correct: the list could be empty.

**The gap.** Bounds checks are in good shape in simple loops, thanks to `hold_steady` and LLVM. Cranelift keeps every check, but it is not the release backend. What remains is **overflow checks on values the compiler knows nothing about: list items.** The range of `s` in `s += xs[i]` is unknown, because `xs[i]` can be any `Int`. Each check is cheap alone: an `add`, a `jo`, a cold call. But it ends vectorization (section 5), and it stops LLVM from reassociating or strength-reducing the loop.

Two ways to remove them without changing what a program does:

1. **Checked sums in blocks.** For a pure sum over a list, run each block of, say, 1,024 items fast: check that every item is within `±2⁵²` (a vector compare) and that the running total is within `±2⁶²`, then add without checks. If either test fails, run that block with the checked loop, which panics at the exact item and line. This is the "prefix bounds" idea `par/reduce.rs` already uses for sums on all cores (decision 0031), applied to one core. It fits KIR's existing `Fallback` node.
2. **Item ranges for lists a function builds.** In the probes, `xs.push(i % 13)` fills a list whose items are all in `0..13`. A list local built only by such pushes, and never passed anywhere that changes it, has a known item range. That is narrower and more fragile; (1) covers more programs.

**Checked arithmetic cost** (not measured). On x86, a checked add is `add` + `jo` to a cold block, about one extra instruction. Multiply uses `imul` + `jo`. The real cost is what it blocks: vector code, and LLVM's loop idioms (a sum of `i` turned into a formula).

**Tests.**
- IR check: `for x in xs { s += x }` has a `<2 x i64>` add after `opt -O3`.
- A conformance test where the overflow happens at item 700 of a 1,024-item block: same panic, same line, in the interpreter and in native code.
- Benchmark row: `math` `sum` on one core.

## 5. Vectorization

**What exists.** LLVM's loop vectorizer runs inside `opt -O3`. Loops whose body has a `%` by a constant are marked so LLVM never vectorizes them (`module.rs:94-103`): plain x86-64 has no vector division, and the scalar loop was faster on `stacks` (58 against 70 ms, 2026-11-05).

**Probe** (`opt -O3 -pass-remarks-missed=loop-vectorize`):

| Loop | Vectorized? | Why not (LLVM's remark) |
|---|---|---|
| `for i in 0..n { fs.push(Float(i)); xs.push(i); ys.push(0) }` | yes, width 2 | (counted loop with reserve: pushes are plain stores) |
| `for i in 0..xs.len() { ys[i] = xs[i] & 255 }` | yes, width 2, interleave 2 | |
| `for x in xs { s += x }` | no | overflow check: "control flow cannot be substituted for a select" |
| `xs.map(x => x + 1)` | no | "early exit loop with complex writes to memory" (the overflow panic) |
| `for x in xs { ys.push(x * k) }` (`Float`) | no | "call instruction cannot be vectorized": the push's slow path (`kv_rt`) is in the loop |
| fused `xs.map(x => x * 3)` then `.filter(y => y % 2 == 0)` then `.sum` | no | overflow check, and `%` marks the loop scalar |

**What blocks it, in order of how often:**

1. **Overflow checks** in the loop body (section 4).
2. **Push slow paths.** A counted `for i in a..b` loop that pushes gets one `kv_list_reserve` and plain stores (`raw_push_targets`, `stmt.rs:130-196`). A `for x in xs` loop that pushes does not: each round checks the count and capacity and may call `kv_rt`. The same reservation works there too, since the list's length is known before the loop.
3. **The target.** Plain x86-64 means SSE2: two `i64` or two `f64` per instruction, and no vector 64-bit multiply. AVX2 doubles the width. AVX-512 adds a 64-bit multiply. The to-do records that targeting AVX2 made `loop` 40% slower once. That points to choosing per loop, not one global switch (section 7).
4. **`%` loops marked scalar.** This is right on SSE2, but the mark should depend on the target and on whether `%` is by a power of two.
5. **Pointers as integers.** `inttoptr` from `i64` leaves LLVM's alias analysis with only TBAA. Where two lists of the same item type are read and written in one loop, LLVM must add run-time overlap checks or give up. Kavach values never alias, which spec 6.1 and the to-do count as Kavach's edge over C++. LLVM can't use that edge until the IR says it: `noalias` on a function's list parameters, or `!alias.scope` metadata on each list's loads and stores.
6. **Floating-point sums** stay in order by design (the to-do: "floating-point sums keep their order unless the program asks"). Element-wise float loops are free to vectorize, and do.

**Benefit (not measured).** Element-wise integer and float loops already vectorize where nothing blocks them. Integer sums, `map`s with arithmetic and fused chains don't, and they are most of `lists`, `math` and `chains`. Fixing 1 and 2 is the precondition for the "math 20× C++" target in step 2.

**Tests.**
- An IR check per row of the table: the vectorized rows keep their `<2 x …>`, and each fixed row gains one.
- Benchmark rows: `math` `sum` and `scale` on one core, `chains` `datachain`.

## 6. Binary size and start-up

**Measured** (Windows, release 0.2.0, `kavach build`; 1 KB = 1,000 bytes):

| Program | Size | Runtime entries it calls |
|---|---|---|
| `print("hi")` | 540 KB | `kv_main`, `kv_print_str`, `kv_str_new`, `kv_rt_version`, `kv_set_functions` |
| records probe (no list) | 571 KB | + `kv_record`, `kv_free`, `kv_overflow` |
| `xs.push(i % 13)` in a loop, then a sum | 1,300 KB | + `kv_list`, `kv_list_reserve`, **`kv_rt`** |

Decision 0024 sets "hello world" under 300 KB for release builds. The repository's test allows 788 KB for the development runtime (`tools/cli/tests/size.rs`); no test checks a release build. The runtime archive itself is 295 MB in the install (`lib/kavach_runtime_native.lib`). Linking it is the measured part of compile time (compiler review, section 4).

**Findings.**

1. **`kv_rt` is the core dispatcher, and one use links all of it.** Ops with no feature group go to `kv_rt` (`kir/src/rt_op/groups.rs:131-200`), one `match` over every core op in `runtime/native/src/rt_data.rs:179` (text methods with Unicode case mapping among them). A list `push` that may need to grow calls `kv_rt(88, …)`, and the program grows by about 760 KB. This is the same cause decision 0024 found at 4.6 MB, one level down: the core group is still one entry. Splitting it by family (lists, maps, text, numbers) follows the pattern that already exists.
2. **What makes up the 540 KB** was not broken down (no symbol tools in the install). The candidates from reading: Rust's standard library pieces the runtime uses (formatting, threads, `std::env`), the shape decoder, the crash-report table, and the signal handling.
3. **`runtime/native` depends on `compiler/kir`.** The compiler review (R4) found the front end inside the runtime archive. Whether any of it reaches an executable was not measured. The linker drops unreferenced code, but `kv_main` calls `kavach_kir::decode_shapes` (`runtime/native/src/lib.rs:291`), so at least that path is in every program.

**Start-up, before `main` runs (from reading `runtime/native/src/lib.rs:274-310` and the generated `@main`):**

1. `kv_set_functions` parses the table of every function's name and line into a `Vec<(String, u32)>` (`crash_stack.rs:30-41`): one allocation per function, used only when a crash report is printed.
2. `kv_main` installs the stop handlers. On Linux that starts a thread that waits for signals (`rt_stop.rs:84-107`).
3. It copies the arguments into `String`s and reads the program's path.
4. It parses the shape table from text (`decode_shapes`).
5. It starts a new thread with a 512 MB stack (`STACK_SIZE`, `lib.rs:165`) and waits for it. Only address space is reserved, not memory.
6. The thread reads `KAVACH_CHECK_LEAKS`.
7. The program's `main` makes a text object of its version marker and calls `kv_rt_version`, which checks that the op table matches the runtime and answers `--version`.

None of this is large. Items 1 and 4 grow with the program. Item 1 can wait until a crash. Item 4 could be a table in binary form, written by the compiler and read in place. Start-up time was not measured; `hyperfine` on hello world against C's would show it.

**Tests.**
- A release-mode size test (or a release CI step) that holds hello world to decision 0024's 300 KB.
- A second budget for "a list push in a loop": after splitting `kv_rt`, about hello world plus the list code.
- A start-up row in `benchmarks/everyday`: hello world against C and Rust, measured with `hyperfine`.

## 7. What each backend asks of its code generator

| | LLVM (`kavach build`) | Cranelift (`kavach run --native`; `kavach build` fallback) |
|---|---|---|
| Optimizer | `opt -O3` (`lib.rs:166`) | `opt_level = speed` (`cranelift/src/lib.rs:148`) |
| Code generator | `llc -O3 -filetype=obj`, `-relocation-model=pic` off Windows (`lib.rs:171-176`) | Cranelift |
| CPU | **plain x86-64 (SSE2)**: no `-mcpu`, no `-mattr`, no `target-cpu` attribute; only `target triple` (`module.rs:154`) | **the build machine's** (`cranelift_native::builder()`, `lib.rs:152`), also for object files (`object.rs:23`, `library.rs:35`) |
| Inlining | KIR's, then LLVM's | KIR's only |
| Loops | bounds-check versioning, push reservation, held-steady buffers (`stmt.rs`, `lists.rs`) | none of these |
| `Fallback` nodes | runs the fast half | runs the code as written (compiler review, section 1) |
| Runtime declarations | no attributes, except `noreturn cold` on panics and two `Bytes` reads (`module.rs:64-81`) | n/a |
| Calls | `notail` everywhere, so a recursion never becomes an endless loop (spec 3.8) | plain |

**Findings.**

1. **The two backends target different CPUs.** An executable built through Cranelift (when LLVM can't build the program, or its tools are missing) may use AVX2 or newer if the build machine has them, and crash with an illegal instruction on an older one (from reading; not run on an older CPU). The JIT may keep host features. Object files should use a fixed baseline, the same as LLVM's.
2. **LLVM's baseline is the safe default**, but it halves vector width and rules out 64-bit vector multiplies. Options, in order of cost:
   - a `[build] cpu` setting (`x86-64-v2`, `-v3`, `native`) passed as `-mcpu` to `opt` and `llc`;
   - function multiversioning for loops the vectorizer widened: a baseline copy and an AVX2 copy, chosen once at start-up. That keeps one portable executable, but it doubles hot code.

   The 40% slowdown recorded for AVX2 on `loop` should be re-checked with the `%`-scalar mark in place before choosing.
3. **The runtime can't be inlined.** The runtime is Rust; the program is LLVM IR. The install's `opt` is LLVM 23.1.1 from Rust 1.99.1, the same LLVM that builds the runtime. So hot runtime functions (map get and insert, text compare and hash, list growth) could ship as bitcode, be joined to the program with `llvm-link`, and be inlined by `opt`. C++ gets this for `std::unordered_map::find` and `std::vector::push_back`. The to-do already asks for "map lookups with integer keys inlined into native code"; this would give it for every runtime function at once.
4. **The stack check before each call** (`func.rs:369-389`) is a read of `rsp`, a thread-local load and a branch. It is needed only on calls that can recurse: calls inside a cycle of the call graph, and calls through function values. A chain of calls that can't recurse is bounded by the program's size. The runtime already keeps 8 MB below the limit for itself (`STACK_RESERVE`, `runtime/native/src/rt.rs:68`). The check also survives LLVM's inlining (confirmed in section 1's probe: `kv_stack_overflow` is still in the loop), because it sits at the call site, not in the callee.

## 8. A way to test optimizations: IR checks

Most "tests that show it done" above are checks on the IR after `opt -O3`. `backend/llvm/src/pointers.rs:45-100` already has one: it compiles a small program, runs `opt -O3 -S`, and checks what is left. One shared harness would make each optimization one small file and one line:

- a folder of tiny programs, each with lines like `// ir: no kv_record` or `// ir: has <2 x i64>`;
- the harness builds each one, runs `opt -O3 -S`, and checks the lines;
- it skips, saying so, when LLVM's tools are missing (as `pointers.rs` does).

The probes of this review are the first cases. Each check stays next to the differential test (step 1, item 7): the IR check shows the optimization happens, and the differential test shows it changes nothing a program does.

## Optimizations worth doing

Ranked by benefit for its cost. "6.1 risk" is the risk of breaking spec 6.1's "No optimization changes what a program does".

| # | What | Benefit | Cost | 6.1 risk | Shown done by |
|---|---|---|---|---|---|
| 1 | **Inline callees first** (call-graph order), so `a → b → c` flattens; inline single-call-site functions up to a larger size | high: allocations and calls gone wherever helpers nest | small (`inline.rs`) | low: inlining keeps arguments' order and lines today | `inline/tests.rs`; IR check on the chain probe; `everyday` rows |
| 2 | **Borrow the base of an index** whenever the index can't set, move or pass `inout` that local (`heap.rs:277`, and Cranelift's twin) | medium-high: 2 count updates and a branch per read | small | low, with the rule as stated | IR check on `weigh`; `lookups` row |
| 3 | **Reserve before a `for x in xs` loop that pushes**, as counted loops do | medium: hand-written `map` loops vectorize | small (`stmt.rs`) | none (the same checks, once) | IR check: `kv_list_reserve` and `<2 x double>` |
| 4 | **Text constants made once per function**, not per read in a loop (hoisted to a local; reads retain it) | medium: an allocation per round in loops with labels or keys | small (KIR pass) | none: text is a value, changes copy | IR check: no `kv_str_new` in the loop |
| 5 | **Fixed baseline CPU for Cranelift object files**; host features only for the JIT | portability, not speed | small (`cranelift/src/object.rs`, `library.rs`) | none | an object built on a newer machine has no AVX instructions (check with `llvm-objdump`) |
| 6 | **Split `kv_rt` by family** (lists, maps, text, numbers) | about 760 KB off every program that pushes | medium (shared files `rt_data.rs`, `rt_op/groups.rs`) | none | a second size budget; a release size test |
| 7 | **`sroa` for records chosen by `if`/`match`, and for records with heap fields** | medium: allocations in "best so far" loops and records with text | medium (`sroa.rs`, `sroa/rewrite.rs`) | low-medium: heap fields need correct retains on rebuild; `Drop` stays excluded | `sroa/tests.rs`; chapter 6 copy test per form |
| 8 | **Checked sums in blocks** (vector add with a bounds test, the checked loop as fallback) | high for sums and fused chains | medium (KIR pass on `Fallback`, LLVM backend) | medium: must panic at the same item; the fallback re-runs the block from its start | IR check `<2 x i64>`; overflow-mid-block conformance test; `math` `sum` 1-core row |
| 9 | **Attributes on runtime declarations** where they are true (`willreturn`, `memory(...)`; for allocators `allockind` and `noalias`, after #10) | medium: LLVM deletes dead allocations and keeps values in registers across calls | small per function, but each must be proven | medium: a wrong attribute is a silent miscompile | IR check: the chain probe's `kv_record` disappears once its result is unused |
| 10 | **Pointers as `ptr`, not `i64`**, with `noalias` or alias scopes for lists | medium: better alias analysis and vectorization | large (the whole LLVM backend) | low | vectorizer remarks on two-list loops |
| 11 | **Lazy crash-name table; binary shape table** | small, and grows with program size | small | none | a start-up row in `everyday` |
| 12 | **Pair-cancel retain/release** on one local with nothing between | small-medium | small | low | IR check |

## Needs a decision record

1. **Borrowed parameters** (section 3, gap 2): a new calling convention for every function, both backends and the runtime's callers (task start, `drop`). It removes a count pair per heap argument per call. It also sets the rule that a borrowed value read whole is retained, which spec 6.1 depends on.
2. **Stack checks only where recursion is possible** (section 7, finding 4), or a guard page instead (the to-do already names that as a runtime design for a maintainer). It changes how spec 3.8's panic is guaranteed; the proof (non-recursive chains bounded, 8 MB reserve) belongs in a record. Task and coroutine stacks need the same proof.
3. **The CPU target** (section 7, finding 2): a `[build] cpu` setting, multiversioned loops, or both, and what the default is. It decides whether a Kavach executable runs on every x86-64 machine.
4. **Runtime as bitcode for inlining** (section 7, finding 3): ships bitcode with the release, ties the runtime's Rust version to the shipped LLVM tools, and changes the build. It gives C++-like inlining of maps, text and list growth.
5. **Returning small records in registers** (section 2, gap 3): a change to how functions return values, in both backends.
6. **Checked sums in blocks** (item 8 above) may fit under decision 0031's prefix bounds. If it is read as new semantics for when an overflow is found, it needs its own record.

## What this review could not check

- **No speed was measured.** Every benefit is an estimate from the IR. The benchmark rows named above are where to measure each change.
- Probes ran only on Windows with the release build; Linux IR may differ in calling convention details, but not in the findings.
- What makes up hello world's 540 KB, and whether front-end code reaches executables through `runtime/native → kir`, needs a symbol listing (`llvm-nm --size-sort`).
- Cranelift-built executables were not run on an older CPU; finding 7.1 is from reading.
- Start-up time was not measured; section 6 lists the work, not its cost.
