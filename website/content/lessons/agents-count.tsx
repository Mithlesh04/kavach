import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C, withCode } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { rustConst, rustRaw, rustStringAfter, rustStrings } from "@/lib/rust-tests";
import { AiFields } from "./ai-fields";
import { Ex, H2, Note, P } from "./parts";

// ---------- ask<T> › Counting tokens ----------
//
// Spec 8.4 *Counting tokens before sending*, 8.3, 8.6, decision 0140
// choice 10: ai.count, Count and Estimate. The programs and what they
// print are read from the end-to-end tests, which run them against a fake
// server; the types' fields are read from std.ai.

const TESTS = "tools/cli/tests/ai_count.rs";
const EXACT = "a_count_is_the_providers_own_from_its_counting_call";
const OFFLINE = "a_provider_without_a_counting_call_gives_its_estimate";
const FAILS = "a_counting_call_that_fails_fails_the_count";
const AGENT = "a_count_is_in_the_audit_log_and_no_limit_counts_it";

/** The test's `COUNT` program with its `MODEL` and `ESTIMATE` filled in, and `Offline` cut when unused, as the test does. */
function program(model: string, estimate: string): string {
  const p = rustConst(TESTS, "COUNT").replace("MODEL", model).replace("ESTIMATE", estimate);
  if (model.includes("Offline")) return p;
  const start = p.indexOf("/// Anthropic, without");
  if (start < 0) throw new Error(`${TESTS}: COUNT changed shape`);
  const end = start + p.slice(start).indexOf("\n}\n") + 3;
  return p.slice(0, start) + p.slice(end);
}

/** The model a test function sets with `let model = "…"`. */
const modelOf = (fn: string) => rustStringAfter(TESTS, fn, "let model =");

/** Each case of the offline test: the estimate `Offline` gives, and what the program prints. */
function offlineCases(): { estimate: string; out: string }[] {
  const s = rustStrings(TESTS, OFFLINE, "let cases");
  if (s.length !== 4) throw new Error(`${TESTS}: the offline test's cases changed shape`);
  return [{ estimate: s[0], out: s[1] }, { estimate: s[2], out: s[3] }];
}

/** What the program prints for each reply of the failing-call test, in the test's order. */
function failCases(): string[] {
  const s = rustStrings(TESTS, FAILS, "let cases").filter((x) => x.startsWith("failed: ") || x.includes("(counted by the provider)"));
  if (s.length !== 4) throw new Error(`${TESTS}: the failing-call test's cases changed shape`);
  return [...s, rustStringAfter(TESTS, FAILS, 'out, "failed: NoKey')];
}

const exactModel = modelOf(EXACT);
const offlineModel = modelOf(OFFLINE);
const offline = offlineCases();
const fails = failCases();
const failRows: [string, string][] = [
  ["Busy (429), with `retries: 0`", fails[0]],
  ["A bad key (401)", fails[1]],
  ["A reply with no count", fails[2]],
  ["Busy, then counted, with `retries: 1`", fails[3]],
  ["No `ANTHROPIC_API_KEY` set: nothing is sent", fails[4]],
];

export const countDoc: LessonDoc = {
  lead: "ai.count counts a prompt's input tokens before it is sent, so a program can see whether it fits a model or a budget before it asks.",
  simple: <>Before you post a heavy parcel, you weigh it to know the price. <C>ai.count</C> weighs a prompt: it says how many tokens it would take, and sending it to count costs nothing.</>,
  minutes: 6,
  before: ["agents/ask/usage-and-cost"],
  body: (
    <>
      <P>
        <C>ai.count</C> comes from <C>std.ai</C>. It is written as <C>ask</C> is, without a type:{" "}
        <C>ai.count(prompt)</C>, or <C>ai.count(prompt, model: m)</C>. It gives a <C>Count</C>:
      </P>
      <CodeBlock code={program(exactModel, "")} file="main.kav, run against a test model" output={rustStringAfter(TESTS, EXACT, 'out, "')} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>The provider counted the tokens, so the count is exact: <C>exact</C> is <C>true</C>.</li>
        <li>It counts what an <C>ask</C> with that model sends: the prompt and the model&apos;s <C>instructions</C>. Not the few dozen tokens <C>ask&lt;T&gt;</C> adds to describe its type, nor the answer.</li>
        <li>The model is chosen as for <C>ask</C>: the call&apos;s <C>model:</C>, else the agent&apos;s, else the default. A list of models counts with its first model.</li>
      </ul>
      <P>
        <C>Anthropic()</C> counts with Anthropic&apos;s free token counting call, <C>POST /v1/messages/count_tokens</C>.
        Its body is the <C>ask</C>&apos;s without <C>max_tokens</C>, <C>stop_sequences</C> and <C>extra</C>. For the
        model above, the test checks it is exactly:
      </P>
      <CodeBlock code={rustRaw(TESTS, EXACT, "body,")} file="the counting call's body" lang="text" />
      <P>Anthropic says its count can differ by a few tokens from what the message takes.</P>
      <AiFields type="Count: Display" extra={{ tokens: "the prompt's input tokens" }} />
      <H2>An estimate, without the network</H2>
      <P>
        A provider with no counting call can give an estimate instead, with its <C>estimate</C> method. Here{" "}
        <C>Offline</C> wraps <C>Anthropic()</C> without its counting call, so nothing is sent:
      </P>
      <CodeBlock code={program(offlineModel, offline[0].estimate)} file="main.kav" output={offline[0].out} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>Anthropic()</C>&apos;s estimate is one token for every 4 characters of the prompt and <C>instructions</C>, rounded up. It is rough: Anthropic&apos;s newer models take about 30% more tokens for the same text than older ones.</li>
        <li>An estimate prints with <C>about</C>, and <C>exact</C> is <C>false</C>.</li>
        <li>With <C>estimate</C> giving <C>none</C> as well, the provider has no way to count: <C>{offline[1].out.trim()}</C></li>
      </ul>
      <AiFields type="Estimate" extra={{ tokens: "the tokens it estimates" }} />
      <Ex name="agents/count-estimate" />
      <H2>When the counting call fails</H2>
      <P>
        The counting call is sent as an <C>ask</C>&apos;s request is: the runtime puts the key where <C>auth</C> says,
        and sends it again after a busy reply, as the <C>Model</C>&apos;s <C>retries</C> say. A call that fails fails{" "}
        <C>ai.count</C> with its <C>AiError</C>: it doesn&apos;t fall back to the estimate, so a bad key or a used-up
        quota shows before the <C>ask</C>. With the first program above:
      </P>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <thead>
            <tr className="border-b border-line text-left text-soft">
              <th className="py-2 pr-4 font-bold">The counting call gets</th>
              <th className="py-2 font-bold">The program prints</th>
            </tr>
          </thead>
          <tbody>
            {failRows.map(([reply, out]) => (
              <tr key={reply} className="border-b border-line align-top">
                <td className="py-2 pr-4 text-muted">{withCode(reply)}</td>
                <td className="py-2">
                  {out.trim().split("\n").map((line) => (
                    <div key={line}><C>{line}</C></div>
                  ))}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <H2>Free, so no limit counts it</H2>
      <P>
        <C>ai.count</C> needs <C>Ai</C>, but it is not an <C>ask</C>. Counting is free, so it adds nothing to{" "}
        <C>ai.spent</C>, and no rule of <C>limit Ai</C> (<C>count</C>, <C>tokens</C> or <C>cost</C>) and no{" "}
        <C>approve</C> applies to it. This agent counts twice, 5,000 tokens each time, under limits of one call and 100
        tokens a day, and its <C>ask</C> still runs:
      </P>
      <CodeBlock code={rustConst(TESTS, "AGENT")} file="main.kav, run against a test model" output={rustStringAfter(TESTS, AGENT, 'out, "')} />
      <P>
        In an agent, each count has its own entry in the{" "}
        <Link href="/docs/agents/audit-log/">audit log</Link>, <C>Ai.count</C>. A dry run counts nothing: the count is{" "}
        <C>about 0 tokens (dry run: nothing was counted)</C>.
      </P>
      <Note>A count isn&apos;t kept in a durable agent&apos;s journal: an event run again after a restart counts again.</Note>
      <P><C>ai.count</C> takes a prompt and <C>model:</C>, and nothing else:</P>
      <Ex name="agents/count-option.fails" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/08-capabilities-and-agents/04-typed-ai-calls.md" label="spec 8.4 · Counting tokens before sending" />
        <SrcRef file="docs/decisions/0140-ai-providers.md" label="decision 0140" />
        <SrcRef file="stdlib/ai/ai.kav" line={243} symbol="pub type Count: Display" label="std.ai: Count" />
        <SrcRef file="stdlib/ai/ai.kav" line={234} symbol="pub type Estimate" label="std.ai: Estimate" />
        <SrcRef file={TESTS} label="tested end to end" />
        <SrcRef file="tests/conformance/08-capabilities/anthropic-estimates-a-count.kav" label="test: the estimate" />
      </div>
    </>
  ),
};
