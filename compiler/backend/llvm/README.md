# compiler/backend/llvm (`kavach-backend-llvm`)

Compiles KIR to optimized machine code through LLVM ([decision 0018](../../../docs/decisions/0018-llvm-backend-through-llvm-tools.md)): it writes the program as LLVM IR text, runs `opt -O3` and `llc` from rustup's `llvm-tools` component, and links the object file with the run-time library like the [Cranelift backend](../cranelift/README.md) does.

`kavach build` uses it when LLVM's tools are installed (`rustup component add llvm-tools`) and the program uses only what this backend supports so far; otherwise it builds with Cranelift. `KAVACH_BACKEND=cranelift` forces Cranelift; `KAVACH_LLVM_BIN` points at another LLVM's `bin` folder.

**Supported now:** every type and every expression the Cranelift backend compiles: numbers, text, lists (flat lists of records and lists of text too), maps, sets, records, tuples, unions, optionals, decimals, `Shared`, regular expressions, closures, functions that can fail (`try`, `catch`, `fail`, `context`), top-level code, every built-in method through the runtime, and every run-time check of spec 5.6. Every conformance test and example builds with it. `KAVACH_LLVM_IR=1` prints the IR, or why a program isn't supported.

The generated code calls the same runtime functions and reads the same object layout as the Cranelift backend, so the two behave the same; the conformance cross-check builds every program with `kavach build` and compares it with the interpreter. Every local is a stack slot that LLVM turns into a register. Calls between Kavach functions are `notail`, so running out of stack stays a panic.

| File | Contents |
|---|---|
| `src/lib.rs` | `build`: finding the tools, `opt`, `llc`, linking |
| `src/check.rs` | What is supported so far |
| `src/module.rs` | Declarations, text constants, the C `main` |
| `src/func.rs` | One function: stack slots, blocks, runtime calls, the stack check, returns |
| `src/expr.rs` | Expressions |
| `src/json.rs` | Reading a `Json` with a constant key: `j["key"]` and `j["key"].int() ?? 0`, read in place (decision 0082) |
| `src/calls.rs` | Calls of program functions and through function values: a closure's function borrows its closure (`kavach_kir::borrows`) |
| `src/stmt.rs` | Statements and loops (with one bounds check per loop where it can) |
| `src/stmt/raw.rs` | Which loop bodies allow raw stores and hoisted pushes: the only-use analyses and the straight-line rule |
| `src/ops.rs` | Arithmetic and conversions with their run-time checks |
| `src/pointers.rs` | Reads and writes through a `Ptr` (spec 6.4): one load or store |
| `src/heap.rs` | Reference counting, places, packing values into slots, runtime calls |
| `src/lists.rs` | Lists: bounds checks, steady buffers in loops, items (one byte per byte for one-byte types), pushes, prefetching |
| `src/records.rs` | Records, tuples and flat lists of them |
| `src/data.rs` | Optionals, unions, errors as values (`try`, `fail`), decimals, closures |
| `src/text.rs` | Text, interpolation, operators on heap values, lists of text |
