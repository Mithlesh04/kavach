# compiler/backend/cranelift (`kavach-backend-cranelift`)

Compiles KIR to machine code with [Cranelift](https://cranelift.dev) 0.125 (the newest version that supports Rust 1.88), two ways:

- `kavach run --native file.kav`: compiled in memory and run (JIT).
- `kavach build file.kav`: written to an object file and linked with the run-time library into an executable ([decision 0016](../../../docs/decisions/0016-native-runtime-and-executables.md)).

Both share one code generator. The run-time library that native code calls (heap values, printing, panics) is [runtime/native](../../../runtime/native/README.md).

Native code must behave exactly like the interpreter. [tools/cli/tests/native.rs](../../../tools/cli/tests/native.rs) runs every conformance test and example that KIR supports all three ways (interpreter, in memory, built executable) and compares output, exit code and panic message.

Heap values (text, decimals, lists, tuples, records, unions, optionals, errors) are reference-counted objects with copy-on-write, so values never alias (spec 6.1). Every expression yields an owned reference; reading a variable takes a new one, storing releases the old value, and a function releases its locals when it returns. With `KAVACH_CHECK_LEAKS=1`, a run reports any object left unfreed, and the cross-check requires zero for every program.

The run-time checks of spec 5.6 are compiled in: overflow-checked integer arithmetic for every integer size, division by zero, `MIN / -1`, and a stack check before each call, so a stack overflow panics at the line of the call. Panics print the same message and line as the interpreter and exit with 101.

A function that can fail (`! E`) returns its error after its value: a pointer that is 0 on success. `fail` releases the locals and returns that pointer. Errors are heap objects that know their own type, so `! Error` can hold any of them; `context` wraps one. When `main` returns an error, the run prints `error: …` and exits with 1 (spec 5.5).

Changing a list or record held in a variable (`items.push(x)`, `p.x = 1`) moves the variable's reference into the change, so an unshared value is changed in place, not copied.

A function value is a closure object whose first slot is the function's address; calling it is an indirect call that passes the closure as the first argument. A variable's last read (`Move` in KIR) hands its value over without touching its reference count.

Text constants are placed in the module's data, never as addresses from the compiler process, so the same code works in memory and in executables.

| File | Job |
|---|---|
| `src/lib.rs` | `run` (JIT), the target machine, compiling every function |
| `src/object.rs` | `build`: the object file and its C `main` |
| `src/unwind.rs` | The object file's unwind tables: `.eh_frame` on Linux (through `gimli`, [decision 0074](../../../docs/decisions/0074-gimli-for-unwind-tables.md)), `.pdata` and `.xdata` on Windows (decision 0065, step 1) |
| `src/link.rs` | Finding the run-time library and the system linker; linking an executable or a shared library |
| `src/library.rs` | `build_library` (`kavach build --lib`, decision 0026): an exported C function in front of each export (turning C text into Kavach text and back, errors into numbers), `<package>_free`, `<package>_last_error`, then a shared library |
| `src/abi.rs` | Machine types, signatures, the runtime functions native code imports |
| `src/codegen.rs` | Functions, statements, control flow, `print`, contracts, text constants |
| `src/errors.rs` | Calls that can fail, `fail`, and in-place changes to variables |
| `src/loops.rs` | `for` over ranges and lists |
| `src/calls.rs` | Calls of program functions and through function values: a closure's function borrows its closure (`kavach_kir::borrows`) |
| `src/data.rs` | Heap values: text, lists, records, optionals; values as 64-bit slots |
| `src/ops.rs` | Operators and conversions, with overflow and division checks |
| `src/pointers.rs` | Reads and writes through a `Ptr` (spec 6.4): one load or store |
| `src/inline.rs` | Heap access in generated code: reference counts, items, fields, `push`, in-place changes |
| `src/jit_unwind.rs` | Unwind tables for code the JIT makes (decision 0065, step 3), registered by the runtime once the code is placed |
| `src/places.rs` | Places: values read where they are, without a reference, including a field of the closure a closure's function borrows |
| `src/alloc.rs` | Small records, tuples and union values taken from the runtime's object pool, without a call |
| `src/text_lists.rs` | Lists of text: items through the runtime, the length of short ASCII text read from the item's tag |
| `src/flat.rs` | Flat lists of plain records: fields read and written in the list's buffer ([design](../../../docs/architecture/flat-lists.md)) |

Linking uses MSVC `link.exe` on Windows and `cc` on Linux and macOS. Windows and Linux (x86-64) are tested; macOS is not yet.

Next: a bundled linker, and fewer reference-count calls for values that never leave a function.
