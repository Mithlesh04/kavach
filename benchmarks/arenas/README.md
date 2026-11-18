# Arena benchmarks

The proof [decision 0128](../../docs/decisions/0128-arenas-as-regions.md) asks for before `arena { }` becomes a real region: many small values made together and dropped together, in Kavach and in C++ (`g++ -O2`), once with C++'s normal allocator and once with `std::pmr::monotonic_buffer_resource`, the arena C++ offers.

| Benchmark | What it measures |
|---|---|
| `tokens` | 100,000 lines split into words (1.1 million of them), ten times over; each round's words are dropped together |
| `frames` | 100 frames, each collecting the hits of 100,000 particles into a temporary list of records, adding them up and dropping the list |

The Kavach programs ([kavach/](kavach)) use a plain block, `{ }`: an `arena` doesn't make values faster yet. [cpp/](cpp) uses C++'s normal allocator, [cpp-pmr/](cpp-pmr) gives each round's values one `std::pmr::monotonic_buffer_resource`. Every version prints the same total.

## Results

Measured 2026-11-18 at commit 5d7ec337 on [the machine](../README.md#the-machine) (WSL Ubuntu 24.04, g++ 13.3), best of 9 runs taken in turns, with the machine's load average about 2–3 (other sessions' builds had just ended). Milliseconds:

| Benchmark | Kavach `{ }` | C++ | C++ with `pmr` | Kavach against C++ `pmr` |
|---|---|---|---|---|
| `tokens` | 353 | 426 | 725 | 2.1× faster |
| `frames` | 44 | 163 | 151 | 3.4× faster |

Decision 0128's targets were an `arena` at least 2× faster than `{ }` and within 1.2× of C++ with `pmr`. Kavach's plain block already beats C++ with `pmr` on both, with no arena. An arena only saves the work of allocating and freeing: in `tokens`, the same lines without the split take 48 ms, so splitting costs about 330 ms for 11 million words, about 30 ns a word, and much of that is scanning and copying characters, which an arena can't save. A 2× gain over `{ }` is unlikely, so, as the decision says, a maintainer decides again before the region is built (to-do item 24).

Run them (Linux or WSL): `sh scripts/run-benchmarks.sh "" benchmarks/arenas` times Kavach against `cpp/`; the `pmr` versions are built and timed the same way by hand: `g++ -O2 -o pmr_tokens cpp-pmr/tokens.cpp`.
