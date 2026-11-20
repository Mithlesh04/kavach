# 0140. AI providers other than Anthropic

- **Status:** Accepted (a maintainer, 2026-11-19: the frame as written, and the recommended option on all ten choices; the frame itself is a maintainer's, from to-do item 17, 2026-11-16)
- **Date:** 2026-11-19
- **Builds on:** [decision 0056](0056-the-model-is-chosen-in-code.md), [decision 0057](0057-ask-puts-the-type-in-the-prompt.md), [decision 0059](0059-model-settings-follow-the-providers.md); would change spec 8.3, 8.4 and 8.6

## Context

Today `ask<T>` (spec 8.4) speaks one API: Anthropic's Messages API. The runtime (`runtime/net/src/ai.rs`) writes the request, reads the answer and its usage, retries a busy call, and slows every `ask` at one shared gate after a 429. A `Model` can name another address (`api_url`) and another key variable (`api_key_env`), so a proxy or a second account works. Any other provider does not.

A maintainer set the frame (to-do item 17, 2026-11-16):

- A provider is an **interface** that a program or a package implements in Kavach. It covers how a request is written, how an answer, its usage and its errors are read, and how its tokens are counted and priced.
- Kavach's own adapters are **only defaults**. A program can replace them, or change one part. A provider's change, or a new provider, needs new Kavach code in a package or the program, not a new Kavach release.
- A detail a provider doesn't give (tokens, cost, a reason) is **`none`**, never a made-up number.
- **Retries and fallbacks**, and errors a program can tell apart.
- **Token counts and prices**, with every estimate saying how far to trust it.

### What the providers do today

We read each provider's official documentation on 2026-11-19. They differ in every part the interface must cover.

| | OpenAI Chat Completions | OpenAI Responses | Azure OpenAI (v1) | Gemini API | Vertex AI | Bedrock Converse | Ollama, llama.cpp, vLLM |
|---|---|---|---|---|---|---|---|
| Key | `Authorization: Bearer` | the same | `api-key`, or an Entra token | `x-goog-api-key` | OAuth token, 1 hour | SigV4, or a Bedrock API key (`Bearer`) | none, or `--api-key` |
| Instructions | a `developer` message | `instructions` | as OpenAI | `systemInstruction` | as Gemini | `system` | `system` message |
| Most output tokens | `max_completion_tokens` | `max_output_tokens` (at least 16) | as OpenAI | `maxOutputTokens`, thinking included | as Gemini | `inferenceConfig.maxTokens` | `max_tokens` |
| Stop text | `stop`, 4 at most | **none** | as OpenAI | `stopSequences`, 5 at most | as Gemini | `stopSequences` | `stop` |
| Effort | `reasoning_effort`, `none` to `max` | `reasoning.effort` | as OpenAI | `thinkingLevel`, `MINIMAL` to `XHIGH` | as Gemini, budget refused on Gemini 4 | in `additionalModelRequestFields` | differs per server |
| Schema mode | `response_format.json_schema` | `text.format` (flat) | as OpenAI | `responseFormat` (new) or `responseJsonSchema` | `responseFormat` is a list there | `outputConfig.textFormat`, the schema **as a string** | `response_format`, held by a grammar |
| Schema rules | every field required, optional as `null` | the same | the same | some keywords dropped silently | the same | `additionalProperties: false`, 12 optional fields at most | llama.cpp drops unknown keywords silently |
| Refusal | `message.refusal` | a `refusal` content part | also `content_filter` | no candidates, `promptFeedback.blockReason` | the same | `stopReason: guardrail_intervened`, `content_filtered` | none |
| Usage | `prompt_tokens`, `completion_tokens` | `input_tokens`, `output_tokens` | as OpenAI | `promptTokenCount`, `candidatesTokenCount` + `thoughtsTokenCount` | as Gemini | `inputTokens` (cached tokens **not** included) | as OpenAI, or none |
| Count first | none | `POST /v1/responses/input_tokens` | not listed | `:countTokens` | `:countTokens`, free | `POST /model/{id}/count-tokens`, free | llama.cpp and vLLM `/tokenize`; Ollama none |
| Wait hint | `Retry-After` (seconds), sometimes | the same | `retry-after-ms` | `RetryInfo.retryDelay` in the body, maybe | the same | `x-amz-retry-after` (ms) | none |
| Prices | no list | no list | a retail price API, by meter | no list | billing SKUs, not by model | a price list, Claude through Marketplace | free |

Things that matter most for the design:

- **One status means different things.** OpenAI's 429 is either "slow down" (retry) or "the money ran out" (`insufficient_quota`, never retry). Gemini's `RESOURCE_EXHAUSTED` is 402 (no retry) or 429 (retry). A bad Gemini key is a **400** with `API_KEY_INVALID`, not a 401. Bedrock's `ThrottlingException` is listed as 429 in one place and 400 in another, and `ModelNotReadyException` is a 429 to retry. So each adapter, not the runtime, must say what an error means.
- **A failure can come with status 200.** A Responses call can be `status: failed` with an `error`. A blocked Gemini prompt is a 200 with no candidates. A filtered Azure answer is a 200 with `finish_reason: content_filter`.
- **Every structured-output mode has its own schema rules,** and some drop keywords silently. Kavach checks every answer against `T` anyway (decision 0057), so the adapter rewrites the schema to fit, and the check stays the last word.
- **Tokens are counted differently.** Gemini bills thinking as output but leaves it out of `candidatesTokenCount`. Bedrock and Anthropic leave cached tokens out of the input count; OpenAI includes them.
- **Azure's two 429s** (the quota, or "high demand") differ only in the message text.
- **A new schema can be slow.** Bedrock compiles each new schema once, which can take about 90 seconds, so its first call needs a long `timeout`. A local server loads its model on the first call too.
- **No provider gives prices by model in a form a program can read.**
- **Local servers speak Anthropic's API too.** Ollama, llama.cpp and vLLM all serve `/v1/messages`, so today's `ask` can reach them with `api_url`. But they hold the answer to a schema only on their OpenAI-shaped endpoint, if at all.
- **Cloud keys don't stay put.** A Vertex token lasts an hour. Bedrock signs each request (SigV4), or takes an API key that lasts 12 hours.

## Decision

The frame below, then the ten choices a maintainer made on 2026-11-19.

### 1. A provider is a Kavach interface that translates; the runtime sends

The provider only **translates**: a `Model` and a prompt into an HTTP request, and a reply into an answer, usage or an error. Its methods are plain functions with no capabilities. The runtime keeps everything else in one place, as today: reading the key, sending, timeouts, retries and waits, the gates, `limit Ai`, `--dry-run` and the audit log.

So a provider from a package can't send a prompt anywhere else, can't see the key, and can't skip a limit. And a new provider is only Kavach code.

```kavach
/// How `ask` talks to one AI provider. Kavach's adapters implement it, and
/// so can a program or a package (decision 0140).
pub interface Provider {
    /// The settings of `model` this provider can't take, each with why.
    /// The compiler runs it when it builds the program, like a tagged
    /// literal's checker (decision 0100).
    fn check(self, model: Model) -> [String]
    /// Where the key goes in a request; the runtime puts it there.
    fn auth(self, model: Model) -> Auth
    /// The request for one try of an `ask`.
    fn request(self, call: AskCall) -> HttpCall
    /// What came back: an answer, or why there is none.
    fn read(self, call: AskCall, reply: HttpReply) -> Reply
    /// The request that counts a prompt's tokens; none: the provider has none.
    fn count_request(self, call: AskCall) -> HttpCall?
    /// The count in that request's reply; none: the reply had none.
    fn read_count(self, reply: HttpReply) -> Int?
    /// A count made without the network; none: no way to make one.
    fn estimate(self, call: AskCall) -> Estimate?
    /// The price of a model's tokens; none: not known.
    fn price(self, model: String) -> Price?
}
```

The types it uses:

```kavach
/// One `ask`, as the provider sees it.
pub type AskCall {
    model: Model
    prompt: String
    /// The JSON Schema built from `T`.
    schema: Json
    /// true: hold the answer to the schema with the provider's own mode;
    /// false: the schema is already in the prompt (decision 0057).
    strict: Bool
}

pub type HttpCall { url: String, headers: Dict<String, String>, body: String }
pub type HttpReply { status: Int, headers: Dict<String, String>, body: String }

/// Where the key goes. The runtime reads it from `api_key_env` and puts it
/// there, so provider code never holds it.
pub type Auth = NoKey                       // a local server
    | Bearer                                // `Authorization: Bearer <key>`
    | InHeader(name: String)                // `x-api-key`, `api-key`, `x-goog-api-key`
    | AwsSignature(service: String, region: String)  // SigV4 (choice 5)
    | GoogleToken(scope: String)            // OAuth from a service account (choice 5)

pub type Reply = Answer(text: String, usage: Usage)
    | Refused(reason: String?, usage: Usage)    // the model or a filter said no
    | CutOff(usage: Usage)                      // stopped at `max_tokens`
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

- **`wait`** is the provider's own hint, read from wherever it puts it: `Retry-After`, `retry-after-ms`, `x-amz-retry-after` or Google's `RetryInfo`. The runtime waits at least that long, but no longer than the `Model`'s `timeout`.
- **`Usage`** is in one meaning for every provider: an adapter adds Bedrock's cached tokens back into `input`, and Gemini's thinking into `output`. What the reply lacks stays `none`.
- **Changing one part.** An interface's methods have no default bodies (spec 2.8), so a program changes one part by wrapping Kavach's adapter and passing the other methods through:

```kavach
/// OpenAI, with the price of this shop's own fine-tuned model.
type ShopOpenAi: Provider {
    base: OpenAi

    fn price(self, model: String) -> Price? {
        match model {
            "ft:gpt-5.5:shop" => Price(input: 1.10, output: 8.80, per: 1_000_000,
                               as_of: Date(2026, 11, 1), from: PriceSource.Program),
            _ => self.base.price(model),
        }
    }
    fn check(self, model: Model) -> [String] { self.base.check(model) }
    // … and the other methods, each passed to `self.base`
}
```

### 2. Errors a program can tell apart

`AiError` gets one variant for each kind of trouble. Each adapter decides the kind (above), so the meaning is the same for every provider.

```kavach
type AiError = NoKey(variable: String)                 // the key's variable is not set
    | RateLimited(message: String)                    // still too many calls after the retries
    | QuotaUsedUp(message: String)                    // the account's money or quota ran out
    | Overloaded(message: String)                     // the provider is busy (503, 529), after the retries
    | BadKey(message: String)                         // the key is wrong, ran out, or may not do this
    | NotFound(model: String)                         // no such model here (a local one not pulled)
    | BadRequest(message: String)                     // a setting or a schema the provider refused
    | Refused(reason: String?)                        // the model or a filter refused to answer
    | CutOff(max_tokens: Int)                         // the answer stopped at `max_tokens`
    | TimedOut(after: Duration)                       // no answer within `timeout`, after the retries
    | Network(message: String)                        // no connection, or it dropped
    | InvalidOutput(answer: String)                   // still not a valid T after the retries
    | Denied(reason: String)                          // a limit on `Ai` refused the call
    | Failed(status: Int, message: String)            // anything else
```

| Kind | OpenAI, Azure | Gemini, Vertex | Bedrock | Local servers | Retried? |
|---|---|---|---|---|---|
| `RateLimited` | 429 `rate_limit_error` | 429 | `ThrottlingException` | 429 | yes |
| `QuotaUsedUp` | 429 `insufficient_quota` and its new names | 402 | — | — | no |
| `Overloaded` | 500, 503 | 500, 503 | 500, 503, 529, `ModelNotReady` | 503 | yes |
| `BadKey` | 401, 403 | 400 `API_KEY_INVALID`, 401, 403 | 403 | 401 | no |
| `NotFound` | 404 | 404 | 404 | 404 | no |
| `BadRequest` | 400 | 400 | 400 | 400 | no |
| `Refused` | `refusal`, `content_filter` | `blockReason`, `SAFETY` and kin | `guardrail_intervened`, `content_filtered` | — | no |
| `TimedOut` | the `timeout` passed | 504 too | 408 too | the `timeout` passed | yes |

`Failed(status, message)` stays for anything an adapter can't place. A `match` that ends with `_ => fail e` keeps working.

### 3. Retries, gates and keys

- **The waits stay** as spec 8.4 has them (doubling, jittered, `retries` times), but a provider's `wait` hint comes first.
- **One gate for each provider and key** in place of the one gate for the program. A 429 from one provider no longer slows the calls to another.
- **What is retried** is decided by the kind, not by the status. `QuotaUsedUp` and `BadKey` are never retried, as waiting can't fix them.

### 4. Tokens, prices and how far to trust them

- An `ask`'s usage is read in code (choice 7), and the audit log gives its cost beside its tokens (spec 8.6).
- **Every number that isn't the provider's own count says why.** A count made offline is an estimate. A price is known as of a date and may have changed. A program's own price table is only as good as what it gave. A budget can be passed by the call that crosses it (8.3). The provider's bill is what counts. The same words go in the docs, in messages and in the audit log:

```
model: claude-opus-5-5, tokens: 412 in, 96 out
cost: about $0.0042 (estimate: prices as of 2026-11-16; the provider's bill is exact)
```

```kavach
pub type Price {
    input: Decimal                 // per `per` input tokens
    output: Decimal                // per `per` output tokens, thinking included
    cached_input?: Decimal         // none: as `input`
    per: Int                       // usually 1,000,000
    as_of: Date
    from: PriceSource
}

pub type PriceSource = Kavach | Program | Package(name: String)

pub type Estimate { tokens: Int, how: String }   // "about 4 bytes a token"
```

- A model with no known price has a cost of `none`, never 0.

## Choices made

A maintainer chose the recommended option on all ten choices (2026-11-19). Each choice below says what was chosen, then the options considered.

### Choice 1. Which providers come first

A maintainer chose (2026-11-19): **the OpenAI-shaped Chat Completions API first**, then each provider's own API in this order: Gemini (`generateContent`, choice 6), Bedrock Converse, Vertex (Gemini, and Claude through `rawPredict`, which is almost Anthropic's format), then OpenAI's Responses API. One adapter reaches OpenAI, Azure (v1), Ollama, llama.cpp and vLLM: Chat Completions is the one API every local server speaks fully, and Azure's v1 takes it with only another key header.

Options considered:

- **Chat Completions first** (chosen): reaches the most with one adapter.
- **Responses first:** OpenAI's newer API, with a counting endpoint; but no stop text, and Ollama supports it only in part.
- **The big clouds first** (Bedrock, Vertex): needs request signing first (choice 5).
- **All at once:** the most work before anyone can use one.

### Choice 2. How a `Model` names its provider

A maintainer chose (2026-11-19): **a `provider` field holding a provider value**, `Model(name: "gpt-5.5", provider: OpenAi())`. The provider is written with plain values, like the rest of `Model`, so the compiler still works it out when it builds the program (spec 8.4), and runs its `check`. Leaving it out means `Anthropic()`, so every program today keeps working. A provider's details are its own fields: `Bedrock(region: "eu-west-1")`, `Vertex(project: "shop", location: "global")`, `Azure(resource: "shop-ai")`. Each provider gives its own default key variable (`OPENAI_API_KEY`, `GEMINI_API_KEY`, `AZURE_OPENAI_API_KEY`, `AWS_BEARER_TOKEN_BEDROCK`) and address.

Options considered:

- **A `provider` field** (chosen): clear, and checked when the program is built.
- **Read from `api_url`:** can't tell a proxy, a company domain, or Ollama's two APIs apart.
- **A prefix in the name** (`"openai/gpt-5.5"`, as LiteLLM does): a string the compiler can't check, with no room for a region or a project.
- **A field, guessed from `api_url` when left out:** wrong guesses are quiet.

### Choice 3. How fallbacks are written

A maintainer chose (2026-11-19): **`model` takes a list, tried in turn**: `model [careful(), backup()]` in an agent, `ask<T>(p, model: [a, b])` in a call. Several keys of one provider are the same model with another `api_key_env` in the list.

- **When it moves on:** on `RateLimited`, `QuotaUsedUp`, `Overloaded`, `BadKey`, `NotFound`, `TimedOut` and `Network`. Never on `Denied`, which is a limit, nor on `Refused`, so a fallback can't be used to get round a safety filter.
- **At once:** a busy model hands the call to the next one at once. The waits of spec 8.4 start only when every model in the list is busy, so a second key never waits while it is free.
- The error is the last model's. The audit log has a line for each model tried.

Options considered:

- **A list** (chosen): one idea covers other models, other providers and other keys.
- **A `fallback` field on `Model`:** a chain of models inside models, harder to read.
- **`catch` in code only:** works today, but is repeated at every call, and agents can't share it.
- **A `Models` value with its own rules:** the most control, and one more type to learn.

### Choice 4. Where prices come from

A maintainer chose (2026-11-19): **both: a dated table in each adapter, with the program's own table winning.** Each table says its date and source. The tables of the providers outside `std.ai` live in their package (choice 8), so a new price needs no Kavach release. A price is in US dollars, the currency of the providers' price pages.

Options considered:

- **Both** (chosen): works at once, and a program can correct it. The tables must be kept up.
- **Built in only:** out of date the day a provider changes a price.
- **Set by the program only:** nothing works until the program gives them.
- **Read from the providers' price services:** none gives prices by model name (Azure's and Google's are by meter or SKU, Bedrock's lists Claude through Marketplace).

### Choice 5. Keys that cloud providers sign or renew

A maintainer chose (2026-11-19): **plain keys first, then signing in the runtime.** First a Bedrock API key (`Bearer`), Azure's `api-key`, and a Vertex token a program puts in a variable. Then SigV4 (`Auth.AwsSignature`) and Google service-account tokens (`Auth.GoogleToken`), signed and renewed by the runtime; `std.crypto` (decision 0093) has the pieces. Until then, a Vertex token lasts an hour and a Bedrock short-term key 12 hours, so long runs renew them outside Kavach.

Options considered:

- **Plain keys, then signing** (chosen): works soon, and as the clouds intend later.
- **Signing from the start:** all the native code before the first cloud call.
- **Signing in provider code, with the key as a `secret` parameter:** provider code would then hold the key, which the frame of section 1 avoids.

### Choice 6. Gemini: `generateContent` or the Interactions API

A maintainer chose (2026-11-19): **`generateContent`**: one adapter for the Gemini API and Vertex, which speaks only `generateContent`. Google calls it "legacy" but says it stays supported, with no end date. Interactions can come as its own adapter when Vertex takes it.

Options considered: **`generateContent`** (chosen); **the Interactions API**, Google's newest, with other field names and another error body, a second adapter for Vertex, and a schema already broken once (May 2026).

### Choice 7. How code reads an `ask`'s tokens and cost

A maintainer chose (2026-11-19): **a second form, `ai.ask_full<T>`, and totals per agent and per run.** `const a = try ai.ask_full<Verdict>(p)` gives `a.value`, `a.usage`, `a.cost` and `a.model`; `ask<T>` stays as short as now. `a.cost` is a `Cost?` that carries its trust note, so printing it prints the note.

Options considered:

- **A second form, plus totals** (chosen): clear and typed.
- **The last `ask`'s details, per task** (`ai.last_usage()`): hidden state, wrong when calls overlap.
- **Totals only:** enough for budgets, not for one call.

### Choice 8. Where the adapters live

A maintainer chose (2026-11-19): **Anthropic's adapter in `std.ai`, rewritten in Kavach against the same interface; the others in a package with its own repository** (decision 0098). So `ask` works with no dependency, Anthropic's adapter is a default a program can wrap like any other, and other providers change without a Kavach release. Translating a request takes microseconds beside a call of seconds.

Options considered:

- **Anthropic's in `std.ai`, the others in a package** (chosen).
- **All in `std.ai`:** every provider change waits for a Kavach release, which the frame rules out.
- **Native adapters in the runtime:** nobody can change them without a release.

### Choice 9. Money budgets in `limit Ai`

A maintainer chose (2026-11-19): **`cost <= 5.00 per day`, in US dollars**, worked out from each model's price. A call to a model with no known price under a `cost` limit is refused with `Denied` ("no price for gpt-5.5: give one, decision 0140"), never counted as free. Like `tokens`, the call that crosses the budget still runs (8.3), and the message says so.

Options considered: **a budget in US dollars** (chosen); **tokens only**, with cost only reported; **a budget with a currency named**.

### Choice 10. Counting a prompt's tokens before sending

A maintainer chose (2026-11-19): **the provider's counting call where there is one** (OpenAI Responses, Gemini, Vertex, Bedrock, llama.cpp, vLLM), **else its offline estimate, marked as one**: `ai.count(prompt, model)` gives a `Count` that says whether it is exact. A counting call needs `Ai`, is in the audit log, and doesn't count toward `limit Ai`'s `tokens`.

Options considered: **the counting call, else an estimate** (chosen); **offline estimates only**; **the counting call only**, `none` without one.

## Built first, and points for a maintainer's review

The first part was built on 2026-11-20: the `Provider` interface, `Anthropic`'s adapter rewritten in Kavach in `std.ai`, and the richer `AiError` (spec 8.4). Where this record didn't settle a point, the simplest option that fits it was taken. Each is **for a maintainer's review**:

1. **The interface has only the methods built so far:** `check`, `key_variable`, `auth`, `request` and `read`. `count_request`, `read_count`, `estimate` and `price` join it with counting and prices; there are no users yet, so adding methods later breaks no one.
2. **`key_variable(self) -> String`** is a method this record didn't have: each provider gives its own default key variable (choice 2), and the interface needed a place for it.
3. **How the runtime calls a provider:** through `std.ai`'s private `wire(provider, settings)`, which gives a `fn(String) -> String` that takes and gives JSON text. So the runtime needs no Kavach types, and native code calls one closure, as it calls a `display` method.
4. **`KAVACH_AI_URL`** now keeps the path the provider wrote and changes only the scheme, host and port (plus a path in front), so it works for every provider.
5. **A gate's key is the provider's type name and the key's variable name.** Two variables holding the same key get two gates.
6. **A gate counts a call as turned away by its status (429),** not by the kind `read` gives, as a try's gate is left on the helper thread that sent it, which keeps a cancelled task from holding a place at the gate.
7. **The wait after a hint** is the longer of the hint and the usual step, capped at the `timeout`.
8. **Anthropic's mapping:** 402 and `billing_error` are `QuotaUsedUp`; 504 is `TimedOut`; every other 5xx is `Overloaded`; 400 and 413 are `BadRequest`; a refusal has no reason, as Anthropic gives none. Cached input tokens (`cache_read_input_tokens`, `cache_creation_input_tokens`) are added into `input`.
9. **A count a reply lacks** counts as 0 for `limit Ai` and the audit log, until usage and costs are built (choice 7).
10. **`check` runs on each `Model` the program writes,** at the place it is given, not on the default model. It runs in the interpreter while the program is compiled, as a tagged literal's checker does, with the same step limit.
11. **A provider's value is a constructor written with plain values** (or a `const` or a function with no parameters that gives one), lowered or run where the `ask` is.
12. **`std.ai`'s variant names don't bring the module in.** `NotFound`, `Answer` and `Other` are common words; every program that needs `std.ai` writes `ask`, `Model` or one of its type names anyway.
13. **The key goes to the address in the provider's request,** so a package's provider decides where a program's key is sent. The spec says to use providers from packages you trust.
14. **`Auth.AwsSignature` and `Auth.GoogleToken`** fail an `ask` with `Failed(status: 0, …)` until signing is built (choice 5).

## Consequences

- Programs can use any provider with an HTTP API, and a package can add one without a Kavach release.
- `AiError` gains variants. Programs that match `Failed(status: 429, …)` change to `RateLimited`; there are no users yet, so no shim.
- Spec 8.3, 8.4 and 8.6 change, with conformance tests, when this is built.
- Each adapter needs a fake server in its tests, as Anthropic's has today, with the provider's real error bodies.
- Price tables must be kept up. Their dates make a stale one visible.
- The runtime's one gate becomes one per provider and key.
- Interface values (decision 0029) must work in native code for a program's own provider.

## Alternatives considered

- **Providers that send their own requests,** through the `http` package and `uses Net`. More freedom (streaming, odd protocols). But retries, gates, limits, dry runs and the audit log would have to be written again in each provider, and a package's provider could send a prompt anywhere.
- **One "OpenAI-compatible" setting and nothing more.** Covers the most servers with no interface. But Bedrock, Vertex's own API and Gemini's own errors stay out, and a provider's change still needs a release.
- **A `Provider` union of the providers Kavach knows** (rejected in decision 0059 too). Closed: a program can't add one.
- **Prices fetched at run time.** No provider offers them by model name (above).
