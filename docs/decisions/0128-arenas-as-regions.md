# 0128. Arenas as regions: made fast, freed at once

- **Status:** Accepted; the region is not built (see Outcome)
- **Date:** 2026-11-18

## Context

`arena { … }` (spec 6.3) is named for a way of managing memory: many small values taken from one block and freed together. As built on 2026-11-17, it differed from a plain block only in two checks: a value made from the arena's own variables can't be kept outside it, and the arena waits for the tasks spawned in it. Every value was still allocated and freed one by one, by reference counting. A maintainer asked what an arena gives over `{ … }`, and confirmed this design on 2026-11-17.

## Decision

An arena is a region of memory:

```kavach
fn parse_all(lines: [String]) -> Summary {
    arena {
        const tokens = lines.flat_map(tokenize)   // thousands of small values, made fast
        summarize(tokens)                         // the block's value: copied out
    }                                             // everything else: freed in one step
}
```

1. **A region.** Everything made while the block runs lives until the arena ends, then all of it is freed at once. That includes values made in functions it calls, such as `tokenize`. The block's value is copied out to normal memory.
2. **Safe without the programmer thinking about it.** The compile-time check refuses code in the block that would keep an arena value outside. For what the compiler can't see, a called function storing an arena value into something made outside the arena (a `Shared`, a list passed in), the runtime copies the value out at that moment. Nothing outside ever points into freed memory, and no program can tell the difference except by its speed.
3. **Tasks** spawned inside may read the arena's values, and the arena waits for them. Their own new values use normal memory, as they may run on other cores.
4. **Nesting:** an inner arena has its own region, and its value is copied into the outer one.
5. **Native code:** making a value in an arena is a pointer bump in a 64 KB chunk, with no lock and no size classes. Values are never freed one by one, and the arena's end returns the chunks in one step. Chunks are kept for the next arena on the same thread, so a per-frame arena asks the system for no memory after the first frame. The interpreter, a development tool, gives the same results without the speed.
6. **Proof first.** Two benchmarks, parsing 1M tokens and per-frame temporary lists for 100k particles, each run as `{ }`, as `arena { }`, and as C++ with `std::pmr::monotonic_buffer_resource`. The target is `arena` at least 2× faster than `{ }` and within 1.2× of C++. If it gains less than 1.3× over the normal allocator, a maintainer decides again: keep `arena` as the escape check only, or remove it.

## Consequences

- `arena` gives the speed its name promises, where many small values are made and dropped together: parsers, a game's frames, a request's work.
- The runtime gains regions and a copy-out step on stores into outside objects; KIR marks the allocations made inside arenas.
- Spec 6.3 states the region and the copy-out rule.

## Alternatives considered

- **Keep `arena` as the escape check only:** honest, but the name promises speed it doesn't give.
- **Remove `arena` until the allocator exists:** nothing to migrate (no users yet), but the escape check, already built, would go with it.
- **Arena allocation only for values made in the block's own code:** simpler, but `tokenize` and the other functions an arena calls are where most values are made.

## Outcome

The proof came first (2026-11-18, [benchmarks/arenas](../../benchmarks/arenas/README.md)): Kavach's plain `{ }` already beat C++ with `std::pmr::monotonic_buffer_resource` (`tokens` 353 ms against 725, `frames` 44 against 151), and most of `tokens`' time was scanning and copying, not allocating, so a 2× gain over `{ }` was unlikely. As point 6 provides, a maintainer chose again on 2026-11-18: `arena` stays as it was, the escape check and the wait for its tasks, with no region. A program whose profile shows allocation as the bottleneck reopens it.
