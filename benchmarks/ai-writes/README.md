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

Measured on 2026-11-14 at commit `22a20eb`, all 75 tasks, one run of each cell (1,200 answers, $24.66 in all). Every answer is in [results/2026-11-14-kavach.json](results/2026-11-14-kavach.json); [summarize.py](summarize.py) makes these tables from it.

| Model | zero | docs | repair | tools |
|---|---|---|---|---|
| Claude Haiku 5.5 | 9% (7/75) | 53% (40/75) | 97% (73/75) | 100% (75/75) |
| Claude Sonnet 5.5 | 4% (3/75) | 75% (56/75) | 99% (74/75) | 100% (75/75) |
| Claude Opus 5.5 | 7% (5/75) | 77% (58/75) | 100% (75/75) | 100% (75/75) |
| Claude Fable 5.2 | 7% (5/75) | 87% (65/75) | 100% (75/75) | 100% (75/75) |

## Repair rounds

The average number of graded tries a passing task took.

| Model | zero | docs | repair | tools |
|---|---|---|---|---|
| Claude Haiku 5.5 | 1.00 | 1.00 | 1.45 | 1.01 |
| Claude Sonnet 5.5 | 1.00 | 1.00 | 1.30 | 1.00 |
| Claude Opus 5.5 | 1.00 | 1.00 | 1.24 | 1.00 |
| Claude Fable 5.2 | 1.00 | 1.00 | 1.16 | 1.00 |

## Cost and time

Dollars and minutes for all 75 tasks.

| Model | zero | docs | repair | tools |
|---|---|---|---|---|
| Claude Haiku 5.5 | $0.02, 5 min | $0.03, 5 min | $0.05, 8 min | $0.09, 9 min |
| Claude Sonnet 5.5 | $0.43, 6 min | $0.45, 7 min | $0.59, 8 min | $1.32, 35 min |
| Claude Opus 5.5 | $0.82, 7 min | $0.73, 6 min | $0.89, 7 min | $2.25, 14 min |
| Claude Fable 5.2 | $3.02, 13 min | $4.57, 20 min | $4.80, 20 min | $4.60, 12 min |

## Failure kinds

What the last failed try of each failed task went wrong on: syntax, type (and names), capability, panic, timeout, or logic (it ran, but printed something else).

| Model | zero | docs | repair | tools |
|---|---|---|---|---|
| Claude Haiku 5.5 | syntax 43, type 25 | type 29, syntax 6 | type 1, syntax 1 | none |
| Claude Sonnet 5.5 | syntax 45, type 27 | type 12, syntax 7 | type 1 | none |
| Claude Opus 5.5 | syntax 40, type 30 | type 13, syntax 4 | none | none |
| Claude Fable 5.2 | syntax 42, type 28 | type 7, syntax 3 | none | none |

## What the run shows

- **Without help, models write Rust.** Zero-shot, 4–9% pass. Of the 280 failures, 168 stop at Rust's `let mut` (a Kavach `let` can already change), and 40 more at a semicolon. At `22a20eb` the message for `let mut` was a plain "expected `=` and a value"; it now says to write `let`. The models know the language's shape, not its words.
- **The guide does most of the work.** With [llms.txt](../../llms.txt) as the system prompt (about 1,500 tokens), 53–87% pass, and syntax errors nearly vanish. What remains is mostly type errors: writing a union's variant without its type (decision 0070), joining lists with `+`, leaving out an argument, a method that isn't there.
- **The guide itself was out of date.** At `22a20eb`, `llms.txt` still showed variants without their type (`status: Status = Pending`, which decision 0070 refuses) and `Int.parse(text)`, which is gone. Those two account for 23 of the 81 `docs` failures. Both are fixed, and its test now compiles every example and refuses such habits.
- **The compiler's messages finish the job.** Sent back the errors, every model reaches 97–100%, the strongest in 1.2 tries on average. With Kavach's own tools (`kavach mcp`), all four pass every task, almost always on the first graded try.
- **Tools cost more than they help here.** `repair` reaches nearly the same score as `tools` for less money and time: Sonnet $0.59 against $1.32, 8 minutes against 35.

The run was stopped once: a model's code looped forever inside `kavach mcp`, which has no time limit of its own. The runner then got its 60-second limit and `--resume`, and finished the rest.

## Found while writing the tasks

Writing the reference solutions, and a first small run, turned up idioms that Kavach refuses but that a writer used to Python or JavaScript reaches for. These are candidates for fixes, in the language or in its messages:

- `chars().reverse().join("")`: `[Char]` has no `join`.
- `"aeiou".contains(c)` with `c: Char`: `String.contains` takes only a `String`.
- `c.code()`: a `Char`'s code point is `Int(c)` (spec 2.1), with no method for it.
- `"([]{})"`: `{` in a string starts interpolation, so a literal brace is written `\{`.
- `0..xs.min() ?? 0`: `??` binds looser than `..`; the message doesn't suggest the parentheses.
- `stack.pop()`: lists have no `pop`.
- `len(xs)`, Python's way: the message suggests `Net`, not `xs.len()`.

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
