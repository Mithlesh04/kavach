# benchmarks/ai-writes/languages/

One folder per language the benchmark asks models to write. The runner ([../run.py](../run.py)) is the same for all of them; each folder says what is particular to its language.

| Folder | Language |
|---|---|
| [kavach/](kavach/README.md) | Kavach |

A new language is a new folder here with:

| File or folder | What it holds |
|---|---|
| `language.py` | `NAME`, `FENCE` (its code blocks' tag), `EXTENSION`, `SCRATCH` (the file its tools work on), and `ask`, `code_of`, `program`, `run`, `failure`, `docs` and `mcp_command`: how to ask for a solution, put it together with the hidden tests, run it, and say why it failed |
| `tasks/<id>.json` | For each task of `../tasks/tasks.json` it takes part in: the code given, the signature to fill in, and the hidden tests (a call, and the line it prints) |
| `solutions/<id><extension>` | A reference solution, which `python run.py --check --language <name>` runs against the tests |
