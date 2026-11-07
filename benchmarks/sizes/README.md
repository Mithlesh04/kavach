# benchmarks/sizes

The same programs at four sizes, from data that fits in the fastest cache to data that only fits in main memory, in Kavach, C++ (`g++ -O2`) and Python. Speed measured at one size can mislead; this shows how each language holds up as data grows.

| File | What it measures |
|---|---|
| [kavach/index.kav](kavach/index.kav), [cpp/index.cpp](cpp/index.cpp), [python/index.py](python/index.py) | 50 million reads at indexes a Lehmer generator picks, from a list of `--items` integers. The work is the same at every size; only where the list lives changes |
| [kavach/sort.kav](kavach/sort.kav), [cpp/sort.cpp](cpp/sort.cpp), [python/sort.py](python/sort.py) | Sorting `--items` integers a Lehmer generator makes: more work as it grows |
| [sizes](sizes) | The sizes: one line each, a label and the arguments every program gets |

Each program reads its size from `--items N`, so the index reads take `% n` by a value known only at run time. The [lookups](../lookups/README.md) benchmark divides by a constant instead, which is why its 10-million row is faster than the one here.

Billions are not here: a billion integers is 8 GB for Kavach and for C++, and Python needs several times that, past this machine's 15 GB.

Run them (Linux or WSL):

```
sh scripts/run-benchmarks.sh "" benchmarks/sizes
```

## Results

Measured 2026-11-02 (WSL Ubuntu, g++ 13, Python 3.12, i7-13700HX), best of 9 (Python: best of 3); ratio is Kavach ÷ C++, so below 1 is faster than C++.

| Benchmark | Kavach ms | C++ ms | Ratio | Python ms |
|---|---|---|---|---|
| `index` · thousands | 255 | 259 | 0.98 | 8814 |
| `index` · millions | 702 | 607 | 1.16 | 29550 |
| `index` · tens-of-millions | 972 | 1119 | **0.87** | 46361 |
| `index` · hundreds-of-millions | 1887 | 2381 | **0.79** | 61076 |
| `sort` · thousands | 1 | 1 | 1.00 | 60 |
| `sort` · millions | 24 | 95 | **0.25** | 749 |
| `sort` · tens-of-millions | 147 | 1074 | **0.14** | 10373 |
| `sort` · hundreds-of-millions | 1593 | 10943 | **0.15** | 107379 |

**What the numbers say:** with a thousand items, both compiled languages read from the first-level cache and tie. At a million (8 MB, inside the second- and third-level caches) Kavach is 1.16× slower than C++: Kavach checks every index and C++'s `operator[]` doesn't, and at cache speed the extra work shows. Past the caches, every read waits for main memory, that wait dwarfs the check, and Kavach comes out ahead (these rows include building the list, 800 MB at the largest size). Sorting a thousand items takes under a millisecond in both, so that row is mostly starting the program; from a million up the radix sort, with its buckets on all cores, is 4–7× faster than `std::sort`. Python stays 30–70× slower throughout.
