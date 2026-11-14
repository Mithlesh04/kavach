# benchmarks/ai-writes/languages/kavach/

Kavach's side of the benchmark: how to ask a model for Kavach, run its answer, and read why it failed.

| File or folder | What it holds |
|---|---|
| `language.py` | Asks for a solution (a ```kavach block, with no `main`), puts it together with the given code and a `main` printing the hidden tests, runs it with `kavach run` (the `kavach` on `PATH`, or `KAVACH`), sorts a failure by its first error, gives `llms.txt` to the `docs`, `repair` and `tools` conditions, and starts `kavach mcp` for `tools` |
| `tasks/<id>.json` | Each task's given code, signature and hidden tests: a call and the line Kavach prints for it |
| `solutions/<id>.kav` | Each task's reference solution, with its given code |
