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

Measured 2026-11-09 at commit 38fb662d (`kavach --version`; WSL Ubuntu, g++ 13, Python 3.12, i7-13700HX, [the machine](../README.md#the-machine)), in three rounds of best of 9 (Python: best of 3) one after another; each row is the best of the three rounds. Ratio is Kavach ÷ C++, so below 1 is faster than C++.

| Benchmark | Kavach ms | C++ ms | Ratio | Python ms |
|---|---|---|---|---|
| `index` · thousands | 170 | 171 | 0.99 | 5825 |
| `index` · millions | 157 | 153 | 1.03 | 11815 |
| `index` · tens-of-millions | 486 | 502 | **0.97** | 17957 |
| `index` · hundreds-of-millions | 655 | 896 | **0.73** | 29496 |
| `sort` · thousands | 1 | 1 | 1.00 | 31 |
| `sort` · millions | 14 | 62 | **0.23** | 426 |
| `sort` · tens-of-millions | 80 | 716 | **0.11** | 5420 |
| `sort` · hundreds-of-millions | 988 | 8497 | **0.12** | 76783 |

The ratio in each round, in order: `index` · thousands 1.01, 1.00, 0.99; · millions 1.11, 1.03, 1.02; · tens-of-millions 0.97, 0.94, 0.97; · hundreds-of-millions 0.71, 0.75, 0.73; `sort` · millions 0.22, 0.22, 0.23; · tens-of-millions 0.11, 0.11, 0.11; · hundreds-of-millions 0.12, 0.12, 0.12.

**What the numbers say:** with a thousand items, both compiled languages read from the first-level cache and tie. At a million (8 MB, inside the second- and third-level caches) Kavach is up to 1.1× slower than C++ (1.16× on 2026-11-02): Kavach checks every index and C++'s `operator[]` doesn't, and at cache speed the extra work shows. Past the caches, every read waits for main memory, that wait dwarfs the check, and Kavach comes out ahead (these rows include building the list, 800 MB at the largest size). Sorting a thousand items takes under a millisecond in both, so that row is mostly starting the program; from a million up the radix sort, with its buckets on all cores, is 4–9× faster than `std::sort`. Python stays 30–80× slower throughout.
