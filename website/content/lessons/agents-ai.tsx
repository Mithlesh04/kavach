import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { loadProject } from "@/lib/samples";
import { fileTables, specBlock, specBlockWith, specTables } from "@/lib/spec-tables";
import { Ex, H2, Note, P, Syntax } from "./parts";
import { ModuleTable, Table } from "./spec-table";
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
      <Syntax code={"const answer = try ask<Type>(`the prompt, with {values}`)"} />
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

const SPEC_84 = "08-capabilities-and-agents/04-typed-ai-calls.md";

/** AiError's variants, each with its comment, as spec 8.4 declares them. */
function aiErrors(): [string, string][] {
  const lines = specBlockWith(SPEC_84, "8.4", "type AiError").split("\n");
  const found = lines.map((l) => l.match(/^(?:type AiError =|\s*\|)\s*(\w+(?:\([^)]*\))?)\s*\/\/\s*(.+)$/));
  if (found.some((m) => !m)) throw new Error("spec 8.4: an AiError line this page can't read");
  return found.map((m) => [m![1], m![2]]);
}

export const aiFailsDoc: LessonDoc = {
  lead: "ask can fail: no key, a busy or refusing provider, an answer that never fits the type, or a limit. Each kind of trouble has its own AiError, the same for every provider.",
  simple: <>Asking an AI is like making a phone call: you may have no number to dial, the line may be busy or down, or the answer may not make sense. Each of these is an ordinary error with its own name, so your program can handle each one its own way.</>,
  minutes: 5,
  body: (
    <>
      <P>
        <C>ask</C> fails with an <C>AiError</C>. Each kind of trouble has a variant of its own, which means the same
        whichever provider the model is reached through:
      </P>
      <Table header={["Error", "When"]} rows={aiErrors().map(([e, when]) => [`\`${e}\``, when])} />
      <P>
        So <C>match</C> picks the cases you can handle. Here an answer that never fits becomes a safe fallback, and
        anything else is passed on with <C>fail e</C>:
      </P>
      <Ex name="agents/ask-fail.dry" />
      <P>Each kind can get its own answer. A program that tells a person what to do next:</P>
      <Ex name="agents/ai-error-kinds" />
      <P>
        Put a <C>limit Ai</C> on an agent to cap its model calls, <C>count &lt;= 1_000 per day</C>, or the tokens they
        take, <C>tokens &lt;= 200_000 per day</C>: past it, <C>ask</C> fails with <C>Denied</C> instead of running up a
        bill. See <Link href="/docs/agents/agents/limit/">limit</Link>.
      </P>
      <H2>What Anthropic&apos;s answers become</H2>
      <P>
        The provider reads each answer and says what kind of trouble it is. For <C>Anthropic()</C>, the provider a{" "}
        <C>Model</C> has unless it names another:
      </P>
      <ModuleTable file={SPEC_84} number="8.4" first="Anthropic's answer" />
      <P>
        Another provider reads its own answers, but gives the same kinds: a 429 that means &quot;the money ran
        out&quot; is <C>QuotaUsedUp</C>, not <C>RateLimited</C>. How a provider reads them:{" "}
        <Link href="/docs/agents/ask/providers/">Providers</Link>.
      </P>
      <H2>Sent again, by kind</H2>
      <P>
        A call turned away as rate limited or overloaded, one the network drops, or one that takes longer than its{" "}
        <C>timeout</C> isn&apos;t a failure yet. It is sent again after a wait that doubles each time (1, 2, 4 and 8
        seconds, then 8 again), <C>retries</C> times (4 unless the <C>Model</C> says), as the providers&apos; own
        libraries do. Only then does <C>ask</C> fail, with <C>RateLimited</C>, <C>Overloaded</C>, <C>Network</C> or{" "}
        <C>TimedOut</C>.
      </P>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Each wait is a random time between half and all of that, so calls turned away together don&apos;t all come back together.</li>
        <li>When the provider says how long to wait (Anthropic&apos;s <C>retry-after</C> header), the wait is at least that long, but never longer than the <C>Model</C>&apos;s <C>timeout</C>.</li>
        <li>What is sent again depends on the kind, not on the status alone. <C>QuotaUsedUp</C>, <C>BadKey</C>, <C>NotFound</C>, <C>BadRequest</C>, <C>Refused</C> and <C>CutOff</C> are never sent again: waiting can&apos;t change them.</li>
      </ul>
      <H2>One gate for each provider and key</H2>
      <P>
        Many <C>ask</C>s at once, one per task, can meet a provider&apos;s limit on calls at the same time. The{" "}
        <C>ask</C>s that go to one provider with one key share a gate. After a call is turned away as rate limited, no
        more calls are sent at once than the others under way then, which the provider took. Each call that succeeds
        lets a little more through, up to 256 at once. A call waiting at the gate isn&apos;t a retry. So 300{" "}
        <C>ask</C>s started together all get their answer, as fast as the provider allows.
      </P>
      <P>
        Each provider and key has a gate of its own. So one provider&apos;s limit never slows the calls to another, nor
        to the same provider with another key.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/08-capabilities-and-agents/04-typed-ai-calls.md" label="spec 8.4 · ask<T>" />
        <SrcRef file="docs/decisions/0140-ai-providers.md" label="decision 0140" />
        <SrcRef file="tests/conformance/08-capabilities/ai-errors-tell-troubles-apart.kav" label="test: errors told apart" />
        <SrcRef file="runtime/net/src/ai/tests.rs" line={176} symbol="a_rate_limited_call_waits_as_the_provider_asks_then_goes_again" label="test: retry-after is honoured" />
        <SrcRef file="runtime/net/src/ai/tests.rs" line={186} symbol="a_quota_used_up_is_never_sent_again" label="test: a used-up quota isn't sent again" />
        <SrcRef file="runtime/net/src/ai/gate.rs" label="the gate" />
        <SrcRef file="runtime/net/src/ai/tests.rs" line={300} symbol="each_provider_and_key_has_its_own_gate" label="test: a gate for each provider and key" />
        <SrcRef file="tools/cli/tests/ai_crowd.rs" line={107} symbol="forty_asks_at_once_all_get_through_a_provider_that_takes_four" label="test: 40 asks, a provider that takes 4" />
      </div>
    </>
  ),
  quiz: [
    {
      type: "choice",
      prompt: <>The account&apos;s money ran out. What does <C>ask</C> do?</>,
      options: [
        "Fails at once with QuotaUsedUp",
        "Waits and sends the call again, then fails with RateLimited",
        "Fails with Failed(status: 402, …)",
      ],
      answer: 0,
      explain: "Waiting can't put money in the account, so QuotaUsedUp is never sent again.",
    },
  ],
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
      <H2>Setting the key</H2>
      <P>
        <C>ask</C> needs an Anthropic API key. Kavach reads it when the program runs, from the environment variable{" "}
        <C>ANTHROPIC_API_KEY</C>. It never goes in the code or in <C>kavach.toml</C>, which are committed. The easiest
        place is the project&apos;s <C>.env</C> file, beside <C>kavach.toml</C>:
      </P>
      <CodeBlock code={"ANTHROPIC_API_KEY=sk-ant-..."} file=".env" lang="text" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>kavach new</C> puts <C>.env</C> in <C>.gitignore</C>, so the key stays out of the repository.</li>
        <li><C>kavach run</C> and <C>kavach test</C> read it. The file&apos;s rules are on <Link href="/docs/tools/environment-variables/">Environment Variables</Link>.</li>
      </ul>
      <P>Or set it in the terminal that runs the program. It lasts as long as that terminal:</P>
      <CodeBlock code={"# macOS and Linux (bash, zsh)\nexport ANTHROPIC_API_KEY=sk-ant-...\nkavach run src/main.kav"} file="terminal" lang="sh" />
      <CodeBlock code={"# Windows PowerShell\n$env:ANTHROPIC_API_KEY = \"sk-ant-...\"\nkavach run src/main.kav"} file="terminal" lang="text" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Where both give a key, the terminal&apos;s wins. The order, strongest first: <C>--env</C>, the terminal, <C>[env.vars]</C>, then <C>.env</C>.</li>
        <li><C>kavach build</C> copies <C>.env</C> beside the executable, key and all, so keep that folder private. Or build with <C>--no-env-file</C> and give the key where the program runs: the shell, a service manager, or <C>docker run -e ANTHROPIC_API_KEY app</C>.</li>
        <li>For a model that uses another account, give it <C>api_key_env: &quot;SUPPORT_KEY&quot;</C> and set <C>SUPPORT_KEY</C> the same way.</li>
        <li><C>kavach run --dry-run</C> calls no model, so it needs no key: check a program that way first.</li>
      </ul>
      <H2>The key and the address</H2>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>The key is read from the variable a <C>Model</C>&apos;s <C>api_key_env</C> names, or else the provider&apos;s own: <C>ANTHROPIC_API_KEY</C> for Anthropic. Without it, <C>ask</C> fails with <C>NoKey</C>, naming the variable. A provider that needs no key, such as a server on the same machine, reads no variable.</li>
        <li>A <C>Model</C>&apos;s <C>api_url</C> sends its calls to another address, such as another account, a proxy or a gateway.</li>
        <li>
          The variable <C>KAVACH_AI_URL</C> sends every call whose <C>Model</C> has no <C>api_url</C> to another address,
          such as a test server or a company proxy. The call keeps the path the provider wrote, after{" "}
          <C>KAVACH_AI_URL</C>&apos;s own path: with <C>KAVACH_AI_URL=http://127.0.0.1:5/proxy</C>, Anthropic&apos;s{" "}
          <C>https://api.anthropic.com/v1/messages</C> goes to <C>http://127.0.0.1:5/proxy/v1/messages</C>.
        </li>
        <li>An answer cut off at <C>max_tokens</C> fails with <C>CutOff</C>, without asking again.</li>
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
        <SrcRef file="runtime/net/src/ai.rs" line={157} label="where the key is read" />
        <SrcRef file="runtime/net/src/ai/tests.rs" line={148} symbol="the_key_goes_where_the_provider_says_and_the_answer_is_read" label="a key in .env, tested" />
        <SrcRef file="runtime/net/src/ai/tests.rs" line={265} symbol="kavach_ai_url_keeps_the_providers_path" label="KAVACH_AI_URL keeps the path, tested" />
      </div>
    </>
  ),
};
