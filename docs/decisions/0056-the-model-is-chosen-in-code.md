# 0056. The AI model is chosen in the code

- **Status:** Accepted (Mithlesh, 2026-11-04)
- **Date:** 2026-11-04
- **Replaces:** the `[ai]` `model` and `max_tokens` settings of `kavach.toml` (spec 8.4, 10.4)

## Context

`ask<T>` took its model from `kavach.toml`: one model, and one token limit, for the whole program. Real agents need more. A reviewer of legal clauses wants a careful model at temperature 0, a ticket sorter wants a fast and cheap one, and a team of agents often shares one choice. The spec already promised options per call (`model: "fast"`), but they were never built.

Mithlesh decided (2026-11-04) that the model belongs to the agent, in the code, not to `kavach.toml`.

## Decision

**A `Model` value, from `std.ai`, says which model and settings an `ask` uses:**

```kavach
fn careful() -> Model {
    Model(name: "claude-opus-5-5", max_tokens: 2000)
}

fn quick() -> Model {
    Model(name: "claude-haiku-5-5")
}

agent Reviewer {
    model careful()
    can Ai
    on check(text: String) -> Verdict ! Error { try ask<Verdict>(text) }
}

const label = try ask<Label>(text, model: quick())   // one call
```

- `Model` has `name`, and optional `temperature` and `max_tokens`.
- An agent's `model` line sets it for every `ask` in its handlers. Kavach has no global constants (top-level statements are `main`'s body), so a model shared by several agents is a function with no parameters: agents that call the same one are a group, changed in one place.
- A call's `model:` wins over its agent's. Without either, `ask` uses the smallest current model, `claude-haiku-5-5`, with 4,096 tokens at most.
- The compiler works the value out when it builds the program: a `Model(…)` written with plain values, a `const` set to one, or a call of such a function. Native code builds each request then.
- Measured while building it: `claude-sonnet-5-5` refuses `temperature` ("deprecated for this model"), so a `Model` with one fails there with `Failed(status: 400, …)`.
- `model` and `max_tokens` under `[ai]` in `kavach.toml` are an error that says where they went. The key stays in the environment.
- Each `Ai.ask` line of an agent's audit log says which model answered and how many tokens it took.

## Alternatives considered

- **Aliases in `kavach.toml`, named by agents and calls.** This keeps the code free of model names, but Mithlesh wants the agent to say what it uses, where it is read.
- **Settings written on the agent, one line each** (`model "…"`, `temperature 0`). This is simple, but agents can't share a choice.
- **A model chosen at run time** (a `Model` read from a file or the environment). This is not built yet. Native code would have to build each request as it runs. Programs that need it can pick between `const`s.

## Consequences

- Moving to another model is a change to one `const`, which review and `git blame` see.
- Token budgets in `limit Ai` (spec 8 open questions) can build on the counts the audit log now records.
