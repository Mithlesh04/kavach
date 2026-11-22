import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import { rustConst, rustLet } from "@/lib/rust-tests";
import { Ex, H2, P } from "./parts";

// A `cost` limit in `limit Ai` (spec 8.3, decision 0140 choice 9). The
// program and its output are read from the end-to-end test, which runs it
// against a fake server.

const TESTS = "tools/cli/tests/ai_costs.rs";
const CROSSING = "a_cost_limit_lets_the_crossing_ask_run_then_refuses_and_totals_add_up";
const NO_PRICE = "a_cost_limit_refuses_a_model_with_no_price";

/** The test's `LIMITED` program as `CROSSING` runs it. */
function limited(): string {
  const text = rustConst(TESTS, "LIMITED");
  if (!text.includes("MODEL") || !text.includes("LIMIT")) throw new Error(`${TESTS}: LIMITED changed shape`);
  return text.replace("MODEL", '"claude-opus-5-5"').replace("LIMIT", "0.001");
}

export function MoneyBudget() {
  const refused = rustLet(TESTS, NO_PRICE, "refused").trim().replace(/^failed: /, "");
  return (
    <>
      <H2>A money budget for AI calls</H2>
      <P>
        <C>cost</C> counts US dollars the way <C>tokens</C> counts tokens: what the window&apos;s earlier{" "}
        <C>ask</C>s cost, worked out from each model&apos;s price (
        <Link href="/docs/agents/ask/usage-and-cost/">usage and cost</Link>). It is a <C>Decimal</C>. Here one{" "}
        <C>ask</C> costs about a third of a cent, against a budget of a tenth of a cent a day:
      </P>
      <CodeBlock code={limited()} file="main.kav, run against a test model" output={rustLet(TESTS, CROSSING, "want")} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>The first <C>ask</C> starts at $0, so it runs, and crosses the budget: a cost is known only after the answer.</li>
        <li>The second is refused before it is sent. The refusal gives the cost so far and says why it can be over the budget.</li>
        <li><C>ai.spent</C> counts only the <C>ask</C> that reached the model.</li>
        <li><C>ask</C>s still waiting for their answer count as the agent&apos;s <C>asks_under_way</C> setting says, as for <C>tokens</C>.</li>
        <li>A reply that gave no token counts costs 0 here, as it takes no <C>tokens</C>.</li>
      </ul>
      <P>
        A model with no known price can&apos;t be counted, so an <C>ask</C> that could reach one is refused, never
        counted as free. With <C>claude-opus-4-1</C>, which Kavach has no price for, each <C>ask</C> fails with{" "}
        <C>{refused}</C>. That holds for any model of a list, even when an earlier one would answer. Give the price
        by wrapping the provider (<Link href="/docs/agents/ask/usage-and-cost/#your-own-prices">your own prices</Link>).
      </P>
      <P>
        Counting a prompt&apos;s tokens with <Link href="/docs/agents/ask/counting-tokens/">ai.count</Link> is free,
        so no rule of <C>limit Ai</C> counts or refuses it, <C>count</C> included.
      </P>
      <P>Money is counted only for AI calls, so on any other capability <C>cost</C> is a compile error:</P>
      <Ex name="agents/limit-cost-other.fails" />
      <P>The budget is a number of dollars, not text:</P>
      <Ex name="agents/limit-cost-text.fails" />
      <P>A rule reads <C>tokens</C> or <C>cost</C>, not both. Write two rules instead:</P>
      <Ex name="agents/limit-cost-both.fails" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="runtime/core/src/agents.rs" line={245} symbol="pub costs" label="the rules that read cost, in the runtime" />
        <SrcRef file={TESTS} line={152} symbol={CROSSING} label="the money budget's test" />
        <SrcRef file="tests/conformance/08-capabilities/cost-is-for-limit-ai.kav" label="test: cost is for limit Ai" />
        <SrcRef file="tests/conformance/08-capabilities/a-rule-reads-tokens-or-cost.kav" label="test: tokens or cost" />
      </div>
    </>
  );
}
