# 0060. An agent's settings are chosen in code

- **Status:** Accepted (Mithlesh's choices, 2026-11-04; the field names and the exceptions for single files, tests and dry runs are for review)
- **Date:** 2026-11-04
- **Builds on:** [decision 0056](0056-the-model-is-chosen-in-code.md); changes spec 8.5, 8.6 and 10.11

## Context

An agent's mode, audit log, state file and journal were set in `kavach.toml`, under `[agents.<Name>]`, one table per agent, each a string to spell right. Decision 0056 moved the model into code, where a function shares it and a typo is a compile error. Mithlesh asked for the same here: "part of agent itself … setting which can shared across the agents with their own overwrite … make developers life easily". And for defaults: "only on those by default which required to survives restarts".

## Decision

1. **Settings in code.** An agent writes `settings <expr>` after its `model`, an `AgentSettings` from `std.ai` (`mode`, `audit`, `state`, `journal`, `folder`). A function with no parameters shares them; `with { mode: Observe }` changes fields for one agent. They are worked out when the program is built.
2. **What survives a restart is on.** In a project, state and journal are on by default (Mithlesh, 2026-11-04), in `agents/<Name>/` beside `kavach.toml`, outside `.kavach/`, which is build output. The audit log is off until `audit: Audit(…)` turns it on.
3. **`kavach.toml` keeps only where.** `[agents] folder` moves every agent's folder, and an agent's own `folder` moves its own (Mithlesh's choice: the toml sets it, code overrides it). `[agents.<Name>]` is an error that says where each key went.
4. **Exceptions:** a program with no `kavach.toml` has no project folder, so it keeps no state or journal unless its settings name a `folder`; `kavach test` and `--dry-run` keep none, so a test starts fresh and a simulated result is never replayed by a real run.

```kavach
fn standard() -> AgentSettings {
    AgentSettings(audit: Audit(format: JsonLines))
}

agent RefundBot {
    model careful()
    settings standard() with { mode: Observe }
    can Payments.refund
}
```

## Consequences

- An agent survives a restart with nothing set; its handlers must be safe to run again from the start, which they are when they reach the world only through capabilities (decision 0025).
- Every agent in a project writes files in `agents/`; `kavach new`'s `.gitignore` leaves it out.
- Programs with `[agents.<Name>]` stop compiling, with a message per key.

## Alternatives considered

- **Keep `[agents.<Name>]`.** Settings far from the agent, strings a typo silently breaks, and nothing to share between agents. Mithlesh chose code.
- **Journal off by default.** Safer for handlers that aren't safe to replay, but an agent then loses half-done work in a crash unless someone knew to turn it on. Mithlesh chose on.
- **The folder in `.kavach/`.** That is build output; anything that clears it would wipe the agents' memory.
