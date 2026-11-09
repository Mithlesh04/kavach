# Benchmark sources

Which README table row and column feed each chart on the site, one file per benchmark suite. There are no numbers here: `scripts/build-benchmarks.mjs` reads every file in this folder, finds each number in the suite README (where every measured run is recorded), and writes `data/benchmarks.json`. It fails when a row, a column or a number is missing, so a renamed benchmark can't leave a stale chart behind.

Each file has the suite's `title`, its `readme`, the `environment` it ran in, the `verify` command, its `series` (the ways compared) and its `benchmarks` (one chart each). A series or a benchmark names the README `row` and `column` to read.

| File | Suite | Read from |
|---|---|---|
| [bytes.json](bytes.json) | Raw bytes | [benchmarks/bytes](../../../benchmarks/bytes/README.md) |
| [chains.json](chains.json) | Method chains | [benchmarks/chains](../../../benchmarks/chains/README.md) |
| [channels.json](channels.json) | Channels | [benchmarks/channels](../../../benchmarks/channels/README.md) |
| [everyday.json](everyday.json) | Everyday operations | [benchmarks/everyday](../../../benchmarks/everyday/README.md) |
| [gpu.json](gpu.json) | On the GPU | [benchmarks/gpu](../../../benchmarks/gpu/README.md) |
| [json.json](json.json) | JSON on large data | [benchmarks/json](../../../benchmarks/json/README.md) |
| [lookups.json](lookups.json) | Single reads, large data | [benchmarks/lookups](../../../benchmarks/lookups/README.md) |
| [loops.json](loops.json) | Loops, calls and lists | [benchmarks/loops](../../../benchmarks/loops/README.md) |
| [math.json](math.json) | Math on large data | [benchmarks/math](../../../benchmarks/math/README.md) |
| [net-clients.json](net-clients.json) | Many network clients | [benchmarks/net](../../../benchmarks/net/README.md) |
| [net.json](net.json) | Network waits | [benchmarks/net](../../../benchmarks/net/README.md) |
| [sizes.json](sizes.json) | As data grows | [benchmarks/sizes](../../../benchmarks/sizes/README.md) |
| [tasks.json](tasks.json) | Tasks on all cores | [benchmarks/tasks](../../../benchmarks/tasks/README.md) |
