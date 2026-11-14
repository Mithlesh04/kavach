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

Measured at commit –. A cell not measured is "–".

| Model | zero | docs | repair | tools |
|---|---|---|---|---|
| Claude Haiku 5.5 | – | – | – | – |
| Claude Sonnet 5.5 | – | – | – | – |
| Claude Opus 5.5 | – | – | – | – |
| Claude Fable 5.2 | – | – | – | – |

## Repair rounds

The average number of graded tries a passing task took.

| Model | zero | docs | repair | tools |
|---|---|---|---|---|
| Claude Haiku 5.5 | – | – | – | – |
| Claude Sonnet 5.5 | – | – | – | – |
| Claude Opus 5.5 | – | – | – | – |
| Claude Fable 5.2 | – | – | – | – |

## Cost and time

Dollars and seconds for all 75 tasks.

| Model | zero | docs | repair | tools |
|---|---|---|---|---|
| Claude Haiku 5.5 | – | – | – | – |
| Claude Sonnet 5.5 | – | – | – | – |
| Claude Opus 5.5 | – | – | – | – |
| Claude Fable 5.2 | – | – | – | – |

## Failure kinds

What the last failed try of each failed task went wrong on: syntax, type (and names), capability, panic, timeout, or logic (it ran, but printed something else).

| Model | zero | docs | repair | tools |
|---|---|---|---|---|
| Claude Haiku 5.5 | – | – | – | – |
| Claude Sonnet 5.5 | – | – | – | – |
| Claude Opus 5.5 | – | – | – | – |
| Claude Fable 5.2 | – | – | – | – |

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
- `KAVACH`: the path of the `kavach` command, when it isn't on `PATH`.

Each run writes every answer, its code, tokens, cost, time and failures to `results/<date>-<language>.json`.

## The files

| File or folder | What it holds |
|---|---|
| [run.py](run.py) | The runner, the same for every language: it asks the model, grades the answer, keeps the results |
| [mcp_client.py](mcp_client.py) | A small client for a language's MCP server, for the `tools` condition |
| [tasks/](tasks/README.md) | The problems, once, apart from any language |
| [languages/](languages/README.md) | One folder per language: how to ask for it, run it and read its failures, and its signatures, tests and reference solutions |
| [prices.json](prices.json) | Each model's price per million tokens, and where and when it was read |
| [requirements.txt](requirements.txt) | The SDK version the runner was tested with |
| [method.md](method.md) | How the tasks, prompts, conditions and grading work |
