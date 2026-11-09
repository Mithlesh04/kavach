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

Linux (WSL Ubuntu 24.04), Intel i7-13700HX ([the machine](../README.md#the-machine)). Kavach: `kavach build` (LLVM backend). Python 3.12.3. Measured 2026-11-09 at commit 38fb662d (`kavach --version`), in three rounds one after another; in each round every step is the median of 5 runs in turns, and the whole program the best of 3. Each row is the best of the three rounds, in milliseconds.

| Step | Kavach ms | Python ms | Kavach is |
|---|---|---|---|
| stringify | 186 | 1232 | 6.6× faster |
| parse | 448 | 564 | 1.3× faster |
| read | 95 | 48 | 2.0× slower |
| decode | 310 | 2298 | 7.4× faster |
| whole program | 1138 | 5467 | 4.8× faster |

How many times faster Kavach was in each round, in order: stringify 6.3, 6.6, 6.7; parse 1.19, 1.29, 1.22; read 0.49, 0.51, 0.51 (2.0× slower); decode 6.1, 7.3, 7.4; whole program 4.4, 4.4, 4.8. On 2026-11-08 a busy machine took about 3× longer on both sides, and `read` was 1.5× slower than Python.

Peak memory: Kavach 0.70 GB, Python 1.06 GB.

A `Json` number keeps its exact digits ([decision 0071](../../docs/decisions/0071-json-numbers-keep-their-digits.md)), so `int()` and `float()` read the digits each time: that is most of what `read` still loses to Python, whose parser made the numbers once.

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

Still to do: the compact `Json` layout, for simdjson-class parsing and faster reads.
