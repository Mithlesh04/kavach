# tools/speedkit

The speed kit's runner, `compare`. The speed kit is a download per system with the benchmark programs already built in Kavach and in C++, their source, and this runner, so anyone can time Kavach against C++ on their own machine with nothing installed: no Rust, no C++ compiler, no Kavach ([to-do](../../docs/vision/todo.md), item 3 of the current order).

| File | What it holds |
|---|---|
| [src/main.rs](src/main.rs) | Reads `kit.txt`, checks that each pair prints the same result (leaving out lines that report times), runs the two in turns, prints the best time of each and how many times faster Kavach is, and saves the table in `results.txt` |
| [src/machine.rs](src/machine.rs) | The system, processor and thread count the table names |

`scripts/package-speed-kit.sh` (Linux, macOS) and `scripts/package-speed-kit.ps1` (Windows) build a kit into `.kavach/dist/`. The C++ is built as the system builds it: `g++ -O2 -static` on Linux, `clang++ -O2` on macOS, MSVC `cl /O2 /MT` on Windows (with Linux's 8 MB of stack, as the `stacks` benchmark recurses deeply). The repository's own benchmark results are measured against `g++` on Linux, so a Windows kit's numbers differ from them where MSVC's library is slower (its hash maps, for one).

Options: `compare --runs N` (best of N runs, default 5), `compare --only <name>`.
