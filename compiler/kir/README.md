# compiler/kir (`kavach-kir`)

KIR, the Kavach IR: a small, typed, structured form of a checked program, between the checker and the code generators. Lowering reads the checker's type for every expression (`Compiled::expr_types`).

KIR grows step by step. Anything outside the supported subset is reported as `Unsupported` with its line, never compiled wrongly; the interpreter still runs such programs.

**Supported now:** integers of every size with `wrapping_*` and `checked_*`, `Float`, `Decimal`, `Bool`, `Char`, and `parse`; text with interpolation and its methods; lists (literals, indexing, `items[i] = v`, `push`, `get`, `for`, `enumerate`); maps (literals, `m[k]`, `m[k] = v`, `get`, `contains_key`, `keys`, `values`, `for (k, v) in m`); `Shared<T>`; tuples and destructuring in `const` and `for`; records with defaults, `with`, field assignment and invariants; unions and their variants; optionals (`none`, `??`, `?.`, `if let`, `unwrap`); methods and `extend`; functions and recursion; generic functions and generic records and unions; `if`, `while`, `for`, `break`, `continue`, `return`; `match` on numbers, booleans, text, tuples, unions and optionals, with guards; `require`, `ensure`, `assert`; `panic`; number conversions; errors (`! E`, `fail`, `try`, `context`, `catch`, `match` with `ok`/`err`, a failing `main`); lambdas and dot lambdas, inlined in `filter`, `map`, `flat_map`, `any`, `all`, `sum`; function values everywhere else (stored, passed, returned, in records and lists) as closures; named functions as values; tasks (`spawn`, `.wait()`, `parallel_map`) and channels (spec 9).

A lambda written into a list method is inlined. Any other function value is a closure: its own function plus a heap object with the function's address and copies of the values it uses (spec 3.3).

Generic functions are compiled once per set of type arguments (monomorphization, `lower_generic.rs`), so each copy is as fast as hand-written code.

After lowering, `lower_tasks.rs` wraps each function that spawns tasks so that it waits for them when it returns (spec 9.2). Then `unrecurse.rs` turns `if c { base } else { e + self(step) }` self-recursion into an accumulator loop (what C++ compilers do), bounded and backed by the code as written as a fallback, so a recursion that would run out of stack still panics there; one that counts a parameter down by one to zero becomes a counted `for` loop (`unrecurse/counted.rs`). Then `inline.rs` turns early returns into `if`/`else` and inlines small functions into their callers (a small recursive function into itself, two levels deep; a recursion turned loop as its loop, with a call as its fallback), then copies a callee's quick exit (`if n < 2 { n }`) in front of each call left, so a recursion's leaves need no call. Then `stop_points.rs` adds the points where a task stopped by `cancel()` or its time limit stops even while it computes, and `cancel.rs` adds the checks a cancelled task stops at (spec 9.6); before it does, `whole_lines.rs` works out the values of each `print` that can wait before any of the line is written, so the line comes out whole (spec 9.8). Then `gpu_stay.rs` leaves a GPU `map`'s result in GPU memory when a local holds it, and fetches it before any other read of that local (spec 15.2, decision 0055), so a loop of `map`s copies back once. Then `devirt.rs` turns a call through a once-stored record field holding a known function — `h.on(x)` — into a direct call, which LLVM then inlines. Then `sroa.rs` splits each record local the function never shares into one local per field — and each fixed-size list stored from literals and read at constant indexes into one local per element — so building, copying and changing them never touch the heap. Then `coalesce.rs` moves `a ?? b` into the branches of an inlined `a`, so `j.int() ?? 0` builds no optional. Then `jam.rs` computes eight rounds of a loop at once when its body sums an inner loop and nothing can fail (unroll and jam); one check before the loop covers every index, so the inner loop's reads are `InRange` and check nothing. Then `fuse.rs` fuses list pipelines: when a list built by one loop (`map`, `filter`…) is used only by the loop of the next statement, the two become one loop and the list is never built. The fused loop is the fast half of a `Fallback`: if a check in it fails, the code as written runs instead and fails where the program says (only the LLVM backend runs the fast half). Then `version.rs` gives a loop that multiplies by a local it never changes a copy, run when a test before the loop shows that local is in `0..=2^31 - 1`, and keeps it only where `ranges.rs` then removes checks from it; the loop as written runs otherwise. Where it does more, the copy gets locals of its own and also needs the loop's accumulators not to be negative, so its `%` loses its sign too. Then `ranges.rs` finds the range of values each integer local can hold and removes checks that can never fail: arithmetic that cannot overflow, and `%` by a constant becomes a compare and a subtract (or nothing) when the value is known to be small. Then `moves.rs` turns the last read of each heap variable into a move: the value is handed over instead of shared, so the next owner can change it in place without copying.

**Next:** methods of generic types, sets.

| File | Job |
|---|---|
| `src/ir.rs` | The KIR types, including heap shapes (`Shape`) |
| `src/types.rs` | KIR types for checker and written types; the shape table |
| `src/lower.rs` | The file, functions, methods, contracts, optional wrapping |
| `src/lower_stmt.rs` | Statements: `const`, assignment to fields, loops |
| `src/lower_expr.rs` | Expressions, `if`, `if let`, `??` |
| `src/lower_calls.rs` | Calls to named functions, and their arguments |
| `src/lower_methods.rs` | Method calls: user methods, through interface values (2.15), and fields holding functions (3.4) |
| `src/lower_c.rs` | C functions (`extern c`, spec 10.7): a function with no body, called as the C symbol of its name |
| `src/lower_ptr.rs` | Raw memory (`Ptr<T>`, spec 6.4): `alloc`, `free`, the address of a `read` or `write`, records laid out as C structs, `p.text()`; text copied for a C call, and callbacks as bare addresses (10.7) |
| `src/lower_schema.rs` | `json.schema<T>()` (spec 2.12): the schema worked out at compile time, built as a constant `Json` |
| `src/lower_tls.rs` | `net.connect_tls` and `net.listen_tls` (spec 14.10): the arguments in place, in the runtime's three operands |
| `src/lower_data.rs` | Records, variants, fields, invariants |
| `src/lower_drops.rs` | Registers each `Drop` type's `drop` at the program's start (spec 6.1) |
| `src/lower_match.rs` | `match` and patterns, including `ok`/`err` |
| `src/lower_errors.rs` | Calls that can fail, `try`, `catch`, `fail`, `context` |
| `src/lower_lambda.rs` | Inlined lambdas, list methods that take them, `Shared.update` |
| `src/lower_lambda/scan.rs` | What a lambda's body reads and changes, so a lambda changes only copies |
| `src/lower_generic.rs` | Generic functions: type arguments and instances |
| `src/lower_std.rs` | The standard library: `std` modules, more methods, sets (spec 14) |
| `src/lower_base64.rs` | `std.base64` (spec 14.20): its two ops, flags in place |
| `src/lower_every.rs` | `for tick in time.every_ms(ms)`: a loop whose rounds each wait for their start (spec 14.4) |
| `src/lower_iface.rs` | Interface values (spec 2.15): a box of the value, its type and one closure per method; calls through the box |
| `src/lower_json.rs` | `Json` literals, `j["key"]`, and `std.json` calls (spec 2.12) |
| `src/lower_log.rs` | Logging (spec 14.12): each event's lowest level worked out when compiling, the `[log]` settings handed over at the start of `main`, OpenTelemetry export |
| `src/lower_cli.rs` | `cli.parse<T>()`: defaults and the record, around the runtime's parsing |
| `src/lower_agents.rs` | Agents (spec 8.3, 8.8): the record a started agent is, `start`, and each handler run with its agent on the runtime's stack |
| `src/lower_dispatch.rs` | Capability calls (spec 8.2): through the running agent, or `Denied` |
| `src/lower_guard.rs` | An agent's guarded call: its limits (`sum`, `count`), approvals and implementation (spec 8.3) |
| `src/lower_replay.rs` | Events run again after a restart: each a task that `start` waits for until it ends or waits for a person, and a send of the same event joining it (decision 0072) |
| `src/lower_ai.rs` | `ask<T>` (spec 8.4): through the running agent's guard or straight to the model; the request with its schema, written when compiling |
| `src/passes.rs` | The passes after lowering, in order: files and tasks, inlining, dropping what no root reaches, then each function's own |
| `src/library.rs` | What a library exports (`pub fn`s C can call) and its C header (spec 10.10) |
| `src/collect.rs` | Lowering every function; one native code can't lower yet stops the build only when the program reaches it |
| `build.rs` | Makes `RtOp` and `RtOp::ALL` from the op lists in `src/rt_op/ops/` |
| `src/rt_op.rs` | Built-in operations the runtime carries out (`RtOp`), as `build.rs` makes them |
| [`src/rt_op/ops/`](src/rt_op/ops/README.md) | The ops, one list per family: an op's code is its place in them |
| `src/rt_op/codes.rs` | The fingerprint of the op codes, and the code of each op |
| `src/rt_op/groups.rs` | What each op is like: whether it changes its receiver or can fail, and which runtime entry carries out its group |
| `src/lower_optional.rs` | `a ?? b`: one map lookup when it can be, and `x ?? break` |
| `src/lower_closure.rs` | Function values: closures, named functions as values, calls through them |
| `src/lower_process.rs` | `spawn process f(args)` (spec 9.8): a task whose thunk runs the call in a child, and in the child an entry per call, run instead of the program |
| `src/lower_tasks.rs` | `spawn` as a task body, a thunk and a closure; functions that spawn wait for their tasks (spec 9), and functions with `uses Fs("folder")` keep file access inside it (spec 8.1); `parallel_map` in `src/lower_tasks/parallel_map.rs`, `for v in receive` in `src/lower_tasks/channels.rs` |
| `src/inline.rs` | Early returns to `if`/`else`; inlining small functions (tests in `src/inline/tests.rs`) |
| `src/sroa.rs` | Scalar replacement: a record local the function never shares, or a fixed-size list read at constant indexes, becomes one local per field |
| `src/coalesce.rs` | `a ?? b` moved into the branches of `a`, so no optional is built |
| `src/counters.rs` | A remainder by a constant of a counted loop's counter (`i % 7`) becomes a counter that wraps, after the loops on all cores are split off |
| `src/stop_points.rs` | Where a task stopped by `cancel()` or its time limit stops while it only computes (spec 9.6): each function's start and each loop round's, in programs that use either |
| `src/whole_lines.rs` | A printed line comes out whole (spec 9.8): a `print` whose values call anything works them out before it writes |
| `src/borrows.rs` | A closure's function borrows its closure, and the captures it never changes: which functions and locals, for both backends |
| `src/cancel.rs` | Cancellation (spec 9.6): after each wait, and each call that can wait, a check that returns when the task is cancelled |
| `src/inline/guard.rs` | Copying a function's quick exit in front of its calls |
| `src/jam.rs` | Unroll and jam of loops that sum an inner loop (building in `src/jam/build.rs`, tests in `src/jam/tests.rs`) |
| `src/gpu_stay.rs` | GPU `map` results kept in a local, or read only by a GPU op, stay on the GPU; other reads of the local fetch them first |
| `src/gpu_later.rs` | A loop's `total += xs.map(f).sum(g)` doesn't wait for the sum: its statement runs later, in order (tests in `src/gpu_later/tests.rs`) |
| `src/fuse.rs` | Fusing list pipelines into one loop, with the code as written as a fallback (tests in `src/fuse/tests.rs`) |
| `src/fuse/hoist.rs` | A chain written as one expression pulled apart into one stage per `const`, the shape fusing matches |
| `src/ranges.rs` | Value ranges of integer locals, narrowed by the `if`s and `Check`s before a use; removes checks they prove unnecessary, and divides in 32 bits what fits (what `a op b` ranges to, and what it rewrites, in `src/ranges/arith.rs`, where `(acc + v) % m` with `acc` in `0..m` becomes `acc + v % m` less at most one `m`, so a loop's running total waits for no division; a branch's condition holds until the branch sets its local; a counted loop's accumulators bounded by its rounds in `src/ranges/accumulate.rs`, also when a step is a remainder or `version.rs` made two copies of the loop, tested in `src/ranges/accumulate/tests.rs`; other tests in `src/ranges/tests.rs`) |
| `src/moves.rs` | Moves at a variable's last use (a liveness pass) |
| `src/par.rs` | Loops on all cores (decision 0031): finds the eligible counted loops, also inside `if`s and inlined function bodies, and outlines each into a chunk function the runtime fans out (`kv_par_for`), the loop as written behind it |
| `src/par/analyze.rs` | Which loops are eligible: the body does arithmetic, reads lists and writes lists this function made and never shared; or it is a sum (`acc += v`, or `acc = (acc + v) % m` with `v` never negative) |
| `src/par/chunk.rs` | A plain loop's chunk function and call site |
| `src/par/reduce.rs` | A sum's chunk function and call site: partial sums per slot, with prefix bounds so an overflow still panics at its round |
| `src/prune.rs` | Only functions reachable from the entry are kept (decision 0024) |
| `src/unrecurse.rs` | Adding self-recursion as a bounded accumulator loop, the code as written as its fallback |
| `src/unrecurse/counted.rs` | A recursion counting a parameter down by one to zero, as a counted `for` loop |
| `src/devirt.rs` | Calls through once-stored function-valued fields become direct calls |
| `src/drop_scopes.rs` | Values holding a `Drop` value end at the end of their block, last made first, and waiting `drop`s run at each statement boundary |
| `src/version.rs` | Loop versioning: a loop that multiplies by a local it never changes gets a copy without checks, run when a test before it shows the local is small; the copy may have locals of its own (`src/version/own.rs`), so its ranges don't mix with the loop as written (tests in `src/version/tests.rs`) |
| `src/walk.rs` | Looking through KIR: whether any expression in a block matches a test |
| `src/encode.rs` | The shape table as text, carried inside executables |
| `src/lower_selected.rs` | `std.crypto` and a WebSocket's handshake: one op per family, the function chosen by its number |
