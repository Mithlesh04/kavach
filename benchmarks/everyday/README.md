# Everyday benchmarks

What programs do all day (to-do item 1: the everyday operations), in Kavach and in C++ (`g++ -O2`), timed whole:

| Benchmark | What it measures |
|---|---|
| `callbacks` | A callback held in a record, called 100 million times |
| `nested` | Fifty handlers each wrapping the next (middleware), called two million times: 100 million hops, fifty frames deep |
| `branches` | An if/else chain decided 200 million times |
| `copies` | A record copied, changed and read 100 million times, each round depending on the last |
| `stacks` | Recursion 100 thousand frames deep, a thousand times: a frame pushed and the stack checked per call |
| `alloc` | A short-lived list made, read and freed 20 million times |

Pipelines (`map`, `filter`, `sum` chains) are the loops set's `closures` benchmark. Run these (Linux or WSL):

```
sh scripts/run-benchmarks.sh "" benchmarks/everyday
```

## Results

Measured 2026-11-02 (WSL Ubuntu, g++ 13, i7-13700HX), best of 9; ratio is Kavach ÷ C++, so below 1 is faster than C++.

| Benchmark | Kavach ms | C++ ms | Ratio | Why |
|---|---|---|---|---|
| `alloc` | 7 | 216 | **0.03** | was **5.9×**: a fixed-size list read at constant indexes never exists |
| `callbacks` | 230 | 438 | **0.53** | was 1.2×: a once-stored callback devirtualizes and inlines; C++'s `std::function` stays indirect. Since 2026-11-07 a call through a function value changes no reference count (0.82 → 0.53). Measured 2026-11-07 on a busy machine, side by side |
| `nested` | 595 | 737 | **0.81** | fifty nested handlers run *faster* than C++'s `std::function` chain: since 2026-11-07 a closure's function borrows its closure and the captures it reads, so a hop changes no reference count (`compiler/kir/src/borrows.rs`; 1.32 → 0.81, the same compiler side by side). Measured 2026-11-07 on a busy machine |
| `copies` | 65 | 64 | **1.02** | was **89×**: scalar replacement keeps the record in registers, as C++ does; since 2026-11-05 `total += q.x + q.z` carries no overflow check either (a sum of bounded parts, `compiler/kir/src/ranges/accumulate.rs`); measured 2026-11-05 on a busy machine |
| `branches` | 337 | 393 | **0.86** | faster than C++ since 2026-11-06: `i % 7` of the loop's counter is a counter that wraps at 7 (`compiler/kir/src/counters.rs`), an add and a compare where C++ multiplies and shifts each round; before that 1.21. Measured 2026-11-06 on a busy machine, side by side |
| `stacks` | 102 | 127 | **0.80** | was **5.9×**: the adding recursion becomes an accumulator loop, with no checks, kept scalar by LLVM (a loop that divides is slower vectorized on plain x86-64); since 2026-11-06 its `% 3` is a counter too (1.02 → 0.80). Measured 2026-11-06 on a busy machine, side by side |

**What the numbers say:** `copies` and `alloc` were the proof that small values lived on the heap — a record copied in a loop, or a three-item list made and read, cost an allocation per round. Scalar replacement (`compiler/kir/src/sroa.rs`) now splits every record local the function never shares into one local per field, and does the same for a fixed-size list stored from literals and read at constant indexes: `copies` went from 89× slower than C++ to even, `alloc` from 5.9× slower to 31× *faster* (the list never exists, so the loop folds to arithmetic). `nested`'s fifty-deep handler chain is rebuilt through a reassigned variable, so its calls stay indirect, as C++'s do. What made each hop slower than C++'s was reference counting: the closure was handed to its function, which let go of it, and each capture read from it took a reference. A closure's function now borrows its closure for the call, as C++ calls a `std::function`, and the captures it never changes borrow too: 810 → 510 ms when first tried by hand.

## Compile speed

Measured by hand with `time` (WSL, release compiler): `kavach build` of hello world is about 0.26 s, nearly all spent linking the runtime archive (the "builds under 0.2 s" note in the [to-do](../../docs/vision/todo.md#noted-for-later)).
