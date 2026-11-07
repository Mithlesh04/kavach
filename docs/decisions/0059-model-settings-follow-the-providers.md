# 0059. A `Model`'s settings follow the providers

- **Status:** Accepted (Mithlesh's choices, 2026-11-04, after research on Anthropic's documentation, LangChain and LiteLLM; the field names and the details are for review)
- **Date:** 2026-11-04
- **Builds on:** [decision 0056](0056-the-model-is-chosen-in-code.md); changes spec 8.4

## Context

`Model` had a name, `temperature`, `max_tokens` and `thinking`. Providers have moved on:

- Current Claude models take only their own `temperature`, and from API version 2026-10-21 every model refuses it, at any value.
- `effort` (`low` to `max`) is now the main control of quality against cost.
- `claude-opus-5-5` can't answer without thinking: it refuses every way of turning it off, so `thinking: false` failed there with status 400.
- Frameworks let each model set its own address, key, timeout and retries (LangChain's `ChatOpenAI`, LiteLLM), where Kavach had one `KAVACH_AI_URL` and one `ANTHROPIC_API_KEY` for the whole program, a fixed 120 s deadline and 4 tries.

## Decision

`Model` gets `effort`, `instructions` (a system prompt), `stop` (stop sequences), `timeout`, `retries`, `api_url`, `api_key_env` and `extra` (request fields Kavach doesn't name, as a JSON object). `temperature` is removed: setting it is a compile error that names `effort`. The compiler refuses `thinking: false` on a model that can't turn it off, and with `effort: XHigh` or `Max`, which no model accepts with thinking off.

```kavach
fn careful() -> Model {
    Model(name: "claude-opus-5-5", effort: High, timeout: 60.seconds)
}
```

## Consequences

- A program can set each model's cost, address and patience in code, next to the agents that use it.
- `api_url` reaches anything that speaks Anthropic's Messages API (a proxy, a gateway, another account). Bedrock and Vertex need their own request signing: an open question, not part of this.
- `extra` lets a new provider feature in before Kavach names it; it is written into the request as it is, so a wrong field fails at the provider.
- Programs that set `temperature` stop compiling, with a message that says what to write instead.

## Alternatives considered

- **Deprecate `temperature` instead of removing it.** Every current model refuses other values, so a deprecated field would only fail later, at run time. Mithlesh chose removal.
- **A `Provider` union (Anthropic, Bedrock, Vertex, …).** It names providers Kavach can't call yet; an address and a key variable cover what works today, and the union can come with the first second provider.
- **`extra` as a `Json` value.** A model is worked out when the program is built, from plain values; JSON text is a plain value, and the compiler parses it then.
