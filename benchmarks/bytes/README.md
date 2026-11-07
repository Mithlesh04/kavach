# Bytes benchmarks

Raw bytes work storage engines live on (to-do item 5), in Kavach and C++ (`g++ -O2`):

| Benchmark | What it measures |
|---|---|
| `crc` | 64 MB built byte by byte, then CRC32C over it 64 times: both sides use the processor's SSE 4.2 instruction |

Run them (Linux or WSL):

```
sh scripts/run-benchmarks.sh "" benchmarks/bytes
```

## Results

Measured 2026-11-02 (WSL Ubuntu, g++ 13, i7-13700HX), best of 9; ratio is Kavach ÷ C++.

| Benchmark | Kavach ms | C++ ms | Ratio |
|---|---|---|---|
| `crc` | 125 | 146 | **0.86** |

**What the number says:** the checksum itself is at hardware parity — timed apart, CRC32C runs 64 MB in about 10 ms on both sides (both use SSE 4.2). Three fixes took the whole run from 8.2× to 3.7×: byte lists now pack one byte per byte ([decision 0030](../../docs/decisions/0030-byte-packed-lists.md)), a counted loop's pushes are hoisted past the checks into reserved room, and `Bytes(list)` stopped copying the slot buffer. A fourth fix proved the `UInt8(i % 251)` conversion can never fail (the value is 0–250), so no check is emitted and LLVM vectorizes the build loop: 294 ms became 141. A fifth made `Bytes(list)` copy whole words instead of extracting byte by byte: 141 became 97. What's left is mostly the reserve's zero-fill and the checksum itself (~10 ms each side). Measured again 2026-11-06 on a busy machine, side by side: 0.86. The build loop's `i % 251` stays a remainder there, since a counter carried from round to round would stop LLVM from vectorizing the loop (`compiler/kir/src/counters.rs` leaves loops that run straight through alone).
