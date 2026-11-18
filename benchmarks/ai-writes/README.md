# benchmarks/ai-writes/

Can AI models write Kavach? A new language is only as usable as the code AI tools write in it. A June 2026 study found models solve 0–1% of hard tasks zero-shot in young languages (Gleam, MoonBit) against 59–89% in popular ones, mostly failing on syntax ([Giagnorio et al.](https://huggingface.co/papers/2606.16827)). This benchmark measures where Kavach stands, and which help closes the gap.

75 small programming tasks, in the style of HumanEval and MBPP, are each given to a Claude model under four conditions. The model's answer passes when its hidden tests print exactly what they should. How everything is done is in [method.md](method.md).

| Condition | What the model gets |
|---|---|
| `zero` | The task only |
| `docs` | The task, and Kavach's guide for AI tools ([llms.txt](../../llms.txt)) |
| `repair` | As `docs`, and each failed try's errors sent back, up to 3 tries |
| `tools` | As `docs`, and Kavach's own tools (`kavach mcp`: check, run, test, outline, query) to try its code, up to 3 graded tries |

## Pass@1

Measured on 2026-11-15 at commit `946b610`, all 75 tasks, one run of each cell (1,200 answers, $19.77 in all), the second run (the first, at `22a20eb`, is compared under "Between the two runs"). Every answer is in [results/2026-11-15-kavach.json](results/2026-11-15-kavach.json); [summarize.py](summarize.py) makes these tables from it.

| Model | zero | docs | repair | tools |
|---|---|---|---|---|
| Claude Haiku 5.5 | 7% (5/75) | 88% (66/75) | 99% (74/75) | 100% (75/75) |
| Claude Sonnet 5.5 | 4% (3/75) | 88% (66/75) | 99% (74/75) | 100% (75/75) |
| Claude Opus 5.5 | 5% (4/75) | 93% (70/75) | 100% (75/75) | 100% (75/75) |
| Claude Fable 5.2 | 8% (6/75) | 97% (73/75) | 100% (75/75) | 100% (75/75) |

## Repair rounds

The average number of graded tries a passing task took.

| Model | zero | docs | repair | tools |
|---|---|---|---|---|
| Claude Haiku 5.5 | 1.00 | 1.00 | 1.11 | 1.01 |
| Claude Sonnet 5.5 | 1.00 | 1.00 | 1.11 | 1.00 |
| Claude Opus 5.5 | 1.00 | 1.00 | 1.05 | 1.00 |
| Claude Fable 5.2 | 1.00 | 1.00 | 1.01 | 1.00 |

## Cost and time

Dollars and minutes for all 75 tasks.

| Model | zero | docs | repair | tools |
|---|---|---|---|---|
| Claude Haiku 5.5 | $0.03, 5 min | $0.03, 5 min | $0.03, 6 min | $0.07, 8 min |
| Claude Sonnet 5.5 | $0.43, 6 min | $0.42, 6 min | $0.50, 7 min | $1.00, 10 min |
| Claude Opus 5.5 | $0.82, 7 min | $0.69, 6 min | $0.74, 6 min | $1.86, 13 min |
| Claude Fable 5.2 | $2.97, 13 min | $3.26, 14 min | $3.15, 13 min | $3.77, 11 min |

## Failure kinds

What the last failed try of each failed task went wrong on: syntax, type (and names), capability, panic, timeout, or logic (it ran, but printed something else).

| Model | zero | docs | repair | tools |
|---|---|---|---|---|
| Claude Haiku 5.5 | type 59, syntax 11 | syntax 5, type 2, panic 1, logic 1 | type 1 | none |
| Claude Sonnet 5.5 | type 67, syntax 5 | syntax 5, type 4 | type 1 | none |
| Claude Opus 5.5 | type 62, syntax 9 | type 4, syntax 1 | none | none |
| Claude Fable 5.2 | type 59, syntax 10 | syntax 1, type 1 | none | none |

## What the run shows

- **Without help, models write Rust.** Zero-shot, 4–8% pass. Of the 282 failures, 157 stop at Rust's `let mut` (a Kavach `let` can already change: the message now says so) and 55 at a semicolon. The models know the language's shape, not its words.
- **The guide does most of the work.** With [llms.txt](../../llms.txt) as the system prompt (about 1,500 tokens), 88–97% pass on the first try. What remains is a handful of type errors and a brace left open in text.
- **The compiler's messages finish the job.** Sent back the errors, every model reaches 99–100%, in 1.01 to 1.11 tries on average. With Kavach's own tools (`kavach mcp`), all four pass every task.
- **Tools cost more than they help here.** `repair` reaches the same score as `tools` for less: Sonnet $0.50 against $1.00, Opus $0.74 against $1.86.

Fable's times in this run were taken while the compiler's own tests ran on the same machine, so its minutes are longer than they would be alone; its answers and costs are not affected.

## Between the two runs

The first run (2026-11-14, commit `22a20eb`, 1,200 answers, $24.66, [results/2026-11-14-kavach.json](results/2026-11-14-kavach.json)) found what models trip on, and the fixes went in before the second:

- `llms.txt` taught code the compiler refused (union variants without their type, decision 0070; `Int.parse`): 23 of that run's 81 `docs` failures. Fixed, and its test now compiles every example.
- Clearer errors for other languages' habits (`let mut`, `len(xs)`, `xs.count()`, `.length`, `append`), `xs + ys` for lists, `xs.pop()`, `[Char].join` and `contains` with a `Char`.

| Model | docs, first | docs, second | repair, first | repair, second | repair rounds, first | repair rounds, second |
|---|---|---|---|---|---|---|
| Claude Haiku 5.5 | 53% | 88% | 97% | 99% | 1.45 | 1.11 |
| Claude Sonnet 5.5 | 75% | 88% | 99% | 99% | 1.30 | 1.11 |
| Claude Opus 5.5 | 77% | 93% | 100% | 100% | 1.24 | 1.05 |
| Claude Fable 5.2 | 87% | 97% | 100% | 100% | 1.16 | 1.01 |

The two runs differ in Kavach's guide and messages, not in their settings, and each cell is one run: a model's answers vary from run to run, so a few points either way are within that spread.

## Against Python

The same 75 tasks in Python 3, as a reference ([languages/python/](languages/python/README.md)), run on 2026-11-15 with the same four models, zero-shot and with repair: 600 answers, $2.73, every one in [results/2026-11-15-python.json](results/2026-11-15-python.json). Python has no `llms.txt` or tools of its own here, so it takes part in `zero` and `repair` only.

| Model | Python, zero | Kavach, zero | Kavach, docs | Kavach, repair | Kavach, tools |
|---|---|---|---|---|---|
| Claude Haiku 5.5 | 100% (75/75) | 7% (5/75) | 88% (66/75) | 99% (74/75) | 100% (75/75) |
| Claude Sonnet 5.5 | 100% (75/75) | 4% (3/75) | 88% (66/75) | 99% (74/75) | 100% (75/75) |
| Claude Opus 5.5 | 100% (75/75) | 5% (4/75) | 93% (70/75) | 100% (75/75) | 100% (75/75) |
| Claude Fable 5.2 | 100% (75/75) | 8% (6/75) | 97% (73/75) | 100% (75/75) | 100% (75/75) |

Kavach's columns are the second run's ([results/2026-11-15-kavach.json](results/2026-11-15-kavach.json)). With `repair`, Python needed 1.00 to 1.01 tries; Kavach 1.01 to 1.11.

| Model | Python, zero | Kavach, zero | Kavach, docs |
|---|---|---|---|
| Claude Haiku 5.5 | $0.01, 2 min | $0.03, 5 min | $0.03, 5 min |
| Claude Sonnet 5.5 | $0.10, 3 min | $0.43, 6 min | $0.42, 6 min |
| Claude Opus 5.5 | $0.20, 3 min | $0.82, 7 min | $0.69, 6 min |
| Claude Fable 5.2 | $1.06, 5 min | $2.97, 13 min | $3.26, 14 min |

What it shows:

- Every model solves every task in Python on its first try. So these tasks measure the language, not the models: what stands between a model and working Kavach is knowing Kavach.
- With its guide, Kavach comes within 3 to 12 points of Python; with its errors fed back, within 1; with its tools, level.
- The tasks are too easy to tell the models apart in Python. Harder tasks would show whether the gap grows with difficulty.
- About 20 Python prompts carry a note that puts a Kavach word in Python's terms (`require` as `assert`, `with` as `dataclasses.replace`); Kavach's prompts don't. A Python name or type error shows only when its line runs, so it counts as a type failure only if a test reaches it.

## Found while writing the tasks

Writing the reference solutions, and a first small run, turned up idioms that Kavach refused but that a writer used to Python or JavaScript reaches for. Most are fixed:

- `chars().reverse().join("")`: `[Char]` now joins (spec 14.2).
- `"aeiou".contains(c)` with `c: Char`: `contains` now takes a `Char` too.
- `0..xs.min() ?? 0`: the message now says to put the `??` in parentheses.
- `stack.pop()`: lists now have `pop` (spec 14.2).
- `len(xs)`, Python's way: the message now says to write `xs.len()`.
- Still as they were: `c.code()` (a `Char`'s code point is `Int(c)`, spec 2.1), and `"([]{})"`, where `{` started interpolation then, so a literal brace was written `\{` (since [decision 0131](../../docs/decisions/0131-plain-strings-and-backtick-templates.md) a string in quotes is plain).

It also found a bug, fixed since: the interpreter gave `0`, not `0.0`, for the `sum` of an empty list with a `Float` lambda.

## Run it yourself

Anyone can run this benchmark, with their own API key. It needs Python 3.10 or later and the `kavach` command.

```
cd benchmarks/ai-writes
python -m venv .venv
.venv/Scripts/activate            # Windows; on Linux and macOS: . .venv/bin/activate
pip install -r requirements.txt   # the official anthropic SDK

python run.py --check             # no key, no cost: every task's own solution passes its tests
python run.py --estimate --models haiku,sonnet,opus,fable --conditions zero,docs,repair,tools
```

`--estimate` prints the most a run could cost, at the rates in [prices.json](prices.json) (read from Anthropic's pricing page; update them if they changed). The whole table above costs at most about $90.

Then, with your key in `ANTHROPIC_API_KEY`:

```
python run.py --models haiku --conditions zero,docs --budget 2
python run.py --models sonnet --conditions repair --tasks fizzbuzz,two-sum
```

- `--language`: `kavach` (the default) or `python`, the reference ([languages/python/](languages/python/README.md)), which takes part in `zero` and `repair` only.
- `--models`: any of `haiku`, `sonnet`, `opus`, `fable`.
- `--conditions`: any of `zero`, `docs`, `repair`, `tools`.
- `--tasks`: task ids from [tasks/tasks.json](tasks/tasks.json); all of them when left out.
- `--budget`: the run stops before spending more than this many dollars (default 5).
- `--resume`: keeps the answers already in the results file (`--out`) and runs only the others, after a run was stopped.
- `KAVACH`: the path of the `kavach` command, when it isn't on `PATH`.

Each run writes every answer, its code, tokens, cost, time and failures to `results/<date>-<language>.json`.

## The files

| File or folder | What it holds |
|---|---|
| [run.py](run.py) | The runner, the same for every language: it asks the model, grades the answer, keeps the results |
| [mcp_client.py](mcp_client.py) | A small client for a language's MCP server, for the `tools` condition |
| [summarize.py](summarize.py) | Turns a results file into the tables above |
| [results/](results/) | The results files of the runs in this README |
| [tasks/](tasks/README.md) | The problems, once, apart from any language |
| [languages/](languages/README.md) | One folder per language: how to ask for it, run it and read its failures, and its signatures, tests and reference solutions |
| [prices.json](prices.json) | Each model's price per million tokens, and where and when it was read |
| [requirements.txt](requirements.txt) | The SDK version the runner was tested with |
| [method.md](method.md) | How the tasks, prompts, conditions and grading work |
