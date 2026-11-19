# 0140. AI providers other than Anthropic

- **Status:** Proposed (the frame is a maintainer's, from to-do item 17, 2026-11-16; the choices below are for a maintainer)
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

Only the frame below is proposed for acceptance now. The choices after it are for a maintainer; each gives a recommendation.

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

## Choices for a maintainer

### Choice 1. Which providers come first

| Option | Reaches | Trade-off |
|---|---|---|
| **A. The OpenAI-shaped Chat Completions API first** | OpenAI, Azure (v1), Ollama, llama.cpp, vLLM, and Gemini and Vertex through their OpenAI-shaped endpoints | One adapter reaches the most. But Gemini's and Vertex's OpenAI endpoints are beta and hide their own errors and thinking. |
| B. OpenAI's Responses API first | OpenAI, Azure, partly the local servers | OpenAI's newer API, with a counting endpoint; but no stop text, and Ollama's support is partial. |
| C. The big clouds first (Bedrock, Vertex) | Companies that must stay in their cloud | Needs request signing first (choice 5). |
| D. All at once | Everything | The most work before anyone can use one. |

**Recommendation: A, then each provider's own API in this order:** Gemini (`generateContent`, choice 6), Bedrock Converse, Vertex (Gemini, and Claude through `rawPredict`, which is almost Anthropic's format), then Responses. Chat Completions is the one API every local server speaks fully, and Azure's v1 now takes it with only another key header.

### Choice 2. How a `Model` names its provider

| Option | Example | Trade-off |
|---|---|---|
| **A. A `provider` field holding a provider value** | `Model(name: "gpt-5.5", provider: OpenAi())` | Clear, and checked when the program is built. A program's own provider fits the same way. One more field to write. |
| B. Read from `api_url` | `api_url: "https://api.openai.com"` | Nothing new to learn; but it can't tell a proxy, a company domain, or Ollama's two APIs apart. |
| C. A prefix in the name | `name: "openai/gpt-5.5"` | Short, as LiteLLM does; but it is a string the compiler can't check, and has no room for a region or a project. |
| D. A field, guessed from `api_url` when left out | — | Short when the guess is right; wrong guesses are quiet. |

**Recommendation: A.** The provider is a value written with plain values, like the rest of `Model`, so the compiler still works it out when it builds the program (spec 8.4), and runs its `check`. Leaving it out means `Anthropic()`, so every program today keeps working. A provider's details are its own fields: `Bedrock(region: "eu-west-1")`, `Vertex(project: "shop", location: "global")`, `Azure(resource: "shop-ai")`. Each provider also gives its own default key variable (`OPENAI_API_KEY`, `GEMINI_API_KEY`, `AZURE_OPENAI_API_KEY`, `AWS_BEARER_TOKEN_BEDROCK`) and address.

### Choice 3. How fallbacks are written

| Option | Example | Trade-off |
|---|---|---|
| **A. `model` takes a list, tried in turn** | `model [careful(), backup()]`; `ask<T>(p, model: [a, b])` | One idea covers other models, other providers and other keys. |
| B. A `fallback` field on `Model` | `Model(…, fallback: backup())` | A chain of models inside models; harder to read. |
| C. No new form: `catch` in code | `ask<T>(p) catch _ => try ask<T>(p, model: backup())` | Works today; but it is repeated at every call, and agents can't share it. |
| D. A `Models` value with its own rules | `Models(each: [a, b], when: …)` | Most control; one more type to learn. |

**Recommendation: A.** Several keys of one provider are the same model with another `api_key_env` in the list. Two parts of this need a choice too:

- **When to move on.** Recommended: on `RateLimited`, `QuotaUsedUp`, `Overloaded`, `BadKey`, `NotFound`, `TimedOut` and `Network`. Never on `Denied`, which is a limit, nor `Refused`, so a fallback can't be used to get round a safety filter.
- **Move on at once, or wait first.** Recommended: a busy model hands the call to the next one at once; the waits of spec 8.4 start only when every model in the list is busy. Otherwise a second key waits 15 seconds while it is free.

The error is the last model's. The audit log has a line for each model tried.

### Choice 4. Where prices come from

| Option | Trade-off |
|---|---|
| A. Built in: a table for the models Kavach knows | Works with no setup; out of date the day a provider changes a price. |
| B. Set by the program only | Always the program's own numbers; nothing works until it gives them. |
| **C. Both: a dated table in each adapter, and the program's own over it** | Works at once, and a program can correct it. The table must be kept up. |
| D. Read from the providers' price services | None gives prices by model name (Azure's and Google's are by meter or SKU, Bedrock's lists Claude through Marketplace). Not possible today. |

**Recommendation: C.** Each adapter's table says its date and source, and the program's table wins. With choice 8's recommendation, the tables live in packages, so a new price needs no Kavach release. A price is in US dollars, the currency of the providers' price pages.

### Choice 5. Keys that cloud providers sign or renew

| Option | Trade-off |
|---|---|
| **A. Plain keys first, signing later** | Bedrock API keys (`Bearer`), Azure `api-key`, a Vertex token a program puts in a variable. Works soon; but a Vertex token lasts an hour and a Bedrock short-term key 12 hours, so long runs need renewing outside Kavach. |
| B. SigV4 and Google service accounts in the runtime from the start | Works as the clouds intend; but more native code (SigV4 is HMAC-SHA-256, a Google token needs an RS256 signature and one more call), all of it written before the first cloud call. |
| C. Signing in provider code, with the key as a `secret` parameter | No runtime work; but provider code then holds the key, which the frame of section 1 avoids. |

**Recommendation: A, then B** (`Auth.AwsSignature` and `Auth.GoogleToken`). `std.crypto` (decision 0093) already has the pieces.

### Choice 6. Gemini: `generateContent` or the Interactions API

Google now names `generateContent` "legacy" and puts a new Interactions API first, with other field names and another error body. Vertex speaks only `generateContent`.

| Option | Trade-off |
|---|---|
| **A. `generateContent`** | One adapter for the Gemini API and Vertex. Google says it stays supported, with no end date. |
| B. Interactions | Google's newest; a second adapter for Vertex. It already broke its schema once (May 2026). |

**Recommendation: A**, with Interactions as its own adapter when Vertex takes it.

### Choice 7. How code reads an `ask`'s tokens and cost

| Option | Example | Trade-off |
|---|---|---|
| **A. A second form that gives the details** | `const a = try ai.ask_full<Verdict>(p)`; `a.value`, `a.usage`, `a.cost`, `a.model` | Clear and typed; `ask<T>` stays as short as now. |
| B. The last `ask`'s details, per task | `ai.last_usage()` | No new form; but hidden state, and wrong when calls overlap. |
| C. Totals only | `agents.spent(RefundBot)` | Enough for budgets, not for one call. |

**Recommendation: A, and C for an agent's and a run's totals.** `a.cost` is a `Cost?` that carries its trust note, so printing it prints the note.

### Choice 8. Where the adapters live

| Option | Trade-off |
|---|---|
| A. All in `std.ai` | One place; every provider change waits for a Kavach release, which the frame rules out. |
| **B. Anthropic's in `std.ai`; the others in a package of their own repository** (decision 0098) | `ask` works with no dependency; other providers change without a Kavach release. One more dependency for a program that uses them. |
| C. Native adapters in the runtime | Fastest; but nobody can change them without a release. |

**Recommendation: B.** Anthropic's adapter is rewritten in Kavach against the same interface, so it is a default a program can wrap like any other. Translating a request takes microseconds beside a call of seconds.

### Choice 9. Money budgets in `limit Ai`

The chapter's open questions ask about this. Option A: `cost <= 5.00 per day`, in US dollars, worked out from each model's price. Option B: tokens only, as now, with cost only reported. Option C: a money budget with a currency named.

**Recommendation: A.** A call to a model with no known price under a `cost` limit is refused with `Denied` ("no price for gpt-5.5: give one, decision 0140"), never counted as free. Like `tokens`, the call that crosses the budget still runs (8.3), and the message says so.

### Choice 10. Counting a prompt's tokens before sending

Option A: the provider's counting call where there is one (OpenAI Responses, Gemini, Vertex, Bedrock, llama.cpp, vLLM), else its offline estimate, marked as one. Option B: offline estimates only, with no network. Option C: the counting call only, `none` without one.

**Recommendation: A**, as `ai.count(prompt, model)` giving a `Count` that says whether it is exact. A counting call needs `Ai`, is in the audit log, and doesn't count toward `limit Ai`'s `tokens`.

## Consequences

- Programs can use any provider with an HTTP API, and a package can add one without a Kavach release.
- `AiError` gains variants. Programs that match `Failed(status: 429, …)` change to `RateLimited`; there are no users yet, so no shim.
- Spec 8.3, 8.4 and 8.6 change, with conformance tests, when the choices are made.
- Each adapter needs a fake server in its tests, as Anthropic's has today, with the provider's real error bodies.
- Price tables must be kept up. Their dates make a stale one visible.
- The runtime's one gate becomes one per provider and key.
- Interface values (decision 0029) must work in native code for a program's own provider.

## Alternatives considered

- **Providers that send their own requests,** through the `http` package and `uses Net`. More freedom (streaming, odd protocols). But retries, gates, limits, dry runs and the audit log would have to be written again in each provider, and a package's provider could send a prompt anywhere.
- **One "OpenAI-compatible" setting and nothing more.** Covers the most servers with no interface. But Bedrock, Vertex's own API and Gemini's own errors stay out, and a provider's change still needs a release.
- **A `Provider` union of the providers Kavach knows** (rejected in decision 0059 too). Closed: a program can't add one.
- **Prices fetched at run time.** No provider offers them by model name (above).
