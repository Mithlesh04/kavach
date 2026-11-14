# How the AI-writability benchmark works

## The tasks

75 small tasks in the style of HumanEval and MBPP, written for this benchmark in November 2026, not copied from those sets, so no model has seen their answers. They come in four kinds:

| Kind | What they ask for | How many |
|---|---|---|
| basics | Numbers, text and lists: sums, words, palindromes, FizzBuzz, digits | 22 |
| data | Records, unions, maps, sorting, optionals, errors | 15 |
| algorithms | Primes, Roman numerals, brackets, edit distance, intervals, a stack | 20 |
| kavach features | Generics, interfaces, contracts, `try`, `?.`, tasks, JSON, `Decimal`, `with`, `inout`, closures | 18 |

Each task has one description, the same for every language ([tasks/tasks.json](tasks/tasks.json)). Each language adds its own signature to fill in, code given with the task (a type it uses, say), hidden tests, and a reference solution ([languages/kavach/](languages/kavach/README.md)).

A hidden test is a call and the line it must print. The runner puts the model's code, any given code, and a `main` printing each call together, runs it with `kavach run`, and compares the lines it printed with the ones expected. Every reference solution passes its tests (`python run.py --check`), so each task is known to be solvable in Kavach and its tests to be right.

## The prompt

Every condition sends the same request, built by the language's `ask` ([languages/kavach/language.py](languages/kavach/language.py)): "Write this in the Kavach programming language", the task's description, the code given, and "Write `<signature>`. Give only the code, in one ```kavach block, with no `main` function and no tests." The code of the answer's first fenced block is graded.

## The conditions

| Condition | What changes |
|---|---|
| `zero` | No system prompt. One try |
| `docs` | The system prompt is Kavach's guide for AI tools, [llms.txt](../../llms.txt) (about 1,500 tokens, cached). One try |
| `repair` | As `docs`. A failed try's errors (the compiler's or the panic's message, or the lines printed against those wanted) go back to the model, which answers again, up to 3 graded tries |
| `tools` | As `docs`, with tools: `kavach mcp`'s (`kavach_check`, `kavach_run`, `kavach_test`, `kavach_outline`, `kavach_query`, `kavach_context`), which work on one file, and the runner's own `write_code`, which saves the model's code there. The model may try its code as often as it likes (it can add a `main` to run it), up to 8 API calls for each graded try, and up to 3 graded tries. A tool call that runs longer than 60 seconds (a loop that never ends) is stopped, and the model is told so |

A task passes under a condition when one of its graded tries passes. Pass@1 is the share of tasks that pass; for `repair` and `tools` it counts the tries the condition allows, and the "Repair rounds" table says how many were used.

## Models and settings

| Name | Model id |
|---|---|
| Claude Haiku 5.5 | `claude-haiku-5-5` |
| Claude Sonnet 5.5 | `claude-sonnet-5-5` |
| Claude Opus 5.5 | `claude-opus-5-5` |
| Claude Fable 5.2 | `claude-fable-5-2` |

Each call uses the official Python SDK ([requirements.txt](requirements.txt)) and `max_tokens` 8,000. No other setting is sent, so each model thinks as it does by default (all four think adaptively) and samples at its own default; these models don't take a temperature of 0. A run's date is in its results file.

## Failure kinds

A failed try is sorted by what went wrong, in this order:

| Kind | When |
|---|---|
| timeout | The program ran for more than 60 seconds |
| syntax | The first error is one of the parser's: "expected …", "unexpected …", a `{` never closed, and the like |
| capability | The first error names a capability or a `uses` clause |
| type | Any other compile error: a type, a name that isn't defined, a method a type doesn't have |
| panic | It compiled, and panicked while running |
| logic | It ran to its end and printed something else |

`kavach check --format json` gives no category with each message yet (spec 13.6 promises a `code`), so the first error's wording is matched ([language.py](languages/kavach/language.py)).

## Cost and time

Tokens are the API's own counts for each response (input, output, and cache reads and writes). Dollars are those counts at the rates in [prices.json](prices.json), read from Anthropic's pricing page on the date written there. Time is the wall-clock time of a task's whole attempt: the calls, the tools and the grading.
