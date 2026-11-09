import fs from "node:fs";
import path from "node:path";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import { rustConst } from "@/lib/rust-tests";
import { specTables } from "@/lib/spec-tables";
import { Ex, H2, Note, P } from "./parts";

// A `tokens` limit with many `ask`s at once (spec 8.3, "`ask`s under way").
// The settings' table comes from the spec; the program, its numbers and how
// many `ask`s each setting lets run come from the test that checks them in
// the interpreter and native code (tools/cli/tests/token_budget.rs).

const TEST = "tools/cli/tests/token_budget.rs";
const TEST_FN = "each_way_of_counting_asks_under_way_keeps_its_promise";
const code = (text: string) => text.split("`").map((part, i) => (i % 2 ? <C key={i}>{part}</C> : part));

function fromTest() {
  const text = fs.readFileSync(path.join(process.cwd(), "..", TEST), "utf8").replace(/\r\n/g, "\n");
  const doc = text
    .split("\n")
    .filter((l) => l.startsWith("//!"))
    .map((l) => l.replace(/^\/\/! ?/, ""))
    .join(" ")
    .replace(/\s+/g, " ");
  const fnAt = text.indexOf(`fn ${TEST_FN}(`);
  const runs = [...text.slice(fnAt).matchAll(/\("(\w+)", (\d+)\)/g)].map((m) => ({ way: m[1], runs: Number(m[2]) }));
  const why = Object.fromEntries([...doc.matchAll(/- `(\w+)`: (.+?)(?=;?\s+- `|\.?$)/g)].map((m) => [m[1], m[2]]));
  const most = doc.match(/each `ask` at most ([^.]+?)\. How many/)?.[1];
  const answer = doc.match(/each answer (\d+) tokens/)?.[1];
  if (fnAt < 0 || runs.length < 3 || !most || !answer || runs.some((r) => !why[r.way] || !why[r.way].endsWith(String(r.runs)))) {
    throw new Error(`${TEST}: the asks-under-way test changed shape`);
  }
  return { runs, why, most, answer, line: text.slice(0, fnAt).split("\n").length };
}

/** How a `tokens` limit counts the `ask`s still waiting for their answer, each way, with the test that checks it. */
export function AsksUnderWay() {
  const table = specTables("08-capabilities-and-agents/03-agents.md", "8.3").find((t) => t.header[0] === "`asks_under_way`");
  if (!table) throw new Error("spec 8.3: no asks_under_way table");
  const t = fromTest();
  const program = rustConst(TEST, "PROGRAM");
  const budget = program.match(/tokens <= (\d+) per day/)?.[1];
  const asks = program.match(/for _ in 0\.\.(\d+)/)?.[1];
  if (!budget || !asks) throw new Error(`${TEST}: PROGRAM no longer has its budget and its count of asks`);
  const head = "py-2 pr-4 font-bold";
  return (
    <>
      <H2>Many asks at once</H2>
      <P>
        Many <C>ask</C>s can run at once, one per task, and each answer&apos;s size is known only when it comes. So{" "}
        <C>tokens</C> also counts the <C>ask</C>s still waiting for their answer. The agent&apos;s settings choose how,
        with <C>asks_under_way</C>:
      </P>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <thead>
            <tr className="border-b border-line text-left text-soft">
              {table.header.map((h) => <th key={h} className={head}>{code(h)}</th>)}
            </tr>
          </thead>
          <tbody>
            {table.rows.map((r) => (
              <tr key={r[0]} className="border-b border-line align-top">
                <td className="py-2 pr-4 whitespace-nowrap">{code(r[0])}</td>
                <td className="py-2 pr-4 text-muted">{code(r[1])}</td>
                <td className="py-2 text-muted">{code(r[2])}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <P>
        Here {asks} <C>ask</C>s start together under a budget of {budget} tokens a day, and each answer takes {t.answer} tokens.
        The test runs it once with each setting, in the interpreter and in native code:
      </P>
      <CodeBlock code={program.replace("UnderWay.WAY", "UnderWay.Average")} file="main.kav, run against a test model" />
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <thead>
            <tr className="border-b border-line text-left text-soft">
              <th className={head}>asks_under_way</th>
              <th className={head}>Answered</th>
              <th className="py-2 font-bold">Why</th>
            </tr>
          </thead>
          <tbody>
            {t.runs.map((r) => (
              <tr key={r.way} className="border-b border-line align-top">
                <td className="py-2 pr-4 whitespace-nowrap"><C>UnderWay.{r.way}</C></td>
                <td className="py-2 pr-4">{r.runs} of {asks}</td>
                <td className="py-2 text-muted">{code(t.why[r.way])}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <P>
        Under <C>Largest</C>, each <C>ask</C> counts the most it could take: for each of its tries, its prompt&apos;s
        length in bytes and its model&apos;s <C>max_tokens</C>. Here that is {t.most}.
      </P>
      <Note>In <C>Observe</C> mode nothing waits: the limit is only reported.</Note>
      <P>The setting is chosen when the program is built, like the rest of the settings:</P>
      <Ex name="agents/under-way-known.fails" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/08-capabilities-and-agents/03-agents.md" label="spec 8.3 · asks under way" />
        <SrcRef file={TEST} line={t.line} symbol={TEST_FN} label="the test of each setting" />
        <SrcRef file="tests/conformance/08-capabilities/asks-under-way-is-known-when-built.kav" label="its conformance test" />
      </div>
    </>
  );
}
