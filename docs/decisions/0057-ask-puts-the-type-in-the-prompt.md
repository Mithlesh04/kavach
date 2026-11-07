# 0057. `ask` puts the type in the prompt, and checks the answer

- **Status:** Accepted (2026-11-04, for review; the goal: "save tokens with no change to the output, at any cost")
- **Date:** 2026-11-04
- **Builds on:** [decision 0056](0056-the-model-is-chosen-in-code.md); changes how `ask<T>` asks (spec 8.4)

## Context

`ask<T>` sent each question in the provider's structured-output mode: the JSON Schema of `T` goes with the request, and the model's answer is held to it. The answer always fits, but the mode costs. Measured on `claude-haiku-5-5` (2026-11-04):

| One yes-or-no question | Input tokens | Output tokens | Time |
|---|---|---|---|
| Structured output (`ask` until now) | 227 | 10 | 1,350 ms |
| A plain prompt asking for JSON | 41 | 10 | 885 ms |
| A forced tool call, as agent frameworks ask | 526 | 34 | 977 ms |

Ten support tickets, each sorted into a kind, an urgency and whether a person must answer:

| | Input tokens | Output tokens | Time per call | Same fields as structured output |
|---|---|---|---|---|
| Structured output | 421 | 65 | 1,551–1,639 ms | 8 of 10 (one structured run against another) |
| The type in the prompt | 149 | 80–97 | 1,144–1,198 ms | 7 and 9 of 10, in two runs |

The type written into the prompt gives answers as often the same as structured output gives itself, with a third of the input tokens, in about 70% of the time.

## Decision

**The first try of an `ask` writes `T` into the prompt, in a short form, and checks the answer against `T`. If it doesn't fit, the next tries use structured output, as before.**

```
Triage this support ticket: The export button crashes the app.

Answer with only this JSON: {"kind": "Billing"|"Bug"|"Account"|"Feature"|"Other" (what the ticket is about), "urgency": int (1 can wait, 5 customers are losing money now), …}
```

- The short form is made from the same JSON Schema the answer is checked against: field names, types, choices, which fields may be left out, and the doc comments (`///`) as notes.
- Every answer is decoded as `T`. A program never gets a value of the wrong shape, as before.
- An answer that doesn't fit is asked again with structured output, which can't give a wrong shape. Up to two retries, then `InvalidOutput`, as before.

## Alternatives considered

- **Structured output always.** It never needs a second try, but every call pays about 200 tokens and 400 ms more.
- **A plain prompt and no check.** It is cheapest, but it is what the usual way does wrong: a missing field or a misspelled choice reaches the program.
- **Tool use.** It costs the most of the three: 526 input tokens for one question.

## Consequences

- Most calls cost about what a hand-written prompt costs, and the program still gets a checked value.
- A model that often answers in the wrong shape pays twice. The audit log's token counts (decision 0056) show it, and the benchmark (`benchmarks/agents/`) measures it.
- The prompt the model sees is longer than the one written in the code, by the short form of `T`. `KAVACH_AI_URL` and the audit log show what was sent.
