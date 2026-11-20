import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { specBlockWith } from "@/lib/spec-tables";
import { Ex, H2, Note, P } from "./parts";

// ---------- ask<T> › Providers ----------

const SPEC_84 = "08-capabilities-and-agents/04-typed-ai-calls.md";

export const providersDoc: LessonDoc = {
  lead: "A provider is how ask speaks to one AI service's API. It is a Kavach interface that only translates; the runtime keeps the key, sends, retries and keeps the limits.",
  simple: <>A provider is like an interpreter at a meeting. It turns your question into the other side&apos;s language, and their answer back into yours. It never holds your wallet and never makes the call itself: Kavach does that, the same way for every provider.</>,
  minutes: 7,
  before: ["agents/ask/when-the-ai-fails", "types/interfaces"],
  body: (
    <>
      <P>
        Every <C>Model</C> has a provider: <C>Anthropic()</C> from <C>std.ai</C>, unless it names another with{" "}
        <C>provider:</C>. A provider is a Kavach interface, so a program or a package can write one. A new provider, or
        a change in a provider&apos;s API, needs new Kavach code, not a new Kavach release.
      </P>
      <H2>The interface and its types</H2>
      <CodeBlock code={specBlockWith(SPEC_84, "8.4", "pub interface Provider")} file="std.ai, as spec 8.4 gives it" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>check</C> says which settings of a <C>Model</C> the provider can&apos;t take. It runs when the program is built (below).</li>
        <li><C>key_variable</C> names the variable holding the key, when the <C>Model</C> names none in <C>api_key_env</C>.</li>
        <li><C>auth</C> says where the key goes in a request. The runtime puts it there.</li>
        <li><C>request</C> writes the request for one <C>ask</C>: once for the first try, with <C>strict: false</C>, and once more for each try after an answer that isn&apos;t a valid <C>T</C>, with <C>strict: true</C>.</li>
        <li><C>read</C> is called for every reply, whatever its status. It says what the reply means: an <C>Answer</C>, a refusal, an answer cut off, or a <C>Problem</C> of some kind.</li>
      </ul>
      <H2>A provider only translates</H2>
      <P>
        A provider&apos;s methods are plain functions. They use no capabilities, so provider code can&apos;t send anything
        itself. The runtime does everything else, for every provider alike:
      </P>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>It reads the key and puts it where <C>auth</C> says, so provider code never holds it.</li>
        <li>It sends the request, waits, retries, keeps the gates, applies <C>limit Ai</C>, and writes the journal and the audit log.</li>
        <li>A dry run or a generated test calls no provider method.</li>
      </ul>
      <P>
        So a request is only a value. Here is the one <C>Anthropic()</C> writes for a <C>Model</C>. The key is not in it:{" "}
        <C>auth</C> only says it goes in the <C>x-api-key</C> header.
      </P>
      <Ex name="agents/provider-request" />
      <P>
        <C>read</C> turns a reply into what it means, not into what its status says. Here <C>Anthropic()</C> reads
        replies written by hand, so nothing is sent. The cached input tokens are added into <C>input</C>, and{" "}
        <C>retry-after</C> becomes the wait:
      </P>
      <Ex name="agents/provider-reads" />
      <P>
        Which kinds are sent again, and the <C>AiError</C> each one becomes:{" "}
        <Link href="/docs/agents/ask/when-the-ai-fails/">When the AI fails</Link>. A count a reply doesn&apos;t have is{" "}
        <C>none</C>, never a made-up number.
      </P>
      <H2>Changing one part</H2>
      <P>
        An interface&apos;s methods have no default bodies. So a program changes one part of a provider by wrapping it
        and passing the other methods on. This gateway adds a header and reads its key from its own variable:
      </P>
      <Ex name="agents/provider-gateway" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>The request goes to the <C>Model</C>&apos;s <C>api_url</C>, with the path Anthropic&apos;s API uses.</li>
        <li>The key will come from <C>SHOP_GATEWAY_KEY</C>, and still goes in <C>x-api-key</C>.</li>
        <li><C>main</C> only calls the methods to show what they give. A program just uses <C>gateway()</C> as an agent&apos;s <C>model</C>, or in <C>ask&lt;T&gt;(prompt, model: gateway())</C>.</li>
      </ul>
      <H2>check runs when the program is built</H2>
      <P>
        The compiler runs <C>check</C> on each <C>Model</C> the program writes, with its provider. Each text it gives is
        a compile error at that <C>Model</C>. A server on this machine that takes no stop text says so:
      </P>
      <Ex name="agents/provider-check.fails" />
      <P>
        <C>Anthropic()</C>&apos;s own <C>check</C> refuses <C>thinking: false</C> on <C>claude-opus-5-5</C>, and with{" "}
        <C>effort</C> above <C>High</C>. A <C>check</C> that panics is a compile error that says so.
      </P>
      <P>
        A <C>Model</C>&apos;s provider is worked out when the program is built, like the rest of the <C>Model</C>: a
        type&apos;s constructor written with plain values (<C>Anthropic()</C>,{" "}
        <C>Gateway(base: Anthropic())</C>), a <C>const</C> set to one, or a call of a function with no parameters that
        gives one. Anything else is a compile error.
      </P>
      <H2>Trust the providers you use</H2>
      <Note>
        The key goes to the address in the provider&apos;s request. So a package&apos;s provider decides where your key is
        sent, as a package&apos;s code does with any secret it is given. Use providers from packages you trust.
      </Note>
      <H2>Not built yet</H2>
      <P>Decision 0140 sets how each of these will work, but none works today:</P>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Other providers. They will come in a package of their own repository: first OpenAI-shaped Chat Completions (OpenAI, Azure, Ollama, llama.cpp, vLLM), then Gemini, Bedrock, Vertex and OpenAI&apos;s Responses API. Today only <C>Anthropic()</C> ships; a service that speaks Anthropic&apos;s API is reached through <C>api_url</C>.</li>
        <li>Signed keys: <C>Auth.AwsSignature</C> and <C>Auth.GoogleToken</C>. For now, an <C>ask</C> whose provider gives one fails with <C>Failed(status: 0, …)</C>.</li>
        <li><C>model</C> taking a list of models, tried in turn.</li>
        <li><C>ai.ask_full&lt;T&gt;</C>, which would give an <C>ask</C>&apos;s usage and cost.</li>
        <li>Prices, costs and money budgets in <C>limit Ai</C>.</li>
        <li>Counting a prompt&apos;s tokens before it is sent (<C>ai.count</C>).</li>
      </ul>
      <P>
        The interface gains the methods those need (<C>count_request</C>, <C>read_count</C>, <C>estimate</C> and{" "}
        <C>price</C>) when they are built.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/08-capabilities-and-agents/04-typed-ai-calls.md" label="spec 8.4 · Providers" />
        <SrcRef file="docs/decisions/0140-ai-providers.md" label="decision 0140" />
        <SrcRef file="stdlib/ai/ai.kav" line={115} symbol="pub interface Provider" label="std.ai: Provider, as declared" />
        <SrcRef file="stdlib/ai/ai.kav" line={183} symbol="pub type Anthropic" label="std.ai: Anthropic" />
        <SrcRef file="tests/conformance/08-capabilities/a-provider-wraps-another.kav" label="test: a provider wraps another" />
        <SrcRef file="tests/conformance/08-capabilities/anthropic-reads-each-kind-of-trouble.kav" label="test: each kind of trouble read" />
        <SrcRef file="tests/conformance/08-capabilities/a-providers-check-refuses-a-model.kav" label="test: check refuses a model" />
        <SrcRef file="tools/cli/tests/ai_providers.rs" line={261} symbol="a_programs_own_provider_translates_and_the_runtime_sends" label="test: a program's own provider, end to end" />
      </div>
    </>
  ),
  quiz: [
    {
      type: "choice",
      prompt: <>A provider from a package writes the request for each <C>ask</C>. Who reads the API key and puts it in the request?</>,
      options: [
        "The runtime, where the provider's auth says",
        "The provider's request method",
        "The program, before each ask",
      ],
      answer: 0,
      explain: "A provider only translates. The runtime reads the key and puts it where auth says, so provider code never holds it.",
    },
    {
      type: "choice",
      prompt: <>Your company&apos;s gateway needs one more header on every request. What do you write?</>,
      options: [
        "A type that implements Provider, wraps Anthropic, changes request and passes the other methods on",
        "A new Kavach release",
        "A setting in kavach.toml",
      ],
      answer: 0,
      explain: "An interface's methods have no default bodies, so you wrap the provider and pass the methods you don't change to it.",
    },
  ],
};
