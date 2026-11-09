# Lookup benchmarks

Single reads at a given index or key (asked for 2026-11-02: at least 20×
the big dynamic languages), in Kavach, C++ (`g++ -O2`), Python and Node
(when installed), timed whole:

| Benchmark | What it measures |
|---|---|
| `index` | 50 million reads at indexes a Lehmer generator picks, from a list of 10 million integers (80 MB, past the caches) |
| `dict` | 20 million reads at keys a Lehmer generator picks, from a map of 2 million entries |

Every language runs the same algorithm: the same generator
(`j = j * 48271 % 2147483647`), the same sizes, a running total printed
at the end so nothing is optimized away. The indexes jump around the
whole list, so the reads miss the caches; that memory latency binds every
language the same way, which makes this an honest floor for "how fast can
a lookup be", not a loop the compiler can vectorize.

Run them (Linux or WSL):

```
sh scripts/run-benchmarks.sh "" benchmarks/lookups
```

## Results

Measured 2026-11-09 at commit 38fb662d (`kavach --version`; WSL Ubuntu, g++ 13, Python 3.12, i7-13700HX, [the machine](../README.md#the-machine)), in three rounds of best of 9 (Python: best of 3) one after another; each row is the best of the three rounds. Ratio is Kavach ÷ C++, so below 1 is faster than C++.

| Benchmark | Kavach ms | C++ ms | Ratio | Python ms | × Python |
|---|---|---|---|---|---|
| `index` | 300 | 561 | **0.53** | 17450 | **58×** |
| `dict` | 272 | 613 | **0.44** | 9195 | **34×** |

The ratio in each round, in order: `index` 0.56, 0.54, 0.53; `dict` 0.42, 0.43, 0.44. On 2026-11-02 they were 0.53 and 0.40.

Both pass the bar.

**Prefetching** (`compiler/kir/src/prefetch.rs`) took `index` from 0.86
to 0.53 of C++'s time and `dict` from 0.69 to 0.40. Each read here waits
on main memory in every language; what can change is whether the waits
overlap. The next index comes from the recurrence `j = j * 48271 %
2147483647`, not from what was read, so the compiler runs a copy of the
recurrence 32 rounds ahead and asks the memory for the list item, or
the map bucket, the loop will need then. Measured in C++ first: 16
rounds ahead gave 1.5×, 32 gave 1.66×, 64 no more; 2 MB pages gave 8%.

**What the numbers said before prefetching:** a list read past the caches is one bounds
check and one load, and the check's branch predicts perfectly. A map
read was 1.16× C++ when this benchmark was first measured (85 ns: a
probe in the position index, then the entry's slots — two cache misses).
Two changes made it 1.55× *faster* than C++: integer-keyed maps now
probe sixteen-byte buckets that hold the key and the value together, so
a lookup costs one memory access; and the table keeps this run's hash
seed in itself instead of reading the process-wide `OnceLock` twice per
lookup. C++'s `std::unordered_map` still chases a node pointer per
lookup and cannot change its layout; that is the better-defaults gap
to-do item 1 predicts.
