import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { loadProject } from "@/lib/samples";
import { fileTables, specBlock, specTables } from "@/lib/spec-tables";
import { Ex, H2, Note, P, Syntax } from "./parts";
import { Limits } from "./limits";

/** A table of a decision record or README, as it is written there: the one at `index`, or the one whose header starts with `first`. */
function DocTable({ rel, index, first }: { rel: string; index?: number; first?: string }) {
  const all = fileTables(rel);
  const t = first ? all.find((x) => x.header.join("|").startsWith(first)) : all[index ?? 0];
  if (!t) throw new Error(`${rel}: no table ${first ?? index}`);
  return (
    <div className="overflow-x-auto">
      <table className="w-full border-collapse text-[15px]">
        <thead>
          <tr className="border-b border-line text-left text-soft">
            {t.header.map((h, i) => (
              <th key={i} className="py-2 pr-4 font-bold">{h.replace(/`/g, "")}</th>
            ))}
          </tr>
        </thead>
        <tbody>
          {t.rows.map((r, i) => (
            <tr key={i} className="border-b border-line align-top">
              {r.map((c, j) => (
                <td key={j} className={`py-2 pr-4 ${j ? "text-muted" : "text-ink"}`}>{c.replace(/`/g, "")}</td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

// ---------- ask<T> ----------

export const askDoc: LessonDoc = {
  lead: "ask<T>(prompt) asks an AI model and gives back a checked value of type T, never loose text to parse.",
  simple: <>Using <C>ask</C> is like handing someone a form with boxes to fill in, instead of asking for a letter. You get the answer in the boxes you asked for, checked, or else an error.</>,
  minutes: 5,
  before: ["agents/agents", "types/records"],
  body: (
    <>
      <Syntax code={"const answer = try ask<Type>(\"the prompt, with {values}\")"} />
      <Ex
        name="agents/sub-ask.dry"
        notes={[
          { target: "/// true if the review is positive", chip: "TELLS THE MODEL", kind: "new", what: "Doc comments become field descriptions in the schema sent to the model.", why: "the type is the prompt's contract, written once, in code." },
          { target: "ask<Verdict>", chip: "TYPED ANSWER", kind: "new", what: "The answer is checked against Verdict: bad output is asked again, then an error.", why: "your code gets a Bool it can trust." },
        ]}
      />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>ask</C> needs the <C>Ai</C> capability, so it is in the agent&apos;s <C>can</C>.</li>
        <li>Kavach builds a schema from <C>Verdict</C> and sends it with the prompt. The <C>{"///"}</C> comments describe each field to the model.</li>
        <li>The answer is checked against <C>Verdict</C>. A wrong answer is asked again, twice by default, then fails.</li>
      </ul>
      <Note>
        This output comes from <C>kavach run --dry-run</C>: no model is called and nothing costs. <C>ask</C> gets a
        made-up <C>Verdict</C>, the same on every run, and the last line shows the simulated call. Without{" "}
        <C>--dry-run</C>, the model the code chooses answers: see{" "}
        <Link href="/docs/agents/ask/choosing-the-model/">Choosing the model</Link>.
      </Note>
      <H2>Limits and sizes</H2>
      <Limits id="ai" />
    </>
  ),
  quiz: [
    {
      type: "choice",
      prompt: <>What can <C>ask&lt;Verdict&gt;(…)</C> give your code?</>,
      options: [
        "A Verdict that passed the checks, or an error",
        "Whatever text the model wrote",
        "A Verdict, or an empty value when the model is unsure",
      ],
      answer: 0,
      explain: "The answer is checked against Verdict. One that doesn't fit is asked again, then the call fails.",
    },
  ],
  deeper: (
    <>
      <H2>How ask asks: the type in the prompt</H2>
      <P>
        The first try writes <C>T</C> into the prompt in a short form: its fields, their types, the choices of a union,
        and the <C>{"///"}</C> notes. Kavach checks the answer against <C>T</C>. Only an answer that doesn&apos;t fit
        is asked again in the provider&apos;s structured-output mode, which holds the model to the type. Measured when
        this was decided:
      </P>
      <DocTable rel="docs/decisions/0057-ask-puts-the-type-in-the-prompt.md" index={0} />
      <DocTable rel="docs/decisions/0057-ask-puts-the-type-in-the-prompt.md" index={1} />
      <P>
        So answers are as good, for about a third of the input tokens. How this adds up on whole tasks, against
        Python&apos;s usual ways: <Link href="/docs/agents/agents-in-practice/">Agents in Practice</Link>.
      </P>
      <div><SrcRef file="docs/decisions/0057-ask-puts-the-type-in-the-prompt.md" label="decision 0057" /></div>
    </>
  ),
};

export const typedAnswersDoc: LessonDoc = {
  lead: "The answer's type can hold anything a type can: true or false, text, numbers, lists and unions.",
  simple: <>Like a multiple-choice question instead of an open one: when the answer must be one of three teams, the AI can&apos;t make up a fourth. The answer can be yes or no, words, numbers, a list, or one of a fixed set of choices.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="agents/ask-triage.dry" />
      <P>
        <C>Category</C> is a union, so the model must pick one of its three cases, and <C>match</C> handles each.
        The model can&apos;t answer with a fourth team, because that answer wouldn&apos;t be a <C>Triage</C>.
      </P>
      <Note>The answers in a dry run are made up from the type, not real judgements: that is why this one sends a billing question to the help desk.</Note>
    </>
  ),
};

export const aiFailsDoc: LessonDoc = {
  lead: "ask can fail: no key, a network or provider error, an answer that never fits the type, or a limit. Handle it like any error.",
  simple: <>Asking an AI is like making a phone call: you may have no number to dial, the line may be down, or the answer may not make sense. Each of these is an ordinary error your program can handle.</>,
  minutes: 3,
  body: (
    <>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <thead>
            <tr className="border-b border-line text-left text-soft">
              <th className="py-2 pr-4 font-bold">Error</th>
              <th className="py-2 font-bold">When</th>
            </tr>
          </thead>
          <tbody>
            {[
              ["NoKey(variable)", "the environment variable with the key, ANTHROPIC_API_KEY, isn't set"],
              ["Failed(status, message)", "the provider refused, or the network failed (status 0)"],
              ["InvalidOutput(answer)", "the answer still didn't fit the type after asking again"],
              ["Denied(reason)", "a limit on Ai refused the call"],
            ].map(([e, when]) => (
              <tr key={e} className="border-b border-line">
                <td className="py-2 pr-4 whitespace-nowrap"><C>{e}</C></td>
                <td className="py-2 text-muted">{when}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <P>
        <C>ask</C> fails with an <C>AiError</C>, so <C>match</C> picks the cases you can handle. Here an answer that
        never fits becomes a safe fallback, and anything else is passed on with <C>fail e</C>:
      </P>
      <Ex name="agents/ask-fail.dry" />
      <P>
        Put a <C>limit Ai</C> on an agent to cap its model calls, <C>count &lt;= 1_000 per day</C>, or the tokens they
        take, <C>tokens &lt;= 200_000 per day</C>: past it, <C>ask</C> fails with <C>Denied</C> instead of running up a
        bill. See <Link href="/docs/agents/agents/limit/">limit</Link>.
      </P>
      <Note>
        A busy provider (status 429, 529 or 5xx) or a dropped connection isn&apos;t a failure yet: the call is sent again
        after a wait that doubles each time (1, 2, 4 and 8 seconds, then 8 again), as the providers&apos; own libraries
        do, and only then fails with <C>Failed</C>. Each wait is a random time between half and all of that, so calls
        turned away together don&apos;t all come back together.
      </Note>
      <P>
        Many <C>ask</C>s at once, one per task, can meet a provider&apos;s limit on calls at the same time. The
        program&apos;s <C>ask</C>s share a gate. After a call is turned away as busy, no more calls are sent at once than
        the provider was taking then, and each call that succeeds lets a little more through, up to 256 at once. A call
        waiting at the gate isn&apos;t a retry. So 300 <C>ask</C>s started together all get their answer, as fast as the
        provider allows.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/08-capabilities-and-agents/04-typed-ai-calls.md" label="spec 8.4 · busy providers" />
        <SrcRef file="runtime/net/src/ai/gate.rs" label="the gate" />
        <SrcRef file="tools/cli/tests/ai_crowd.rs" line={103} symbol="forty_asks_at_once_all_get_through_a_provider_that_takes_four" label="its test: 40 asks, a provider that takes 4" />
      </div>
    </>
  ),
};

/** The [ai] rows of spec 10.11's settings table. */
function AiSettings() {
  const toml = specTables("10-modules-and-packages/11-settings-reference.md", "10.11").find((t) => t.heading === "kavach.toml");
  const rows = toml?.rows.filter((r) => r[0] === "`[ai]`") ?? [];
  if (!rows.length) throw new Error("spec 10.11: no [ai] settings");
  return (
    <div className="overflow-x-auto">
      <table className="w-full border-collapse text-[15px]">
        <thead>
          <tr className="border-b border-line text-left text-soft">
            <th className="py-2 pr-4 font-bold">Key</th>
            <th className="py-2 pr-4 font-bold">Values</th>
            <th className="py-2 font-bold">Default</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((r) => (
            <tr key={r[1]} className="border-b border-line">
              <td className="py-2 pr-4"><C>{r[1].replace(/`/g, "")}</C></td>
              <td className="py-2 pr-4 text-muted">{r[2].replace(/`/g, "")}</td>
              <td className="py-2"><C>{r[3].replace(/`/g, "")}</C></td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

const tomlMoved = loadProject("agents/model-toml.fails", ["kavach.toml"]);
const withModel = loadProject("agents/ask-model.dry", ["src/main.kav"]);

export const modelDoc: LessonDoc = {
  lead: "An agent chooses its AI model in code, with model; a single call can choose another, with model:. The key comes from an environment variable, never from a file.",
  simple: <>Choosing a model is like choosing which expert to call: a careful one for hard questions, a quick one for easy ones. The choice is written in the code, so anyone can read it, and the secret key stays out of your files.</>,
  minutes: 5,
  body: (
    <>
      <CodeBlock code={withModel.files[0].code} file="src/main.kav" output={withModel.output} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>model careful()</C> sets the model for every <C>ask</C> in the agent&apos;s handlers.</li>
        <li><C>ask&lt;T&gt;(prompt, model: quick())</C> chooses another for one call. The call&apos;s model wins over the agent&apos;s.</li>
        <li>With neither, <C>ask</C> uses <C>claude-haiku-5-5</C> with answers of at most 4,096 tokens.</li>
        <li>This ran with <C>kavach run --dry-run</C>, so no model was called and nothing cost.</li>
      </ul>
      <H2>A Model</H2>
      <Syntax code={"Model(name: \"claude-opus-5-5\", effort: Effort.High, max_tokens: 2000)"} />
      <P>
        Every setting, its type, what it is when left out and what it does are in one table, on{" "}
        <Link href="/docs/agents/agents/model/">Agents: model</Link>, with how agents share one model through a
        function like <C>careful()</C>.
      </P>
      <H2>Thinking before answering</H2>
      <P>
        Most current models think before they answer, and for a short answer most of the output tokens are that
        thinking. <C>thinking: false</C> asks the model to answer straight away, as <C>quick()</C> does above. Kavach
        sends each model the form it takes. A model that always thinks, such as <C>claude-opus-5-5</C>, can&apos;t turn
        it off, and the compiler says so: lower its <C>effort</C> instead.
      </P>
      <P>It saves tokens, but can cost answers. Measured in the agent benchmark, three runs each:</P>
      <DocTable rel="benchmarks/agents/README.md" first="Case|Right, thinking on" />
      <P>
        Off can get an answer wrong that on gets right, so Kavach keeps the model&apos;s own default. Turn it off where
        a short, simple answer is enough, and check the answers. The whole benchmark is in{" "}
        <Link href="/docs/agents/agents-in-practice/">Agents in Practice</Link>.
      </P>
      <H2>Known when the program is built</H2>
      <P>The model is worked out when the program is built, so you can always read which model an agent uses. A model that depends on a value while running is refused:</P>
      <Ex name="agents/model-known.fails" />
      <P>Choosing it in <C>kavach.toml</C>, as before, now says where it moved:</P>
      <CodeBlock code={tomlMoved.files[0].code} file="kavach.toml" lang="toml" output={tomlMoved.output} outcome="refused" />
      <P><C>[ai]</C> keeps only the provider:</P>
      <AiSettings />
      <H2>The key and the address</H2>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>The key is read from <C>ANTHROPIC_API_KEY</C>, or from the variable a <C>Model</C>&apos;s <C>api_key_env</C> names. Without it, <C>ask</C> fails with <C>NoKey</C>, naming the variable.</li>
        <li>A <C>Model</C>&apos;s <C>api_url</C> sends its calls to another address, such as another account, a proxy or a gateway. <C>KAVACH_AI_URL</C> does the same for every call without one, such as a test server.</li>
        <li>An answer cut off at <C>max_tokens</C> fails with <C>Failed</C>, without asking again.</li>
      </ul>
      <H2>In the audit log</H2>
      <P>
        Each <C>ask</C> an agent makes is in its audit log with the model it used. In a dry run, as in the output
        above, the line names the model a real run would use, so you can check each call&apos;s model before paying for
        one. In a real run it gives the tokens too, so what each agent costs can be read from its log:
      </P>
      <CodeBlock code={specBlock("08-capabilities-and-agents/06-audit-log.md", "8.6", 1)} file="audit.log, as spec 8.6 shows it" lang="text" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/08-capabilities-and-agents/04-typed-ai-calls.md" label="spec 8.4 · ask<T>" />
        <SrcRef file="docs/decisions/0056-the-model-is-chosen-in-code.md" label="decision 0056" />
      </div>
    </>
  ),
};
