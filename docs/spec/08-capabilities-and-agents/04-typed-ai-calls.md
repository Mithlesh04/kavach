# 8.4 Typed AI calls: `ask<T>`

`ask<T>` sends a prompt to an AI model and returns a **checked value of type `T`**, not raw text.

```kavach
type RefundDecision {
    /// true if the refund should be issued
    approve: Bool
    /// one sentence, shown to the finance team
    reason: String
}

const decision = try ask<RefundDecision>("Should we refund order {order.id}?")
```

- Requires the `Ai` capability.
- Kavach builds an output schema from `T`. Doc comments (`///`) on types and fields become their descriptions for the model.
- The first try writes `T` into the prompt in a short form (its fields, types, choices and doc comments), which costs a third of the tokens of the provider's structured-output mode ([decision 0057](../../decisions/0057-ask-puts-the-type-in-the-prompt.md)). The response is validated against `T`. An answer that doesn't fit is asked again in structured-output mode, which holds the model to the schema (default: 2 retries), then fails with `AiError.InvalidOutput`.
- The model is chosen in the code, with a `Model` value from `std.ai` ([decision 0056](../../decisions/0056-the-model-is-chosen-in-code.md), its settings [decision 0059](../../decisions/0059-model-settings-follow-the-providers.md)):

```kavach
type Model {
    name: String               // the provider's name for the model
    effort?: Effort            // none: the model's own; Low, Medium, High, XHigh or Max
    max_tokens?: Int           // none: 4,096; at most 128,000
    thinking?: Bool            // none: the model's own; false: answer without thinking first
    instructions?: String      // none: none; standing instructions sent with every ask
    stop?: [String]            // none: none; text that ends the answer where the model writes it
    timeout?: Duration         // none: 120 seconds for each try
    retries?: Int              // none: 4; tries again after a busy or dropped call
    api_url?: String           // none: `KAVACH_AI_URL`, or Anthropic's address
    api_key_env?: String       // none: "ANTHROPIC_API_KEY"; the variable holding the key
    extra?: String             // none: none; more request fields, as a JSON object
}

type Effort = Low | Medium | High | XHigh | Max
```

```kavach
fn careful() -> Model {
    Model(
        name: "claude-opus-5-5",
        effort: Effort.High,
        instructions: "You review refunds for a shop. Be brief.",
        timeout: 60.seconds,
    )
}
```

  - `effort` is how hard the model works on an answer, and how many tokens it spends: the provider's main control of quality against cost. Without it, the model's own default.
  - `temperature` is gone: current models take only their own, and newer API versions refuse it on every model. A `Model` that sets it is a compile error that says to use `effort`.
  - `api_url` and `api_key_env` send one model's calls elsewhere, to another account, a proxy or a gateway that speaks Anthropic's Messages API. Other providers' own APIs (Amazon Bedrock, Google Vertex AI) are an open question.
  - `extra` is added to each request as it is, for a provider setting Kavach doesn't name yet: `extra: "\{\"metadata\": \{\"user_id\": \"shop-7\"}}"` (`\{` is a brace, not interpolation, 1.7). The compiler checks that it is a JSON object.

  An agent's `model` (8.3) sets it for its handlers, and a call's `model:` sets it for that call: `ask<T>(prompt, model: quick())`. The call's wins over the agent's. The compiler works the value out when it builds the program, so it is a `Model(…)` written with plain values, a `const` set to one, or a call of a function with no parameters that gives one; anything else is a compile error that says so.

  Not every model takes every setting. The compiler refuses what a model it knows can't do: `thinking: false` on `claude-opus-5-5`, which always thinks first, and `thinking: false` with `effort: Effort.XHigh` or `Effort.Max`. A setting a model refuses at run time fails the call with `AiError.Failed(status: 400, …)` and the provider's message.

  `thinking: false` asks the model to answer without thinking first, which most current models do by default. It saves most of the output tokens of a short answer, but can cost some right answers: in the agent benchmark it took 55–85% fewer output tokens, and got one answer fewer right in two of four cases, every run (`benchmarks/agents/`). So the model's own default stays, and turning it off is the program's choice.

- Without a `model`, `ask` uses Anthropic's smallest current model, `claude-haiku-5-5`, with answers of at most 4,096 tokens, reasoning included. An answer cut off at the limit fails with `Failed`, without asking again.
- A call the provider turns away because it is busy (status 429, 529 or 5xx), one the network drops, or one that takes longer than its `timeout`, is sent again after a wait that doubles each time (1, 2, 4 and 8 seconds, then 8 again), `retries` times (4 unless the `Model` says), as the providers' own libraries do; only after that does `ask` fail with `Failed`. Each wait is a random time between half and all of that, so calls turned away together don't all come back together.
- Many `ask`s at once (tasks, 9.1) can meet a provider's limit on calls at the same time. The program's `ask`s share a gate: after a call is turned away as busy (status 429), no more calls are sent at once than the others under way then, which the provider took, and each call that succeeds lets a little more through, up to 256 at once. An `ask` waiting at the gate is not a retry. So 300 `ask`s started together all get their answer, as fast as the provider allows.
- The key comes from the environment variable `ANTHROPIC_API_KEY`, or the one a `Model`'s `api_key_env` names. The variable `KAVACH_AI_URL` sends every call without an `api_url` to another address instead, such as a test server or a company proxy.
- `ask` fails with an `AiError`, from `std.ai` (like every standard type, its name needs no `use`):

```kavach
type AiError = NoKey(variable: String)                  // the key's variable is not set
    | Failed(status: Int, message: String)             // the provider refused, or the network failed (status 0)
    | InvalidOutput(answer: String)                     // still not a valid T after the retries
    | Denied(reason: String)                            // a limit on `Ai` refused the call
```

```kavach
const decision = ask<RefundDecision>(prompt) catch e => match e {
    AiError.InvalidOutput(_) => RefundDecision(approve: false, reason: "The model gave no clear answer."),
    _ => fail e,
}
```
