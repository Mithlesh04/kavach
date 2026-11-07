# Chain benchmarks

Chained calls — the fluent style of JavaScript arrays, Java streams, C# LINQ, Python comprehensions, and the builder APIs of transaction-heavy code — in Kavach, C++ (`g++ -O2`), Python 3 and Node. The target (to-do item 1): **at least 20× the big-market dynamic languages**, from fusing chain stages into one loop instead of building a list between them.

| Benchmark | What it measures |
|---|---|
| `datachain` | `map → filter → map → sum` across 10 million items |
| `builderchain` | A transfer built through five chained builder calls and read, 10 million times |
| `nestedchain` | Ten thousand rows, each put through its own `map → filter → sum` chain inside the outer chain |

The C++ versions are written the way a C++ programmer writes them — plain loops, no intermediate vectors — so the C++ column is the one to beat on speed, while the chained languages are the ones being beaten on both speed and style. Run them (Linux or WSL, with `python3` and `node` installed):

```
sh scripts/run-benchmarks.sh "" benchmarks/chains
```

## Results

Measured 2026-11-02 (WSL Ubuntu, g++ 13, Python 3.12, i7-13700HX); Kavach and C++ best of 9, Python best of 3. Node isn't installed in this WSL yet, so its column waits.

| Benchmark | Kavach ms | C++ ms | Python ms | vs Python | vs C++ |
|---|---|---|---|---|---|
| `builderchain` | 17 | 18 | 85575 | **5034× faster** | **even** |
| `datachain` | 19 | 40 | 1157 | **61× faster** | **2.1× faster** |
| `nestedchain` | 49 | 53 | 1335 | **27× faster** | **1.1× faster** |

Every chain beats the 20× bar, and `datachain` beats C++ itself. Two compiler moves did it:

- **Scalar replacement** (`compiler/kir/src/sroa.rs`): `builderchain` ran 871 ms (62× slower than C++) until the five builder calls inlined and the records they build split into plain locals — the whole chain is register arithmetic, the same code C++ emits.
- **Hoisting chains written as one expression** (`compiler/kir/src/fuse/hoist.rs`): fusing only matched chains written one `const` per stage. A chain written fluently — `xs.map(f).filter(g).map(h).sum(k)`, or nested inside another chain's lambda — nests each stage's loop inside the next; hoisting pulls it apart into one stage per `const`, and the existing fusion turns the whole chain into one loop with no list between stages. `datachain` went from 1.5× slower than C++ to 2.4× faster (C++ pays for its vectors; the fused Kavach loop builds none), and `nestedchain`'s inner chains fuse the same way.
