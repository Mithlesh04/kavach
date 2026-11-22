import fs from "node:fs";
import path from "node:path";
import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { rustConst, rustLet, rustStrings } from "@/lib/rust-tests";
import { AiFields } from "./ai-fields";
import { Ex, H2, Note, P } from "./parts";

// ---------- ask<T> › Usage and cost ----------
//
// Spec 8.4 *Prices* and *Usage and cost*, decision 0140 (choices 4, 7, 9):
// ai.ask_full<T>, Price and Anthropic's dated table, a program's own
// prices, and ai.spent. The programs and their output are read from the
// end-to-end tests, which run them against a fake server; the price table
// and the types' fields are read from std.ai.

const TESTS = "tools/cli/tests/ai_costs.rs";
const ANTHROPIC = "stdlib/ai/anthropic.kav";
const SPEC_84 = "docs/spec/08-capabilities-and-agents/04-typed-ai-calls.md";

/** The test's `FULL` program with its `MODEL` filled in. */
const full = (model: string) => rustConst(TESTS, "FULL").replace("MODEL", model);

/** Each case of the ask_full test: the model the program asks, and the cost it prints. */
function fullCases(): { model: string; cost: string }[] {
  const s = rustStrings(TESTS, "ask_full_gives_the_usage_and_the_cost_with_its_note", "let cases");
  if (s.length !== 6) throw new Error(`${TESTS}: the ask_full test's cases changed shape`);
  return [0, 2, 4].map((i) => ({ model: s[i], cost: s[i + 1] }));
}

/** Anthropic's price table in std.ai: each model's dollars per million tokens, and the day it was read. */
function anthropicPrices() {
  const text = fs.readFileSync(path.join(process.cwd(), "..", ANTHROPIC), "utf8");
  const at = text.indexOf("fn anthropic_prices()");
  const body = text.slice(at, text.indexOf("\n}\n", at));
  const day = body.match(/Date\(year: (\d+), month: (\d+), day: (\d+)\)/);
  const per = body.match(/per: ([\d_]+)/)?.[1];
  const rows = [...body.matchAll(/"([\w.-]+)": at\(([\d.]+), ([\d.]+), ([\d.]+)\)/g)];
  if (at < 0 || !day || per !== "1_000_000" || rows.length === 0) throw new Error(`${ANTHROPIC}: the price table changed shape`);
  const date = `${day[1]}-${day[2].padStart(2, "0")}-${day[3].padStart(2, "0")}`;
  return { date, line: text.slice(0, at).split("\n").length, rows: rows.map((m) => ({ model: m[1], input: m[2], output: m[3], cached: m[4] })) };
}

const cases = fullCases();
const prices = anthropicPrices();
const shop = cases.find((c) => c.model.includes("ShopAi"));
if (!shop) throw new Error(`${TESTS}: no case with the shop's own prices`);
const fullWant = (c: { model: string; cost: string }) => `true\n412 in, 96 out\n${c.model.split('"')[1]}\n${c.cost}\n`;
const listed = rustConst(TESTS, "LISTED");
const listedWant = rustLet(TESTS, "every_try_counts_at_its_own_models_price", "want");

export const costsDoc: LessonDoc = {
  lead: "ai.ask_full<T> gives an answer with the tokens it took, the model that gave it and what it cost. ai.spent() adds up every ask of the run.",
  simple: <>Each question to an AI model costs a little money, counted in tokens. <C>ai.ask_full</C> is like a shop receipt: the answer, how much it took, and the price. <C>ai.spent</C> is the total of all the receipts so far.</>,
  minutes: 8,
  before: ["agents/ask/choosing-the-model", "agents/ask/providers"],
  body: (
    <>
      <P>
        <C>ai.ask_full&lt;T&gt;</C> asks exactly as <C>ask&lt;T&gt;</C> does: the same <C>model:</C>, retries,
        fallbacks, limits and errors. It gives back an <C>Asked&lt;T&gt;</C> instead of only the answer. Both come
        from <C>std.ai</C>.
      </P>
      <CodeBlock code={full(shop.model)} file="main.kav, run against a test model" output={fullWant(shop)} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>The test model answers <C>{"{\"ok\": true}"}</C> after taking 412 tokens in and 96 out.</li>
        <li><C>a.value</C> is the answer, the <C>Yes</C> that <C>ask&lt;Yes&gt;</C> would give.</li>
        <li><C>a.model</C> is the name of the model that answered: with a list of models, the one that got there.</li>
        <li><C>a.cost</C> prints as <C>about $…</C> with a note that says how far to trust it.</li>
        <li><C>ShopAi</C> gives the program&apos;s own price for one model (<Link href="#your-own-prices">below</Link>).</li>
      </ul>
      <P>The same program, asking other models, prints a different last line:</P>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <thead>
            <tr className="border-b border-line text-left text-soft">
              <th className="py-2 pr-4 font-bold"><C>model:</C></th>
              <th className="py-2 font-bold"><C>print(a.cost)</C> prints</th>
            </tr>
          </thead>
          <tbody>
            {cases.map((c) => (
              <tr key={c.model} className="border-b border-line align-top">
                <td className="py-2 pr-4"><C>{c.model}</C></td>
                <td className="py-2 text-muted"><C>{c.cost}</C></td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <P>A model with no known price costs <C>none</C>, never 0: a made-up 0 would hide real spending.</P>
      <H2>What it gives</H2>
      <AiFields type="Asked<T>" extra={{ value: "the answer: the `T` that `ask<T>` would give" }} />
      <P><C>Cost</C> is a <C>Display</C> type, so printing it prints the dollars and the note:</P>
      <AiFields type="Cost: Display" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><strong className="text-ink">The usage adds up every try</strong> that got a reply: an answer asked for again because it wasn&apos;t a valid <C>T</C>, and each model of a list. A count no reply gave is <C>none</C>.</li>
        <li><strong className="text-ink">The cost</strong> is each try&apos;s tokens at its own model&apos;s price: input tokens at <C>input</C>, those read from the provider&apos;s cache at <C>cached_input</C>, and output tokens at <C>output</C>, each divided by <C>per</C>.</li>
        <li><strong className="text-ink">The note</strong> names the oldest date of the prices used: <C>prices as of …</C> for Kavach&apos;s, <C>the program&apos;s prices as of …</C>, or <C>prices from &lt;package&gt; as of …</C>.</li>
      </ul>
      <H2>Every try counts</H2>
      <P>
        Here the first model&apos;s answer isn&apos;t a valid <C>Yes</C>, so it is asked again; then it is busy, so the
        second model answers. The cost counts the first try at Opus&apos;s price and the last at Haiku&apos;s:
      </P>
      <CodeBlock code={listed} file="main.kav, run against a test model" output={listedWant} />
      <H2>Prices</H2>
      <P>
        A cost comes from a price, and each provider gives its models&apos; prices with its <C>price</C> method (
        <Link href="/docs/agents/ask/providers/">providers</Link>). A price is in US dollars, the currency of the
        providers&apos; price pages, and says when it was known and where from:
      </P>
      <AiFields type="Price" extra={{ from: "where the price comes from: `PriceSource.Kavach`, `PriceSource.Program` or `PriceSource.Package(name)`" }} />
      <P>
        <C>Anthropic()</C>&apos;s prices are a table in <C>std.ai</C>, read from{" "}
        <a href="https://platform.claude.com/docs/en/about-claude/pricing">Anthropic&apos;s pricing page</a> on{" "}
        {prices.date}. In US dollars per million tokens:
      </P>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <thead>
            <tr className="border-b border-line text-left text-soft">
              <th className="py-2 pr-4 font-bold">Model</th>
              <th className="py-2 pr-4 font-bold">Input</th>
              <th className="py-2 pr-4 font-bold">Output</th>
              <th className="py-2 pr-4 font-bold">Cached input</th>
              <th className="py-2 font-bold">As of</th>
            </tr>
          </thead>
          <tbody className="text-muted">
            {prices.rows.map((r) => (
              <tr key={r.model} className="border-b border-line align-top">
                <td className="py-2 pr-4"><C>{r.model}</C></td>
                <td className="py-2 pr-4">${r.input}</td>
                <td className="py-2 pr-4">${r.output}</td>
                <td className="py-2 pr-4">${r.cached}</td>
                <td className="py-2">{prices.date}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <Ex name="agents/prices" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Any other model has no price: <C>price</C> gives <C>none</C>.</li>
        <li>Prices are worked out when the program is built, as a provider&apos;s <C>check</C> is. A built program keeps the prices of its build, and their date says how old they are.</li>
      </ul>
      <Note>
        A cost is an estimate. A price may have changed since its date, and the provider&apos;s bill is what counts.
        Two cases make the estimate low: a <C>claude-haiku-5-5</C> prompt over 100,000 tokens costs five times as
        much, and writing to the cache costs 1.25 or 2 times <C>input</C>, but both are counted at the prices above.
        A setting passed through <C>extra</C> that changes the price isn&apos;t seen either: fast mode (Opus 5.5 at $8
        and $40, Fable 5.2 at $16 and $80 per million tokens), US-only inference (<C>inference_geo: &quot;us&quot;</C>,
        1.1 times) and the Batch API (half price).
      </Note>
      <H2>Your own prices</H2>
      <P>
        A program with its own price, such as a negotiated one, wraps the provider and answers <C>price</C> for the
        models it knows, with <C>from: PriceSource.Program</C>. It passes the others on. That is the{" "}
        <C>ShopAi</C> type in the first program on this page. The program&apos;s own price wins, and the note says it
        is the program&apos;s.
      </P>
      <H2>Totals: ai.spent</H2>
      <P>
        <C>ai.spent()</C> gives what every <C>ask</C> of the run took so far, and <C>ai.spent(by: &quot;Reviewer&quot;)</C>{" "}
        what one agent&apos;s took, since the program started. Before any <C>ask</C>, nothing:
      </P>
      <Ex name="agents/spent" />
      <AiFields type="Spent" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>An <C>ask</C> counts once it reached a model, whether it answered or failed. A dry run or a generated test reaches none, so it adds nothing.</li>
        <li><C>ask</C> and <C>ai.ask_full</C> count alike.</li>
        <li>An <C>ask</C> with no known cost is counted in <C>unpriced</C> and left out of <C>cost</C>; the note says how many were left out.</li>
        <li><C>ai.spent</C> needs no capability. Its totals last while the program runs.</li>
      </ul>
      <P>
        To see a prompt&apos;s size before sending it, count it with{" "}
        <Link href="/docs/agents/ask/counting-tokens/">ai.count</Link>, which is free. To stop spending at a budget,
        give an agent a <Link href="/docs/agents/agents/limit/">money limit</Link>,{" "}
        <C>cost &lt;= 5.00 per day</C>. Each <C>ask</C>&apos;s cost is also in the{" "}
        <Link href="/docs/agents/audit-log/">audit log</Link>.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={SPEC_84} label="spec 8.4 · Prices, Usage and cost" />
        <SrcRef file="docs/decisions/0140-ai-providers.md" label="decision 0140" />
        <SrcRef file="stdlib/ai/ai.kav" line={225} symbol="pub type Asked<T>" label="std.ai: Asked" />
        <SrcRef file="stdlib/ai/ai.kav" line={209} symbol="pub type Cost: Display" label="std.ai: Cost" />
        <SrcRef file="stdlib/ai/ai.kav" line={188} symbol="pub type Price" label="std.ai: Price" />
        <SrcRef file="stdlib/ai/ai.kav" line={249} symbol="pub fn spent" label="std.ai: spent" />
        <SrcRef file={ANTHROPIC} line={prices.line} symbol="fn anthropic_prices" label="std.ai: Anthropic's prices" />
        <SrcRef file="runtime/core/src/agents/costs.rs" line={136} symbol="pub fn cost" label="a cost from a price, in the runtime" />
        <SrcRef file="runtime/core/src/agents/costs.rs" line={203} symbol="pub fn note" label="the note, in the runtime" />
        <SrcRef file={TESTS} label="tested end to end" />
        <SrcRef file="tests/conformance/08-capabilities/anthropic-prices-are-dated.kav" label="test: prices are dated" />
      </div>
    </>
  ),
};
