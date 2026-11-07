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

Linux (WSL Ubuntu 24.04), Intel i7-13700HX. Kavach: `kavach build` (LLVM backend). Python 3.12.3. Typical of 5 runs, in milliseconds; timings vary by about 10%.

| Step | Kavach ms | Python ms | Kavach is |
|---|---|---|---|
| stringify | 700 | 1400 | 2.0× faster |
| parse | 510 | 580 | as fast |
| read | 70 | 50 | 1.4× slower |
| decode | 350 | 2800 | 8× faster |
| whole program | 1830 | 6450 | 3.5× faster |

Peak memory: Kavach 0.67 GB, Python 1.06 GB.

Since [decision 0071](../../docs/decisions/0071-json-numbers-keep-their-digits.md) (2026-11-05), a `Json` number keeps its exact digits. Against the commit before it, on the same machine: `stringify` and `decode` unchanged, `parse` about 15% slower, `read` about 60% slower (`int()` and `float()` read the digits each time). The table above is from before; it is measured again with the next benchmark pass.

## Progress

| Date | Change | read ms | decode ms |
|---|---|---|---|
| 2026-10-31 | First measurement | 240 | 1300 |
| 2026-10-31 | `a ?? b` moves into the branches of an inlined function, so `j.int() ?? 0` builds no optional | 207 | |
| 2026-10-31 | A constant argument is written into the inlined function; `map["key"]` with text keys reads the map in place, with no text object for a constant key | 145 | |
| 2026-10-31 | Union variants and fields are read inline, not through the runtime | 80 | |
| 2026-10-31 | `decode` reads straight from the text, asking for what the type expects next; only a failure goes through the tree, to say why | | 350 |
| 2026-10-31 | Parsing shares `true`, `false`, small integers and recently seen short text (parse 540 → 510 ms, 75 MB less memory) | 70 | |

Still to do: the compact `Json` layout, for simdjson-class parsing and faster reads.
