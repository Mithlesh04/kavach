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
- Providers other than Anthropic, fallbacks, prices and `ai.ask_full` are in 8.4; money budgets in `limit Ai` in 8.3 ([decision 0140](../../decisions/0140-ai-providers.md)).
- Fallbacks (8.4), where decision 0140 is silent, for a maintainer's review: `BadRequest`, `CutOff`, `InvalidOutput`, `NoKey` and `Failed` don't move on; when every model fails, the error is the last try's; an answer that isn't a valid `T` is asked for again from the start of the list; the call counts once for `count`, with the largest `max_tokens` in the list for `UnderWay.Largest`; the audit log's line for a model moved on from names the error's variant, not its message.
- Prices, costs and money budgets (8.3, 8.4, 8.6), where decision 0140 is silent, for a maintainer's review: a program gives its own prices by wrapping its provider, as the decision's example does, with nothing new to learn; `price` is worked out when the program is built, as `check` is, so a built program keeps the prices of its build; Anthropic's table has only the four models whose names the pricing page gives, and `claude-haiku-5-5` at its price for prompts up to 100,000 tokens (a longer prompt costs five times as much, so its estimate is low); cache writes are counted at the `input` price, though Anthropic bills them at 1.25 or 2 times it; `a.model` is the model's name, not its `Model`; a usage count is the sum of the tries that gave it, `none` when none did; an `ask` that reached a model counts in `ai.spent` whether it answered or failed, and one with no known cost goes into `unpriced`; `ai.spent` needs no capability; `cost <=` refuses an `ask` whose list has any model with no price, even when an earlier one would answer; a rule can't read both `tokens` and `cost`; a cost the reply gave no counts for counts as 0 in a limit; the refusal of a `cost` limit gives the window's cost so far.
