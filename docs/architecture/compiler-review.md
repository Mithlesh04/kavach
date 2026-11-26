# Compiler review

A review of the compiler stages: step 1, item 3 of the [to-do](../vision/todo.md) (foundation 1). It checks each stage for one clear job and clean boundaries, finds where the interpreter and native code do the same work twice, and lists what to change.

**How it was made (2026-11-26, at commit `685d352c0`).** Read only: source files, READMEs, `Cargo.toml` files and `git log`. Counts come from `git ls-files`, `wc -l` and `grep`. Nothing was built, run or timed, so every claim about speed or memory below is marked as not measured.

## Summary

- **The front end is clean; the boundary after it is not.** Lexer, parser, resolver and checker each have one job. But the checker hands on its results as about 30 side tables keyed by source span (`driver/src/lib.rs:85-146`), and KIR lowering and the interpreter each work out the rest again: which method a call is, which methods change their receiver, the order operands run in.
- **The interpreter and native code are two full implementations.** About 15,500 of the interpreter's 18,000 lines do work that native code does through KIR lowering (22,800 lines) and `runtime/native` (21,700 lines). Since the history reset (2026-11-10), 100 of the 182 commits that touched the compiler or `runtime/native` had to change both sides.
- **Native code has two more copies.** Cranelift and LLVM each generate code for every KIR node; 12 of 48 backend commits changed both.
- **Compile-error tests check less than they say.** 624 of the 630 compile-error tests mark the expected line and text with `// error: …`. The test runner checks only that compiling fails. The checker's `spec_sync` checks the line, not the text and not the number of messages. "One mistake, one message" is not tested.
- **Nothing is cached between compiles.** Every compile joins the program and the standard library modules it uses into one string and checks it all again. Spans are offsets into that string, so one edit moves every later span, and nothing keyed by span can be reused.
- **A surprising dependency edge:** `runtime/native` (the library linked into every executable) depends on `compiler/kir`, so it pulls in the checker, driver, parser, `toml` and `serde`. Every checker edit rebuilds the runtime archive.
- **KIR lowering reads `kavach.toml` from disk** in five places, and one pass asks the driver for a setting. Lowering is not a pure function of the checked program.
- **The pass pipeline is ordered by comments, not by checks.** The riskiest coupling: `moves.rs` must follow the order in which each backend evaluates operands (`moves.rs:13`), and that order is written down nowhere in KIR. `moves.rs` and `exits.rs` are the only passes with no unit tests.
- **`KAVACH_PASSES=none` is easy to add** in `passes.rs`, but 12 of the 32 steps there are needed for correctness. Some optimizations happen during lowering, so the switch won't turn them off.
- **The docs have drifted.** `overview.md` names four runtime folders that hold only `.gitkeep`, leaves the interpreter out of the pipeline, and says the driver runs every stage (it stops after checking). About 100 compiler source files are missing from their README tables.

## 1. Stages and boundaries

### Size of each stage

| Crate | `.rs` files | Lines | Job |
|---|---|---|---|
| `lexer` | 9 | 1,847 | text → tokens |
| `ast` | 7 | 1,476 | syntax tree types |
| `parser` | 21 | 5,030 | tokens → tree |
| `resolver` | 14 | 3,277 | names → definitions |
| `checker` | 115 | 23,284 | types, capabilities, contracts, secrets |
| `driver` | 28 | 5,622 | front-end pipeline, `kavach.toml`, packages, rewrites after checking |
| `interp` | 96 | 18,029 | runs the checked tree |
| `kir` | 168 | 36,832 | IR, lowering, 30 passes, runtime op table |
| `backend/cranelift` | 26 | 5,847 | KIR → machine code, JIT, **linking** |
| `backend/llvm` | 22 | 5,008 | KIR → LLVM IR text → `opt`, `llc` |
| `backend/wgsl` | 15 | 2,700 | `gpu fn` → WGSL |
| **Total** | **521** | **~108,950** | |

Line counts include each crate's tests.

### Dependency graph (from the `Cargo.toml` files)

```
lexer ← ast ← parser
        ast ← resolver ← checker
driver  → lexer, ast, parser, resolver, checker, runtime-core, toml, serde
interp  → driver, checker, resolver, ast, runtime-core + 11 runtime crates
kir     → driver, checker, resolver, ast, runtime-core
runtime-native    → kir, runtime-core + 12 runtime crates
backend-cranelift → kir, runtime-native
backend-llvm      → kir, runtime-native, backend-cranelift
backend-wgsl      → kir
tools/cli         → all of the above
```

Cargo forbids cycles, so there are none. Four edges are surprising:

1. **`runtime/native` → `compiler/kir`.** The runtime uses `RtOp`, `Shape`, `Type`, `packed_size`, `decode_shapes` and `borrows` from KIR (about 100 imports). It is built as a `staticlib`, which bundles every crate it depends on. So the front end and its libraries go into the runtime archive. The linker reads that whole archive for every `kavach build`, and the archive has grown from 74 MB to 190 MB (to-do, "Builds under 0.2 s"). How much of that is the front end has not been measured.
2. **`backend-llvm` → `backend-cranelift`.** LLVM uses Cranelift's crate only for `link`, `link_library`, `build_folder` and `Exe`. Linking is a job of its own, not Cranelift's.
3. **`kir` → `driver`.** This edge is expected for `Compiled`. But KIR also calls `kavach_driver::Config::for_file` to read `kavach.toml` (`lower_agents.rs:337`, `lower_drops.rs:35`, `lower_log.rs:37, 102, 123`), and `passes.rs:45` calls `kavach_driver::watch_nan(path)`.
4. **`driver` → `runtime-core`.** `Config` returns runtime types (`log::Settings`, `json::Tree`). This is small, but it means the front end links runtime code.

### Findings, stage by stage

- **Lexer, parser, resolver.** Each has one job. Parser recovery is simple and clear (section 3). The resolver names `std.` only to treat the standard library's modules specially (5 places). These stages are fine.
- **Checker.** The checker decides a lot, but records only part of it: the types, by span (`expr_types: HashMap<(u32, u32), Ty>`), plus about 25 other tables. It does not record which method a call resolves to, where a value is wrapped in an optional or boxed into an interface, or the order operands run in. Later stages work these out again (section 2).
- **Driver.** The driver does three jobs in one crate:
  - **pipeline:** `lib.rs`, `std_modules.rs`, `std_table.rs`, `codes.rs`, `limits.rs`;
  - **packages and config:** `deps`, `git_fetch`, `lock_file`, `workspace`, `grants`, `fetch_checks`, `config`, `run_config`, `agent_config` (about 1,800 lines);
  - **rewriting the tree.** `variants::name_variants` runs before the resolver. `project::unqualify` runs between the resolver and the checker. After the checker, `operators`, `indexing`, `tensors`, `tables` and `frozen` change both the tree and the checker's tables (`lib.rs:293-311`).

  These rewrites are a desugaring stage. They belong in a named step, not in the middle of the orchestration code.
- **The pipeline doesn't end in the driver.** `overview.md` says the driver "runs all stages". It runs only the front end. The CLI then calls `kavach_interp::check_literals`, which runs Kavach code at compile time (`tools/cli/src/main.rs:320`), lowers to KIR, and chooses the backend (`tools/cli/src/native.rs:156-168`). The conformance runner repeats those steps by hand (`tools/cli/tests/conformance.rs:91-95`). So the CLI, the language server and the tests can drift apart. `Compiled.ai_prices` is a `OnceLock` that the interpreter fills in later (`driver/src/lib.rs:134`). The driver's output is not final when the driver returns it.
- **Missing types.** When a type is missing, `Lowerer::ty_of` gives `Type::Unit`, with no error (`kir/src/lower_expr.rs:17-20`). An expression the checker skipped, or a rewritten node whose span changed, becomes `Unit` without a word. Spans can also collide: "a dot lambda and its body share a span" (`lower_expr.rs:28-29`).
- **The standard library is done twice.** 65 modules are written in Kavach (13,022 lines in `stdlib/`). The rest are built into Rust, and each one is spelled out in four or five places. For example, `fs.read_text` appears in `checker/src/stdlib.rs:145` (its signature), `checker/src/std_caps.rs:60` (its capability), `interp/src/std_lib.rs:163` (behaviour), `interp/src/fakes.rs:157` (the fake used in tests) and `kir/src/std_ops.rs:14` (its op). The Rust side of the standard library is 1,775 lines in the checker, 2,515 in the interpreter, and its own `lower_*.rs` files in KIR.
- **Backends.** Cranelift and LLVM both read KIR and call the same runtime entries; that boundary is clean. LLVM falls back to Cranelift for programs it doesn't support. Only LLVM runs the fast half of a `Fallback` node (`kir/README.md`), so the two backends run different code for the same KIR.
- **The interpreter already uses KIR in one case.** For GPU pipelines, the CLI lowers the whole program to KIR to get WGSL (`tools/cli/src/native.rs:190-205`). It decides whether to do this by searching the source text: `c.src.contains("gpu.pipeline")`.

## 2. The interpreter and native code

### What already agrees by construction

Both sides call `runtime/core` (11,800 lines) for JSON parsing and writing, text methods, `Decimal`, regular expressions, float formatting, the task scheduler, agent bookkeeping and the journal. Both use the checker's `grouped_sum` and `json_literal_kind`. Both read the checker's types. This is the right direction.

### What is done twice (lines)

| Area | Interpreter | KIR lowering | `runtime/native` | Shared in `runtime/core` |
|---|---|---|---|---|
| Printing values (spec 1.11) | `value.rs` 476 (part) | – | 305 | float formatting only |
| JSON, `json.decode<T>` | 1,156 | 1,299 | 2,786 | 1,499 (parse, write, read) |
| Operators and overflow | 613 | 708 | – (plus 342 Cranelift, 329 LLVM) | `int_pow`, `Decimal` |
| Agents, guard, `ask`, journal | 1,351 | 1,919 | 1,352 | 2,490 |
| Tasks, channels, cancel | 1,321 | 2,622 | 1,685 | scheduler |
| `Drop` | 130 | 568 | 214 | – |

The interpreter has 18,029 lines. About 2,400 of them exist only in the interpreter: generated tests, fakes and hot reload. The other ~15,500 are a second implementation of what native code does. Native code itself has two code generators (10,855 lines).

### Where they drift

Since the history reset, 100 of 182 commits that touched `compiler/` or `runtime/native` changed both the interpreter and the native side (9 changed only the interpreter, 43 only the native side). Many of these add a feature to both sides on purpose. But several are drift fixes, where one side had it wrong:

| Commit | What differed |
|---|---|
| `9717d397e` | **Order of evaluation** (spec 3.10): the interpreter worked out a method's arguments before its receiver; native code worked out named arguments and record fields in their declared order. 9 interpreter files and 8 KIR files changed. |
| `04139d930` | **Which methods change their receiver**: native code's list missed `pop` and the text builder's methods, so a lambda's `pop` changed the outside variable (a spec 6.1 break). |
| `eef56b5e3` | Native code lost a call's error when an argument held a `try` deeper inside. |
| `c4a39b00a`, `f8ccc2105`, `15d441562` | `json.decode` into bit and range numbers, generic parameters, `Float32` defaults: fixed on one side at a time. |
| `8e7e987c8` | Invariants of generic records were not checked in native code. |

"Which methods change their receiver" is still written three times, and the three lists differ:

- `checker/src/methods.rs:415`, `changes_in_place`: by type; `pub(crate)`, so the other stages can't use it;
- `interp/src/receivers.rs:20`, `MUTATING`: 8 names, with `pop` handled separately;
- `kir/src/lower_lambda/scan.rs:11`, `CHANGING`: 9 names, with no types.

The cross-check that catches drift, `native_code_matches_the_interpreter`, still asserts only `supported.len() >= 85` (`tools/cli/tests/native.rs:219`). About 1,470 conformance tests and examples now run. A gap in lowering already fails the test (line 85), so the floor protects little. It compares stdout, the exit code and the first panic line, but not the rest of stderr.

### Options for "one shared IR, so they agree by construction"

**A. Share the leaves.** Move each leaf operation into one function both sides call: printing values (`runtime/native/display.rs` walks shapes; the interpreter walks `Value`s), JSON decode plans (built once from the checker's type, used by both), receiver-changing methods, overflow helpers. Add one built-in catalog (rewrite R2).
- *Cost:* 3–6 weeks, area by area.
- *Risk:* low.
- *Limit:* it doesn't fix drift in control flow or evaluation order.

**B. The checker hands on its decisions.** The checker gives an elaborated tree with node ids. Each node records its method target, the coercions it needs, and the order its operands run in, as temporaries. The driver's rewrites move into this step. The interpreter and KIR lowering both read it instead of working things out again.
- *Cost:* 2–3 months. It touches the checker's output and every reader of `expr_types` (118 references), the interpreter and lowering.
- *Risk:* medium.
- *What it gives:* evaluation order is decided in one place. It also ends span keys, which incremental compilation needs (section 4).

**C. The interpreter runs KIR.** Lower with passes off, and carry out each op with `runtime/native`'s entries (or JIT at a low optimization level).
- *What it gives:* it removes most of the 15,500 lines.
- *Cost:* 2–4 months.
- *Risk:* high.
  - Generated-test fakes, hot reload, test mode, panic places and compile-time literal checks would all have to be rebuilt on KIR.
  - `kavach run` would pay for whole-program lowering before it starts (not measured).
  - The interpreter would stop being an independent oracle: a lowering bug would show on both sides. The hand-written `// expect output:` lines in about 1,470 tests stay an oracle. The passes-off/passes-on differential test (step 1, item 7) would become the native side's main cross-check.
- The browser playground only checks programs (it depends on `driver` and `runtime-core`), so it doesn't constrain this choice.

**Recommendation:** do A now; it pays at once and fits B later. Write a decision record for B. Decide on C only after B and the differential test exist.

## 3. Error recovery: does one mistake give one message?

What exists:

- **Parser:** `recover_line` (`parser/src/parser.rs:281`) skips to the end of the line, or to the `}` that closes the block, keeping bracket depth. It has 8 call sites. One test checks that two mistakes give two errors and that parsing goes on (`parser/tests/syntax_errors.rs:156-160`).
- **Driver:** when the program's own files have syntax errors, only those are shown (`driver/src/lib.rs:400-405`, spec 13.6). Errors in standard library files are hidden when the program has its own.
- **Checker:** `Ty::Error` "fits anywhere, so one mistake is reported once" (`checker/src/ty.rs:133-135`). There are local fixes such as `checker/src/items.rs:194`.

What is missing:

- **Most compile-error tests don't check the message.** The conformance runner checks only `has_errors()` for the 630 compile-error tests (`conformance.rs:96-99`). A test passes even if it fails for the wrong reason, such as a typo that makes a syntax error.
- **The checker's `spec_sync` checks only the line.** It requires every error to be on the first `// error:` line (`checker/tests/spec_sync.rs:60-74`). It doesn't check how many errors there are, so two errors on the marked line pass. It doesn't compare the text after `// error:`. And it skips project folders (lines 14-16).
- **Nothing removes duplicate messages.** The resolver's and the checker's messages are joined and sorted (`lib.rs:330-406`), with no step that drops a repeat. Whether repeats happen today is not measured.
- **Message codes come from wording.** `codes.rs` picks a message's stable code (spec 13.6) by looking for words in its text, first match winning. Rewording a message can change its code without anyone noticing. `driver/tests/codes.rs` checks only that the first error of each test has some known code.

So "one mistake, one message" is followed in the code, but no test holds it in place. The cheapest strong check is to use the markers the tests already carry (cheap fix 2).

## 4. Incremental compilation and compile speed

**What is cached today.** Nothing between runs. Within one process:

- `std_modules.rs` remembers which standard library modules a text names (capped at 64 MB), for the language server.
- The language server's quick check drops other files' bodies (spec 13.12).
- The LLVM backend remembers where LLVM's tools are, in a temp file.

**What each compile does.** `compile_sources` joins the program's files and each standard library module it uses into one string (`lib.rs:249-288`). It then parses, resolves and checks that string from the start. The Kavach-written standard library is 13,022 lines across 65 modules, and a program that uses `std.http` re-parses its modules on every compile. The conformance suite compiles about 2,100 files once in the runner, again in each of the four `spec_sync` tests (lexer, parser, resolver, checker), and again in the native cross-check.

**Where time goes.** Only linking has been measured. `kavach build` of hello world took about 0.26 s, of which linking was about 0.25 s (`benchmarks/everyday/README.md`). On Windows with the 190 MB archive it took 0.74–0.77 s. No stage reports its own time, so front-end, lowering, pass and code generation times are not known. Memory is not measured either.

**What stands in the way of recompiling only what changed:**

1. **Spans are offsets into one joined string.** Every side table is keyed by `(start, end)`. An edit near the top moves every key below it. Nothing can be reused until spans are relative to their file, or nodes have stable ids (rewrite R1).
2. **Lowering has hidden inputs.** It reads `kavach.toml` itself (section 1), so a cache key would have to include files the cache doesn't know it reads.
3. **The whole program is one unit.** Monomorphization, inlining, `prune` and `par` work on the whole program. Caching at the KIR level would have to work per function: hash a function's KIR together with its callees' signatures, and keep its object code. Cranelift has a function-level cache (its `incremental-cache` feature) that fits this.
4. **The runtime archive is large.** It is the one measured cost. Cutting the `runtime/native → kir` edge (rewrite R4), and the ideas already in the to-do (a prebuilt runtime object, a smaller archive), come first.

**Order of work:**

1. Add a per-stage timer (cheap fix 9) and measure.
2. Parse and resolve the standard library modules once per process. They are constants, so this needs no invalidation. The test suites and the language server gain most.
3. File-relative spans and node ids (with R1).
4. Check each module against the interface summaries of the modules it uses, cached in `.kavach/` by content hash.
5. Per-function object caching in the backends.

## 5. Size and structure

- **Files pile up against the limit.** 37 files have 450–499 lines, but only 19 have 400–449. Files grow until the limit forces a split. Near the line: `checker/src/items.rs` (500), `driver/src/config.rs` (499), `checker/src/calls.rs` (499), `resolver/src/modules.rs` (498), `backend/cranelift/src/codegen.rs` (498), `checker/src/patterns.rs` (497), `checker/src/stdlib.rs` (497), `kir/src/lower_agents.rs` (497). Splitting a file before it is full, as `3e894a42b` did, is the better habit.
- **`kir/src` is flat:** 128 `.rs` files in one folder, where AGENTS.md says to nest past about 10. It mixes four kinds of file: the IR (`ir.rs`, `types.rs`, `rt_op*`), ABI (`c_abi.rs`, `list_layout.rs`, `encode.rs`), lowering (81 `lower_*.rs` files, 22,800 lines with their helpers) and passes (about 30 files, 13,700 lines).
- **Lowering files are split three ways at once:**
  - by syntax: `lower_if`, `lower_match`, `lower_stmt`, `lower_expr`;
  - by spec feature: `lower_whole`, `lower_sizes`, `lower_frozen`, `lower_iface`;
  - by standard library module: `lower_base64`, `lower_tls`, `lower_log`, `lower_cli`, `lower_selected` (crypto).

  When a file filled up, the overflow went to a file with a suffix: `lower_json` → `_checks`, `_texts`, `_wholes`; `lower_fn` → `lower_fn_errors`; `lower_ptr` → `lower_ptr_syntax`; `lower_bytes` → `lower_bytes_text`; `lower_whole` → `lower_whole_generic`. Some lowering files have no `lower_` prefix at all: `grouped_sums.rs`, `distinct.rs`, `invariant_checks.rs`, `collect.rs`. Most splits follow a responsibility, but the file name doesn't tell lowering from passes.
- **One struct per stage, with methods spread across about 90 files.** `Lowerer` has 45 fields and `impl` blocks in 95 files (about 515 functions). `Checker` has `impl` blocks in 96 files, `Interp` in 86. Splitting into files gives no encapsulation: any file can change any field, so the real boundaries inside a stage are not visible in the code.
- **README tables have drifted.** These source files are not named in their crate's README: 35 of 154 in `kir`, 42 of 108 in `checker`, 20 of 95 in `interp`. The KIR README's prose on pass order leaves out 10 of the passes that `passes.rs` runs.
- **Suggested shape:** split `kir` into crates: `kir` (IR, `RtOp`, `Shape`, ABI, layout; what the runtime and backends need), `lower` and `passes`. Separate crates also make Rust's own rebuilds smaller. Nesting under `kir/src/lower/` would also work, but it reaches the 4-level depth limit and leaves no room to group further.

## 6. The passes pipeline (`kir/src/passes.rs`)

### Order today, and which steps are required

"Required" means that without the step the program behaves differently or leaks. "Optional" means it only makes the program faster.

| # | Step (line) | Kind | Must come after / before |
|---|---|---|---|
| 1 | `scope_files`, `scope_blocks`, `join_tasks` (36-38) | required (8.1, 9.2) | adds code; runs first |
| 2 | `caller_lines` (42) | required (5.6) | before `inline`, so inlined standard library code keeps the call's line |
| 3 | `nan_watch` (48), when on | required (5.9) | before `inline` and the loop passes; never in a `gpu fn` |
| 4 | `unrecurse` (53) | optional | before `inline` |
| 5 | `inline` (55) | optional | before `prune` and `stop_points` |
| 6 | `inout_stash` (64) | optional | needs every caller rewritten too (whole program) |
| 7 | `prune` (65) | keep on (decision 0024) | after `inline` |
| 8 | `drop_scopes` (89-94) | required (6.1) | before `sroa` and `moves` (comment at 66) |
| 9 | `configure_log` (96) | required (14.12) | lowering work done inside the pass list |
| 10 | `stop_points`, `cancel` with `whole_lines` (97-98) | required (9.6, 9.8) | after `inline` |
| 11 | `checked_fits`, `gpu_stay`, `gpu_later`, `devirt`, `unused`, `tuple_lets`, `loop_fields`, `sroa`, `coalesce`, `loop_lens`, `jam`, `fuse`, `version`, `ranges`, `prefetch` (101-115) | optional | `devirt` before `sroa` (else no match); `version` before `ranges`; `fuse` and `jam` before `moves` |
| 12 | `exits` (117) | required (no leaks, 6.1) | after `coalesce` (comment at 116) |
| 13 | `moves` (118) | keep on (see below) | after everything that copies or reorders code, except `par` |
| 14 | `par::outline`, then `ranges` on new functions (120-123) | optional | after `moves` |
| 15 | `counters` (125) | optional | after `ranges` (needs its proven ops) and `par` |

### Reorderings that could break the value rules (spec 6.1)

1. **`moves` before a pass that copies or reorders code.** A `Move` empties a variable. If `fuse`, `version`, `jam`, `unrecurse` or `inline` ran after `moves`, a copied read could come after the move and see an emptied variable: two names sharing a value, or a crash.
2. **`moves` and the backends must agree on evaluation order** (`moves.rs:13`). The order of spec 3.10 is written separately in the interpreter's `eval`, in KIR lowering, in `moves`' liveness walk and in each backend's code generator. Commit `9717d397e` shows these did differ. KIR doesn't make the order explicit, and no test pins it for `moves`. This is the highest-risk coupling in the pipeline.
3. **`par::outline` is the one pass after `moves` that copies loop bodies:** the chunk function, plus the loop as written behind it. It re-runs `ranges` on the new functions but not `moves` (`passes.rs:120-123`). It is safe today only because a variable read in a loop is live around the loop, so it has no `Move` inside. Any change to `par` that reads a local after a moved use would break 6.1 without a test catching it.
4. **`sroa` before `drop_scopes`** would split a `Drop` record into locals, and its `drop` would never run. This is guarded by passing `holders` to `sroa`.
5. **`exits` before `coalesce`** would leak (the comment at 116). The leak check (`KAVACH_CHECK_LEAKS`) would catch it.

`moves.rs` (275 lines) and `exits.rs` (171 lines) are the only passes with no unit tests.

### A "passes off" switch (`KAVACH_PASSES=none`, step 1 item 7)

It is feasible, and the place for it is `passes::finish`:

- Read the switch once in `lower_with` (`kir/src/lower.rs`) into a small `PassSet`, and hand it to `finish`. Don't read the environment inside a pass, as `sroa.rs:31-38` does now for each function. Fold `KAVACH_NO_SROA` and `KAVACH_SROA_ONLY` into it, for example `KAVACH_PASSES=none`, `all`, or `-sroa,-fuse`.
- With `none`, skip the steps marked optional above. Keep the required ones, and keep `prune` and `moves`. Without `moves`, the program stays correct but copies whole lists in loops: the 10-second limits in `native.rs` would fail. Make turning `moves` off a separate choice.

Some optimizations happen outside `passes.rs`, so the switch won't turn them off:

- lambdas inlined into list methods (`lower_lambda.rs`);
- sums per key (`grouped_sums.rs`);
- one-lookup `??` (`lower_optional.rs`);
- LLVM's own `opt -O3`;
- the fast half of `Fallback` nodes, which LLVM runs and Cranelift doesn't.

The differential test should list these, and either add lowering switches for them or compare Cranelift and LLVM builds as well.

Inlining and `unrecurse` change frame sizes, so a program near the stack limit can overflow with passes off and not with them on. The test should compare such programs as `native.rs:44-46` already does: same panic, line ignored.

## Rewrites worth a decision record

**R1. An elaborated tree from the checker.**
- *What:* nodes with stable ids. Each records its method target, its coercions, its evaluation order (as temporaries) and its desugarings. The driver's rewrites move here.
- *Why:* it ends working things out again in the interpreter and in lowering, ends span keys and the silent `Unit`, and fixes evaluation order in one place.
- *Cost:* 2–3 months.
- *Risk:* medium, because every reader of `expr_types` changes.
- *Unblocks:* option B, incremental compilation, language server features, and option C later.

**R2. One built-in catalog.**
- *What:* one declarative list, like `kir/src/rt_op/ops/*.ops`. For each built-in method and standard library function it gives the name, receiver type, signature, whether it changes the receiver, its capability, its `RtOp` and its interpreter entry. `build.rs` generates the checker's, the interpreter's and KIR's tables from it.
- *Why:* each built-in is spelled out in 4–5 places today, and the lists drift (`04139d930`).
- *Cost:* 3–5 weeks.
- *Risk:* low.
- *Unblocks:* adding a standard library function by editing the catalog and the runtime only, in line with keeping modules separate from the core.

**R3. Should the interpreter run KIR?**
- *What:* option C, decided after R1 and after the differential test exists.
- *Why:* it removes about 15,500 lines of second implementation.
- *Cost:* 2–4 months.
- *Risk:* high (it loses the independent oracle; the features in section 2 must be rebuilt).
- *Unblocks:* one semantics for everything below KIR.

**R4. Split `kir`, and give linking its own crate.**
- *What:* `kir` (IR, `RtOp`, `Shape`, ABI, layout), `lower` and `passes` become separate crates. `runtime/native` and the backends depend only on `kir`. Linking (`link.rs`, `Exe`, `resources.rs`) moves out of `backend/cranelift`.
- *Why:* it removes the front end from the runtime archive and from runtime rebuilds, removes the LLVM → Cranelift edge, and puts lowering and passes behind real crate boundaries.
- *Cost:* 1–2 weeks.
- *Risk:* low (it is mechanical; `rt_data.rs` and `rt_op.rs` are shared files, so a maintainer coordinates).
- *Unblocks:* a smaller and faster link (to measure), and faster compiler development builds.

**R5. The driver owns the whole pipeline.**
- *What:* move literal checks, lowering, backend choice and the `kavach.toml` reads into one driver API that the CLI, the language server and the tests all call. Give desugaring a named step (with R1).
- *Why:* today the CLI, the conformance runner and the language server each put the steps together themselves.
- *Cost:* 1–2 weeks.
- *Risk:* low.
- *Unblocks:* pure lowering with caching keys; `KAVACH_PASSES` in one place.

**R6. Message codes carried by the message.**
- *What:* each error is created with its code (an enum) instead of `codes.rs` matching words.
- *Cost:* 2–3 weeks (at least 440 calls that make a message in the parser, resolver and checker).
- *Risk:* low.
- *Unblocks:* stable codes (spec 13.6) and tests per code.

**R7. Incremental compilation** (after R1).
- *What:* file-relative spans, the standard library's tree cached, modules checked against interface summaries, and per-function object caches.
- *Cost:* 2+ months.
- *Risk:* medium.
- *Unblocks:* refreshing the language server and `kavach context` (to-do) file by file.

## Cheap fixes (small and safe, can be done now)

1. **Raise the native cross-check floor** from 85 (`tools/cli/tests/native.rs:219`) to the number that runs today, or assert that every runnable file was checked.
2. **Check the `// error:` markers.** In `checker/tests/spec_sync.rs:71-74`, require exactly one error per marked line, and require that its text contains the marker's text. First run it as a report, to see how many tests differ.
3. **One "changes its receiver" function.** Make `checker::changes_in_place` public and use it from `interp/src/receivers.rs:20` and `kir/src/lower_lambda/scan.rs:11`. Add a test with `pop` and the text builder's methods.
4. **No silent `Unit`.** In `kir/src/lower_expr.rs:19`, a missing type should be a lowering error (`Unsupported`, with its line), or at least a `debug_assert`.
5. **Read `kavach.toml` once.** The driver puts the log, drop, agent and NaN-watch settings into `Compiled` or the options. KIR stops calling `Config::for_file` (5 places) and `watch_nan`.
6. **Add `KAVACH_PASSES`** in `passes.rs` as section 6 describes, with `KAVACH_NO_SROA` folded in, documented in `kir/README.md`.
7. **Unit tests for `moves.rs` and `exits.rs`:** operand order for calls with named arguments, record literals and method receivers (spec 3.10); and an early exit in the middle of an expression.
8. **Bring the docs up to date:**
   - `overview.md`: add the interpreter, WGSL and `runtime/native`; replace the runtime table, whose four folders (`memory`, `tasks`, `agents`, `durable`) hold only `.gitkeep`; say that the driver stops after checking.
   - `compiler/README.md`: it still says "89 of the conformance tests", "not yet errors, lambdas, maps" and "Phase 1 has started".
   - `interp/README.md`: it says "Not supported yet: agents, capabilities, `ask`", but `agents.rs`, `guard.rs` and `ai.rs` exist.
   - Replace the KIR README's prose on pass order with a pointer to `passes.rs`.
   - Fill in the missing README rows.
9. **A per-stage timer** (for example `KAVACH_TIMES=1`) covering parse, resolve, check, rewrites, lowering, passes, code generation and link, so section 4's open numbers can be measured.
10. **Decide GPU shaders from the checker.** Replace `c.src.contains("gpu.pipeline")` (`tools/cli/src/native.rs:191`) with what the checker knows, such as a `gpu_pipelines` table.

## What this review could not check

- Nothing was built, run or timed (the machine was busy). Stage times, memory use and the share of the runtime archive that the front end takes are open; cheap fix 9 and R4 measure them.
- Whether one mistake gives more than one message today: cheap fix 2 will tell.
- Whether `par::outline` can meet a `Move` inside a loop body. This was reasoned from liveness, not tested.
- Counts of drift commits come from file paths, not from reading every diff. Some "both sides" commits add a feature to both on purpose.
