# 0031. Math loops on all cores, with the plain loop's exact results

- **Status:** Accepted
- **Date:** 2026-11-02

## Context

To-do item 1 sets a target of **20× C++** (one thread, `g++ -O2`) for math
over large data: sums, dot products, element-wise arithmetic, `sqrt` over
big lists. One core cannot get there — a single `a * b + c` compiles to
the same instructions for every compiler. The 20× must come from doing
many operations at once: SIMD (4–8 numbers per instruction) and all cores.
Kavach can do this **automatically** where C++ cannot, because its values
never alias: a loop over a list provably touches memory no other thread
sees. The hard part is the promise that makes it safe to turn on by
default: **the results, panics and output are exactly those of the plain
loop.**

## Decision

A KIR pass marks eligible `for` loops; the backends compile a marked loop
twice (the plain body, and a chunked body as its own function); the
runtime's task pool runs chunks on all cores and keeps the plain loop's
behaviour observable:

1. **Eligible loops** (first slice): `for i in a..b` whose body only reads
   locals and lists, does arithmetic, and writes `out[i]` — one store per
   round, to an index equal to the loop variable, into a list made or
   uniquely owned by the loop's function. No calls except pure built-ins
   (`math.sqrt`, …), no prints, no pushes, no reads of what it writes.
   Reductions (`sum`, `dot`) come second, with the rule below.
2. **Panics replay.** Chunks run speculatively. If any chunk panics, the
   result is thrown away and the loop **re-runs serially from the start**;
   the serial run panics at the plain loop's exact round, message and
   line. A panicking loop pays double; a loop that doesn't (all of them,
   in practice) pays nothing.
3. **Reductions keep their order.** Integer sums are associative when
   every partial sum is overflow-checked against the serial prefix — so
   integer reductions combine chunk totals, and any overflow triggers the
   serial replay, which panics at the right round. Floating-point sums are
   **not** reordered: an eligible float reduction splits into chunks that
   each compute their serial partial, and the partials combine **in chunk
   order** only when the program asked for `fast_sum` (spec change,
   later); until then float reductions stay serial.
4. **Small loops stay serial.** The pass guesses work per round from the
   body (ops × width); the runtime only fans out when
   `rounds × work ≥ ~1 ms` and more than one worker exists. One thread
   environments run the plain loop — same code path, no pool.
5. **The interpreter never parallelizes.** It is the semantics reference.

## Consequences

- The backends gain loop outlining: a marked body becomes a function
  `fn chunk(lo, hi, captures…)`, called by `kv_par_for(lo, hi, chunk, ctx)`
  from the runtime's existing task pool (`rt_tasks`). This is the main
  new machinery, shared later by `parallel_map` and the JSON work.
- `benchmarks/math` is the gate: `scale` and `roots` first (element-wise),
  then `sum` and `dot` (reductions). Memory-bound loops cap at the
  machine's bandwidth (~4–5× one core); compute-bound loops (`roots`)
  should approach cores × SIMD width.
- SIMD inside each chunk comes from LLVM's vectorizer once the per-round
  checks are proven away (the `Convert` proof landed 2026-11-02; bounds
  checks fall to `raw_sets`); this record adds no hand-written SIMD.
- Debug switch `KAVACH_SERIAL=1` forces every loop serial, for bisecting.

## Alternatives considered

- **An explicit `parallel for`:** pushes the burden onto the program and
  splits the language into fast and slow dialects; the automatic rule
  with exact-results semantics is the product promise (to-do item 1).
- **Reordered float sums by default:** faster, but silently changes
  results between runs and machines — against spec 2.1's determinism.
- **A work-stealing runtime of our own:** the task pool already balances;
  chunk counts (~4× workers) are enough for even math loops.
