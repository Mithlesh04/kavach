# 8. Capabilities and Agents

This chapter is what makes Kavach different. Code must **declare what it can touch**, and agents get **guardrails that the AI model cannot bypass**.

Each section is a file of its own: new sections go at the end, with the next number.

| Section | File |
|---|---|
| 8.1 Capabilities | [01-capabilities.md](01-capabilities.md) |
| 8.2 Declaring your own capabilities | [02-declaring-capabilities.md](02-declaring-capabilities.md) |
| 8.3 Agents | [03-agents.md](03-agents.md) |
| 8.4 Typed AI calls: `ask<T>` | [04-typed-ai-calls.md](04-typed-ai-calls.md) |
| 8.5 Agent modes: earning trust step by step | [05-agent-modes.md](05-agent-modes.md) |
| 8.6 Audit log | [06-audit-log.md](06-audit-log.md) |
| 8.7 Secrets | [07-secrets.md](07-secrets.md) |
| 8.8 Running an agent | [08-running-an-agent.md](08-running-an-agent.md) |
| 8.9 Generated plans (Phase 4) | [09-generated-plans.md](09-generated-plans.md) |

## Open questions

- Fine-grained built-in capabilities beyond files, e.g. `Net("api.stripe.com")`.
- A folder that comes from a value (`uses Fs(config.dir)`), not only written text.
- How events reach agents in production: HTTP, queues, schedules. Probably a `std.ai` server helper.
- Where approval requests are delivered (webhook, Slack, email) and the timeout setting.
- Fallbacks, prices and money budgets in `limit Ai`: settled by [decision 0140](../../decisions/0140-ai-providers.md), and *not built yet*; their rules join 8.3, 8.4 and 8.6 when they are built. Providers other than Anthropic are in 8.4.
