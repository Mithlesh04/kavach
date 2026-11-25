# benchmarks/

Honest performance comparisons between Kavach and C#, Go, Rust, Java and Python.

## Rules

- Same algorithm in every language, written idiomatically by someone fluent in that language.
- Same number of cores: one core against one core, all cores against all cores. A Kavach program that uses all cores (a loop written `on cores`, a big `sort`, a tensor product) gets a row on one core (`KAVACH_CORES=1`) and a row on all cores, against programs in the other languages written for all cores (`<name>_cores.cpp` and the like). "N× faster" is said only of such pairs.
- Publish the code, machine, versions and raw numbers, not only charts.
- Measure startup time, peak memory and latency percentiles (p50/p99), not only throughput.
- Report where Kavach loses, too.

## The machine

Every suite here ran on one laptop, unless its README says otherwise:

| Part | What it is |
|---|---|
| CPU | Intel Core i7-13700HX: 16 cores (8 fast, 8 efficient), 24 threads |
| Memory | 32 GB DDR5-4800 (two 16 GB modules); WSL gets 15.6 GB of it |
| GPU | NVIDIA GeForce RTX 4050 Laptop GPU, 6 GB (and the CPU's Intel UHD Graphics) |
| Windows | Windows 11 Home 10.0.26300, power plan Balanced, on mains power |
| Linux | WSL 2 on the same machine: Ubuntu 24.04, kernel 5.15.167.4-microsoft-standard-WSL2 |

Each suite's README gives the compilers and their flags, how many runs there were and how its figure is chosen. Results measured before 2026-11-07 came from commits that are no longer in the history, which was started again that day. From now on, each new result names the Kavach commit it was measured at, which `kavach --version` prints (spec 10.5).

From 2026-11-02 to 2026-11-17, Kavach spread some loops over all cores by itself ([decision 0031](../docs/decisions/0031-loops-on-all-cores.md)); since then a loop uses all cores only when written `on cores` ([decision 0126](../docs/decisions/0126-loops-on-one-core-unless-on-cores.md)). Rows known to have compared Kavach on all cores with another language on one are split into a one-core and an all-core row in their README, with *to measure* where a new run is needed. A row measured in those days whose Kavach program sums whole numbers in a long loop should be measured again before its ratio is quoted.

## Running them

`sh scripts/run-benchmarks.sh [kavach] [folder]` on Linux or WSL. To set up a Windows machine, see [windows-setup.md](../docs/architecture/windows-setup.md).

## Benchmarks

| Folder | What it compares |
|---|---|
| [loops/](loops/README.md) | Loops, calls and lists: Kavach against C++, Go and C# |
| [math/](math/README.md) | Arithmetic over large data: sums, dot products, element-wise operations, square roots |
| [everyday/](everyday/README.md) | What programs do all day: callbacks (nested too), branches, copies, deep call stacks, short-lived values |
| [bytes/](bytes/README.md) | Raw bytes work storage engines live on: checksums (CRC32C) |
| [lookups/](lookups/README.md) | Single reads at a given index or key, from a large list and a large map, against C++, Python and Node |
| [chains/](chains/README.md) | Chained calls, the fluent style: data chains, builder chains and chains inside chains, against C++, Python and Node |
| [tasks/](tasks/README.md) | Tasks that only compute: `map` against `parallel_map` on all cores |
| [channels/](channels/README.md) | Passing values between tasks through a channel, against C++ and Python |
| [sizes/](sizes/README.md) | The same reads and sorts at four sizes, from thousands to hundreds of millions of items, against C++ and Python |
| [json/](json/README.md) | Writing, parsing, reading and decoding 52 MB of JSON: Kavach against Python |
| [gpu/](gpu/README.md) | `map` with a `gpu fn` on the GPU, against the CPU and C++ on one core and on all cores |
| [agents/](agents/README.md) | AI agents: tokens and time for the same tasks and answers, against Python's plain prompts and agentic tool loops |
| [ai-writes/](ai-writes/README.md) | Can AI models write Kavach? 75 tasks graded by hidden tests, four Claude models, with no help, Kavach's guide, error feedback and Kavach's tools; anyone can run it with their own key |
| [net/](net/README.md) | How fast a task waiting for the network wakes: TCP round trips between two tasks, against C++ and Python |
| [mistakes/](mistakes/README.md) | Common mistakes from real code, and which step catches each before a run |
| [arenas/](arenas/README.md) | Many small values made and dropped together, as `arena { }` would hold them: Kavach's plain block against C++ and C++ with `std::pmr` |

## Planned subfolders

Created when Phase 3 starts: one folder per benchmark (e.g. `http-server/`, `binary-trees/`), each with a subfolder per language.
