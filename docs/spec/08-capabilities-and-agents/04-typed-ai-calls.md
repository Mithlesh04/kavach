# 8.4 Typed AI calls: `ask<T>`

`ask<T>` sends a prompt to an AI model and returns a **checked value of type `T`**, not raw text.

```kavach
type RefundDecision {
    /// true if the refund should be issued
    approve: Bool
    /// one sentence, shown to the finance team
    reason: String
}

const decision = try ask<RefundDecision>(`Should we refund order {order.id}?`)
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
    api_url?: String           // none: `KAVACH_AI_URL`, or the provider's address
    api_key_env?: String       // none: the provider's, "ANTHROPIC_API_KEY" for Anthropic
    extra?: String             // none: none; more request fields, as a JSON object
    provider?: Provider        // none: Anthropic(); whose API the model is reached through
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
  - `api_url` and `api_key_env` send one model's calls elsewhere, to another account, a proxy or a gateway that speaks the provider's API.
  - `provider` says whose API the model is reached through: `Anthropic()` from `std.ai` unless the `Model` names another (*Providers*, below).
  - `extra` is added to each request as it is, for a provider setting Kavach doesn't name yet: `` extra: `\{"metadata": \{"user_id": "shop-7"}}` ``, on one line or over several (`\{` is a brace in a template, 1.7). The compiler checks that it is a JSON object.

  An agent's `model` (8.3) sets it for its handlers, and a call's `model:` sets it for that call: `ask<T>(prompt, model: quick())`. The call's wins over the agent's. Either can be a list of models, tried in turn (*Fallbacks*, below). The compiler works the value out when it builds the program, so it is a `Model(…)` written with plain values, a `const` set to one, or a call of a function with no parameters that gives one; anything else is a compile error that says so.

  Not every model takes every setting. The compiler refuses what a model it knows can't do: `thinking: false` on `claude-opus-5-5`, which always thinks first, and `thinking: false` with `effort: Effort.XHigh` or `Effort.Max`. A setting a model refuses at run time fails the call with `AiError.BadRequest` and the provider's message.

  `thinking: false` asks the model to answer without thinking first, which most current models do by default. It saves most of the output tokens of a short answer, but can cost some right answers: in the agent benchmark it took 55–85% fewer output tokens, and got one answer fewer right in two of four cases, every run (`benchmarks/agents/`). So the model's own default stays, and turning it off is the program's choice.

- Without a `model`, `ask` uses Anthropic's smallest current model, `claude-haiku-5-5`, with answers of at most 4,096 tokens, reasoning included. An answer cut off at the limit fails with `CutOff`, without asking again.
- A call the provider turns away as rate limited or overloaded, one the network drops, or one that takes longer than its `timeout`, is sent again after a wait that doubles each time (1, 2, 4 and 8 seconds, then 8 again), `retries` times (4 unless the `Model` says), as the providers' own libraries do; only after that does `ask` fail, with `RateLimited`, `Overloaded`, `Network` or `TimedOut`. Each wait is a random time between half and all of that, so calls turned away together don't all come back together. When the provider says how long to wait (Anthropic's `retry-after` header), the wait is at least that long, but never longer than the `Model`'s `timeout`.
- What is sent again depends on the kind of trouble, which the provider reads from its answer, not on the status alone: `QuotaUsedUp`, `BadKey`, `NotFound`, `BadRequest`, `Refused` and `CutOff` are never sent again, as waiting can't change them.
- Many `ask`s at once (tasks, 9.1) can meet a provider's limit on calls at the same time. The `ask`s that go to one provider with one key share a gate: after a call is turned away as rate limited, no more calls are sent at once than the others under way then, which the provider took, and each call that succeeds lets a little more through, up to 256 at once. An `ask` waiting at the gate is not a retry. So 300 `ask`s started together all get their answer, as fast as the provider allows. Each provider and key has a gate of its own, so one provider's limit never slows the calls to another, nor to the same provider with another key.
- The key comes from the environment variable a `Model`'s `api_key_env` names, or else the provider's own (`ANTHROPIC_API_KEY` for Anthropic), looked up as the program sees variables: `--env`, the shell's environment, `[env.vars]` and the project's `.env` file (10.13). So a key kept in `.env`, out of the code and the repository, is found. A provider that needs no key, such as a server on the same machine, reads no variable.
- The variable `KAVACH_AI_URL` sends every call whose `Model` has no `api_url` to another address instead, such as a test server or a company proxy. The call keeps the path the provider wrote (`/v1/messages`), after `KAVACH_AI_URL`'s own path, and goes to `KAVACH_AI_URL`'s scheme, host and port.
- `ask` fails with an `AiError`, from `std.ai` (like every standard type, its name needs no `use`). Each kind of trouble has a variant of its own, which means the same for every provider ([decision 0140](../../decisions/0140-ai-providers.md)):

```kavach
type AiError = NoKey(variable: String)                 // the key's variable is not set
    | RateLimited(message: String)                    // still too many calls after the retries
    | QuotaUsedUp(message: String)                    // the account's money or quota ran out
    | Overloaded(message: String)                     // the provider is busy, after the retries
    | BadKey(message: String)                         // the key is wrong, ran out, or may not do this
    | NotFound(model: String)                         // no such model here
    | BadRequest(message: String)                     // a setting or a schema the provider refused
    | Refused(reason: String?)                        // the model or a filter refused to answer
    | CutOff(max_tokens: Int)                         // the answer stopped at `max_tokens`
    | TimedOut(after: Duration)                       // no answer within `timeout`, after the retries
    | Network(message: String)                        // no connection, or it dropped, after the retries
    | InvalidOutput(answer: String)                   // still not a valid T after the retries
    | Denied(reason: String)                          // a limit on `Ai` refused the call
    | Failed(status: Int, message: String)            // anything else
```

  How Anthropic's answers map to these:

  | Anthropic's answer | `AiError` | Sent again? |
  |---|---|---|
  | status 429 | `RateLimited` | yes |
  | status 529, 500, or any other 5xx but 504 | `Overloaded` | yes |
  | status 504, or no answer within `timeout` | `TimedOut` | yes |
  | no connection, or it dropped | `Network` | yes |
  | status 402, or an error of type `billing_error` | `QuotaUsedUp` | no |
  | status 401 or 403 | `BadKey` | no |
  | status 404 | `NotFound` | no |
  | status 400 or 413 | `BadRequest` | no |
  | `stop_reason: "refusal"` | `Refused` (with no reason: Anthropic gives none) | no |
  | `stop_reason: "max_tokens"` | `CutOff` | no |
  | any other status | `Failed` | no |

```kavach
const decision = ask<RefundDecision>(prompt) catch e => match e {
    AiError.InvalidOutput(_) => RefundDecision(approve: false, reason: "The model gave no clear answer."),
    _ => fail e,
}
```

## Providers

A provider is how `ask` speaks to one AI service's API. It is a Kavach interface from `std.ai`, which `std.ai`'s `Anthropic` implements, and so can a program or a package ([decision 0140](../../decisions/0140-ai-providers.md)). So a new provider, or a change in a provider's API, needs new Kavach code, not a new Kavach release.

```kavach
/// How `ask` talks to one AI provider: it only translates.
pub interface Provider {
    /// The settings of `model` this provider can't take, each with why.
    fn check(self, model: Model) -> [String]
    /// The variable holding the key when the `Model` names none.
    fn key_variable(self) -> String
    /// Where the key goes in a request; the runtime puts it there.
    fn auth(self, model: Model) -> Auth
    /// The request for one `ask`.
    fn request(self, call: AskCall) -> HttpCall
    /// What came back: an answer, or why there is none.
    fn read(self, call: AskCall, reply: HttpReply) -> Reply
    /// The price of a model's tokens; none: not known (*Prices*, below).
    fn price(self, model: String) -> Price?
}

/// One `ask`, as the provider sees it.
pub type AskCall {
    model: Model
    prompt: String
    schema: Json        // the JSON Schema built from `T`
    strict: Bool        // true: hold the answer to `schema` with the provider's own mode;
                        // false: the type is already written into `prompt` (decision 0057)
}

pub type HttpCall { url: String, headers: {String: String}, body: String }
pub type HttpReply { status: Int, headers: {String: String}, body: String }

/// Where the key goes. The runtime reads it and puts it there.
pub type Auth = NoKey                    // a server that takes none
    | Bearer                             // `authorization: Bearer <key>`
    | InHeader(name: String)             // `x-api-key`, `api-key`, `x-goog-api-key`
    | AwsSignature(service: String, region: String)
    | GoogleToken(scope: String)

pub type Reply = Answer(text: String, usage: Usage)
    | Refused(reason: String?, usage: Usage)   // the model or a filter said no
    | CutOff(usage: Usage)                     // stopped at `max_tokens`
    | Problem(kind: Trouble, message: String, wait: Duration?)

/// What went wrong, as the provider means it, not as its status says.
pub type Trouble = RateLimited | QuotaUsedUp | Overloaded | BadKey
    | NotFound | BadRequest | TimedOut | Other(status: Int)

/// Tokens as the provider counts them; none: it didn't say.
pub type Usage {
    input?: Int          // every input token, cached ones included
    output?: Int         // every output token, thinking included
    cached_input?: Int
    thinking?: Int
}
```

- **A provider only translates.** Its methods are plain functions: they use no capabilities, so provider code can't send anything itself. The runtime does everything else, for every provider alike: it reads the key and puts it where `auth` says, so provider code never holds it; and it sends the request, waits, retries, keeps the gates, applies `limit Ai`, and writes the journal (8.5) and the audit log (8.6). A dry run or a generated test (8.5, 11.5) calls no provider method.
- **`request`** is called once for the first try of an `ask`, with `strict: false`, and once more for each try after an answer that isn't a valid `T`, with `strict: true`. A request turned away as busy is sent again as it was.
- **`read`** is called for every reply, whatever its status. Header names in `reply.headers` are in lowercase. The provider decides what the reply means: an `Answer`'s `text` is checked against `T` (it may have text or a code fence around its JSON), and a `Problem`'s `kind` decides whether the call is sent again (above). `wait` is the provider's own hint, read from wherever it gives one; the runtime waits at least that long, but never longer than the `Model`'s `timeout`. A connection that fails or a `timeout` that passes never reaches `read`: the runtime makes those `Network` and `TimedOut` itself.
- **`Usage`** means the same for every provider: the adapter adds what its provider leaves out (cached tokens, thinking) back in. A count the reply doesn't have is `none`, never a made-up number; `limit Ai` and the audit log count it as 0.
- **`check`** runs when the program is built, on each `Model` the program writes, with its provider. Each text it gives is a compile error at that `Model`. `Anthropic`'s refuses `thinking: false` on `claude-opus-5-5`, and `thinking: false` with `effort: Effort.XHigh` or `Effort.Max` (above). A `check` that panics is a compile error that says so.
- **A `Model`'s provider is worked out when the program is built,** as the rest of the `Model` is: a type's constructor written with plain values (`Anthropic()`, `ShopAi(base: Anthropic(), team: "eu")`), a `const` set to one, or a call of a function with no parameters that gives one. Anything else is a compile error that says so.
- **`Anthropic()`** speaks Anthropic's Messages API: `POST <api_url>/v1/messages` (`https://api.anthropic.com` unless the `Model` says), `anthropic-version: 2023-06-01`, the key in `x-api-key`, and the settings of `Model` as above. With `strict: true` the schema goes in `output_config.format`.
- **Changing one part.** An interface's methods have no default bodies (2.8), so a program changes one part of a provider by wrapping it and passing the other methods on:

```kavach
/// Anthropic, through the shop's gateway, which wants its own header.
type Gateway: Provider {
    base: Anthropic

    fn request(self, call: AskCall) -> HttpCall {
        const r = self.base.request(call)
        let headers = r.headers
        headers["x-shop-team"] = "refunds"
        HttpCall(url: r.url, headers: headers, body: r.body)
    }
    fn check(self, model: Model) -> [String] { self.base.check(model) }
    fn key_variable(self) -> String { "SHOP_GATEWAY_KEY" }
    fn auth(self, model: Model) -> Auth { self.base.auth(model) }
    fn read(self, call: AskCall, reply: HttpReply) -> Reply { self.base.read(call, reply) }
    fn price(self, model: String) -> Price? { self.base.price(model) }
}

fn gateway() -> Model {
    Model(name: "claude-haiku-5-5", provider: Gateway(base: Anthropic()),
          api_url: "https://ai.shop.example")
}
```

- The key goes to the address in the provider's request. So a package's provider decides where a program's key is sent, as a package's code does with any secret it is given: use providers from packages you trust.

## Fallbacks: a list of models

`model` takes a list of models too, tried in turn: `model [careful(), backup()]` in an agent, `ask<T>(prompt, model: [careful(), backup()])` in a call ([decision 0140](../../decisions/0140-ai-providers.md), choice 3). The first model is asked first. When it can't answer for a reason another model may not have, such as a rate limit or a bad key, the call goes to the next one. Several keys of one provider are the same model with another `api_key_env`:

```kavach
fn careful() -> Model {
    Model(name: "claude-opus-5-5", effort: Effort.High)
}

/// The same model, through the team's second account.
fn careful_second_key() -> Model {
    Model(name: "claude-opus-5-5", effort: Effort.High, api_key_env: "ANTHROPIC_API_KEY_2")
}

fn backup() -> Model {
    Model(name: "claude-haiku-5-5")
}

agent Reviewer {
    model [careful(), careful_second_key(), backup()]
    can Ai
    on check(text: String) -> Verdict ! Error { try ask<Verdict>(text) }
}
```

- **How it is written.** A list literal, `[…]`, where a model goes. Each item is a model written as above: a `Model(…)` with plain values, a `const` set to one, or a call of a function with no parameters that gives one. A model may be in the list more than once. An empty list is a compile error, `a list of models needs at least one model`. Each model's provider `check`s it (*Providers*, above).
- **When it moves on:** after `RateLimited`, `QuotaUsedUp`, `Overloaded`, `BadKey`, `NotFound`, `TimedOut` or `Network`. Never after `Refused`, so a fallback can't be used to get round a safety filter, nor after `Denied`, which is a limit: these fail the call as they do with one model. `BadRequest`, `CutOff`, `InvalidOutput`, `NoKey` and `Failed` don't move on either: they come from the request or the program's setup, which the next model shares (for a maintainer's review).

```kavach
const v = ask<Verdict>(text, model: [careful(), backup()]) catch e => match e {
    AiError.Refused(_) => Verdict(risky: true, reason: "refused"),   // backup() was never asked
    _ => fail e,
}
```

- **At once.** A model that is busy (`RateLimited`, `Overloaded`, `TimedOut`, `Network`) hands the call to the next model at once, so a second key never waits while it is free. The waits above start only when every model left is busy: the call waits until the first of their waits is over, then tries the models in turn again, each one whose wait is over. Each model waits and is sent again as its own `Model` says (`retries`, `timeout`, its provider's hint). A model is left out for the rest of the call once its `retries` are used up, or after `QuotaUsedUp`, `BadKey` or `NotFound`, which waiting can't fix.
- **When every model fails,** the call fails with the error of its last try, which is the last model's when the models are tried once each (for a maintainer's review: decision 0140 says "the last model's").
- An answer that isn't a valid `T` is asked for again (above) from the start of the list: the first model left whose wait is over (for a maintainer's review).
- **Limits and gates.** However many models it tries, the call is one `ask`: `count` in `limit Ai` (8.3) counts it once, and `tokens` counts the tokens of every try. `UnderWay.Largest` counts it with the largest `max_tokens` in the list (for a maintainer's review). Each try passes the gate of its own provider and key (above), so a model that is busy never slows the next one.
- **The audit log** (8.6) has a line for each model the call moved on from, then one for the model that answered, or that was tried last.
- A dry run or a generated test (8.5, 11.5) asks no model; its audit line names the first model in the list.

- Every try's tokens and cost count, whichever model gave them (*Usage and cost*, below).

## Prices

Each provider gives the price of a model's tokens with its `price` method ([decision 0140](../../decisions/0140-ai-providers.md), choice 4). Prices are in US dollars, the currency of the providers' price pages, and each says when it was known and where from:

```kavach
/// The price of a model's tokens, in US dollars.
pub type Price {
    input: Decimal              // per `per` input tokens
    output: Decimal             // per `per` output tokens, thinking included
    cached_input?: Decimal      // per `per` input tokens read from the provider's cache; none: as `input`
    per: Int                    // usually 1,000,000
    as_of: Date                 // the day the price was read
    from: PriceSource
}

pub type PriceSource = Kavach | Program | Package(name: String)
```

- **`Anthropic()`'s prices** are a table in `std.ai`, dated, with its source: Anthropic's pricing page (`https://platform.claude.com/docs/en/about-claude/pricing`), read on 2026-11-22. It has the models of the current line whose names that page gives: `claude-fable-5-2`, `claude-opus-5-5`, `claude-sonnet-5-5` and `claude-haiku-5-5`. Any other name has no price.
- **A program's own prices win.** A program gives a price by wrapping the provider (*Changing one part*, above) and answering `price` for the models it knows, with `from: PriceSource.Program`, passing the others on:

```kavach
type ShopAi: Provider {
    base: Anthropic

    fn price(self, model: String) -> Price? {
        if model == "claude-haiku-5-5" {
            // The shop's negotiated price.
            return Price(input: 0.08, output: 0.40, per: 1_000_000,
                         as_of: Date(year: 2026, month: 11, day: 1), from: PriceSource.Program)
        }
        self.base.price(model)
    }
    // … the other methods, each passed to `self.base`
}
```

- **A price is worked out when the program is built,** as `check` is (*Providers*, above): once for each model the program names, and for the default model. So a built program carries the prices of its build, and their date says how old they are.
- **A model with no known price** has a cost of `none`, never 0.
- **A cost is an estimate.** A price is known as of a date and may have changed; a program's own price is only as good as what it gave; the provider's bill is what counts. So every cost carries a note that says so, and printing a cost prints it (*Usage and cost*, below).

## Usage and cost: `ai.ask_full<T>`

`ai.ask_full<T>` asks as `ask<T>` does, with the same `model:`, retries, fallbacks, limits and errors, and gives the answer with what it took ([decision 0140](../../decisions/0140-ai-providers.md), choice 7). `ask<T>` stays as it is.

```kavach
use std.ai

const a = try ai.ask_full<Verdict>(`Is this clause risky? {clause}`)
print(a.value.risky)
print(a.usage.output)     // 96
print(a.model)            // claude-haiku-5-5
print(a.cost)             // about $0.0000892 (estimate: prices as of 2026-11-22; the provider's bill is exact)
```

```kavach
/// An answer, with what it took.
pub type Asked<T> {
    value: T
    usage: Usage           // every try's tokens, added up
    cost: Cost?            // none: a model it used has no known price, or a reply gave no counts
    model: String          // the model that answered
}

/// What an `ask` cost, worked out from prices: an estimate.
pub type Cost: Display {
    dollars: Decimal       // in US dollars
    note: String           // how far to trust it
}
```

- **`usage`** adds up the `Usage` of every try that got a reply: an answer asked for again because it wasn't a valid `T`, and each model of a list. A count no reply gave is `none`.
- **`cost`** is each try's tokens at its own model's price: input tokens not read from the cache at `input`, those read from it at `cached_input`, and output tokens at `output`, each divided by `per`. It is `none` when a try's model has no price, or its reply had no input or output count.
- **Printing a cost** prints `about $<dollars> (<note>)`. The note is `estimate: prices as of <date>; the provider's bill is exact`, with the oldest date of the prices used; a program's own price says `the program's prices as of <date>`, and a package's `prices from <name> as of <date>`.

### Totals per run and per agent

`ai.spent()` gives what every `ask` of the run took so far, and `ai.spent(by: "Reviewer")` what one agent's took, since the program started:

```kavach
/// What the `ask`s of a run, or of one agent, took.
pub type Spent {
    asks: Int              // the asks that reached a model
    usage: Usage           // their tokens, added up
    cost: Cost?            // the asks whose cost is known; none: no such ask
    unpriced: Int          // the asks whose cost isn't known, left out of `cost`
}

const s = ai.spent()
print(`{s.asks} asks, {s.usage.input ?? 0} tokens in, cost {s.cost}`)
```

- An `ask` counts once it reached a model, whether it gave an answer or failed. A dry run or a generated test (8.5, 11.5) reaches none, so it adds nothing.
- `ask` and `ai.ask_full` count alike. The totals last while the program runs; a limit's totals across restarts are the agent's state file (8.3, 8.5).
- `ai.spent` needs no capability.
- A cost left out because it isn't known is counted in `unpriced`, and the note says so: `estimate: prices as of 2026-11-22; 2 asks with no known cost left out; the provider's bill is exact`.

*Not built yet* (decision 0140 sets how each works): the other providers, in a package of their own repository, starting with OpenAI-shaped Chat Completions (OpenAI, Azure, Ollama, llama.cpp, vLLM), then Gemini, Bedrock, Vertex and OpenAI's Responses API; `Auth.AwsSignature` and `Auth.GoogleToken`, for now an `ask` whose provider gives one fails with `Failed(status: 0, …)`; counting a prompt's tokens before it is sent (`ai.count`). The interface gains the methods it needs (`count_request`, `read_count` and `estimate`) when it is built.
