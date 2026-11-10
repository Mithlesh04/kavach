# benchmarks/json

JSON on large data, in Kavach and Python, doing exactly the same work ([to-do item 1](../../docs/vision/todo.md)). The program makes 300,000 orders (records with a list of items), then times four steps:

| Step | What it does |
|---|---|
| stringify | Writes every order as JSON text: 52 MB |
| parse | Reads the text back into dynamic values (`Json` in Kavach, `dict`/`list` in Python) |
| read | Reads `qty` and `price` of each of the 1.5 million items from the parsed values |
| decode | Reads the text straight into records (`json.decode<[Order]>`; dataclasses in Python) |

| File | What it is |
|---|---|
| [kavach/json.kav](kavach/json.kav) | The Kavach program |
| [python/json_bench.py](python/json_bench.py) | The same in Python, with its `json` module (written in C) |

Python is the comparison because its `json` module is a fast C parser in daily use. C++ has no JSON in its standard library; a comparison with simdjson comes with the compact `Json` layout (to-do item 1).

## Results

Linux (WSL Ubuntu 24.04), Intel i7-13700HX ([the machine](../README.md#the-machine)). Kavach: `kavach build` (LLVM backend). Python 3.12.3. Measured 2026-11-10 at commit 567af3f with the key change below, in three rounds one after another; in each round every step is the median of 5 runs in turns, and the whole program the best of 3. Each row is the best of the three rounds, in milliseconds. The machine was busier than on 2026-11-09, when Python took 48 ms for `read`, so compare the two columns, not the numbers with older ones.

| Step | Kavach ms | Python ms | Kavach is |
|---|---|---|---|
| stringify | 209 | 1501 | 7.2× faster |
| parse | 503 | 690 | 1.4× faster |
| read | 94 | 58 | 1.6× slower |
| decode | 381 | 2534 | 6.7× faster |
| whole program | 1336 | 6519 | 4.9× faster |

How many times faster Kavach was in each round, in order: stringify 8.0, 6.2, 7.9; parse 1.65, 1.28, 1.37; read 0.54, 0.59, 0.62 (on 2026-11-09, before keys were found by their address: 0.49, 0.51, 0.51); decode 6.3, 6.7, 6.7; whole program 5.0, 4.4, 4.9.

Peak memory: Kavach 0.70 GB, Python 1.06 GB.

A `Json` number keeps its exact digits ([decision 0071](../../docs/decisions/0071-json-numbers-keep-their-digits.md)), so `int()` and `float()` read the digits each time, but that is only about 5% of `read` (profiled 2026-11-10). Since 2026-11-10 a key is found as Python finds it, by comparing the addresses of two keys the parser keeps one copy of. Most of what `read` still loses to Python is reaching each map: every item is a map of its own, and its table and its entries are allocations of their own.

## Progress

| Date | Change | read ms | decode ms |
|---|---|---|---|
| 2026-10-31 | First measurement | 240 | 1300 |
| 2026-10-31 | `a ?? b` moves into the branches of an inlined function, so `j.int() ?? 0` builds no optional | 207 | |
| 2026-10-31 | A constant argument is written into the inlined function; `map["key"]` with text keys reads the map in place, with no text object for a constant key | 145 | |
| 2026-10-31 | Union variants and fields are read inline, not through the runtime | 80 | |
| 2026-10-31 | `decode` reads straight from the text, asking for what the type expects next; only a failure goes through the tree, to say why | | 350 |
| 2026-10-31 | Parsing shares `true`, `false`, small integers and recently seen short text (parse 540 → 510 ms, 75 MB less memory) | 70 | |
| 2026-11-08 | Against the commit before, best of 7 in turns on a quiet machine: `stringify` writes text straight from the values (868 → 231 ms); small allocations come from blocks on huge pages (`runtime/native/src/small_blocks.rs`), numbers keep their digits with none worked out first, repeated keys and values are found faster (parse 734 → 528 ms); `Int(text)` and `Float(text)` read the text without copying it | 149 → 112 | 492 → 363 |
| 2026-11-10 | Each constant key gets a slot of its own in the key cache | 91 → 81 | |
| 2026-11-10 | A key found as the same object twice is held, then found by its address | 95 → 83 | |

Still to do: the compact `Json` layout, for simdjson-class parsing and faster reads.
