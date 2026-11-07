# benchmarks/

Honest performance comparisons between Kavach and C#, Go, Rust, Java and Python.

## Rules

- Same algorithm in every language, written idiomatically by someone fluent in that language.
- Publish the code, machine, versions and raw numbers, not only charts.
- Measure startup time, peak memory and latency percentiles (p50/p99), not only throughput.
- Report where Kavach loses, too.

## Running them

`sh scripts/run-benchmarks.sh [kavach] [folder]` on Linux or WSL. To set up a Windows machine, see [windows-setup.md](../docs/architecture/windows-setup.md).

## Benchmarks

| Folder | What it compares |
|---|---|
| [loops/](loops/README.md) | Loops, calls and lists: Kavach against C++ |
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
| [net/](net/README.md) | How fast a task waiting for the network wakes: TCP round trips between two tasks, against C++ and Python |
| [mistakes/](mistakes/README.md) | Common mistakes from real code, and which step catches each before a run |

## Planned subfolders

Created when Phase 3 starts: one folder per benchmark (e.g. `http-server/`, `binary-trees/`), each with a subfolder per language.
