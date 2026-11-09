import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { typeChoices } from "@/lib/ai-model";
import { loadProject } from "@/lib/samples";
import { FieldTable } from "./field-table";
import { Ex, H2, Note, P, Syntax } from "./parts";

const agentModel = loadProject("agents/agent-model.dry", ["src/main.kav"]);

// What each Model field is for, in plain words, and the values it takes.
// The fields themselves, their types and defaults come from
// stdlib/ai/ai.kav; a field without a line here fails the build, so a new
// setting is never left unexplained. "checked when built" means the
// compiler refuses a value outside the range (spec 8.4).
const fieldRange: Record<string, React.ReactNode> = {
  name: <>A model the provider serves: <C>claude-opus-5-5</C>, <C>claude-sonnet-5-5</C>, <C>claude-haiku-5-5</C> and others</>,
  effort: <>{typeChoices("Effort").map((c, i) => <span key={c}>{i > 0 && ", "}<C>Effort.{c}</C></span>)}</>,
  max_tokens: <><C>1</C> to <C>128,000</C>, checked when built</>,
  thinking: <><C>true</C> or <C>false</C>. <C>false</C> is refused when built for <C>claude-opus-5-5</C>, which always thinks, and with <C>effort</C> above <C>High</C></>,
  instructions: <>Any text</>,
  stop: <>A list of texts</>,
  timeout: <>A <C>Duration</C>, such as <C>60.seconds</C></>,
  retries: <><C>0</C> to <C>10</C>, checked when built</>,
  api_url: <>The address of a service that speaks Anthropic&apos;s Messages API: another account, a proxy, a gateway</>,
  api_key_env: <>The name of an environment variable</>,
  extra: <>The text of a JSON object, checked when built</>,
};

const fieldMeans: Record<string, React.ReactNode> = {
  name: <>Which model answers, by the provider&apos;s name for it. Bigger models usually cost more per token; smaller ones are usually quicker and cheaper.</>,
  effort: <>How hard the model works on an answer: more effort, better answers and more tokens. The provider&apos;s main control of quality against cost.</>,
  max_tokens: <>The longest answer allowed, thinking included. An answer cut off at this length fails with <C>Failed</C>, without asking again.</>,
  thinking: <><C>false</C> asks the model to answer straight away, without thinking first: far fewer output tokens, but it can get some answers wrong. See <Link href="/docs/agents/ask/choosing-the-model/">Choosing the model</Link> for what it measured.</>,
  instructions: <>Standing instructions the model gets with every <C>ask</C>, such as who it works for and how to answer: a system prompt.</>,
  stop: <>The answer ends where the model writes one of these texts.</>,
  timeout: <>How long one try may take. A try that takes longer is sent again, like a busy one.</>,
  retries: <>How many times a busy, dropped or late call is sent again, after waits of 1, 2, 4 and 8 seconds, then 8 again, each a random time between half and all of that. Then the <C>ask</C> fails with <C>Failed</C>.</>,
  api_url: <>Where this model&apos;s calls go, instead of Anthropic&apos;s own address.</>,
  api_key_env: <>Which environment variable holds the key for this model, so two models can use two accounts. The key itself never goes in the code.</>,
  extra: <>More request fields, added as they are, for a provider setting Kavach doesn&apos;t name yet. A field here replaces one Kavach would send.</>,
};

// ---------- Agents: model ----------

export const agentModelDoc: LessonDoc = {
  lead: "model names the AI model, and its settings, for every ask in the agent's handlers.",
  simple: <>An agent that asks an AI needs to know which one to ask. <C>model</C> says it once, at the top of the agent, the way a company names one supplier for all its orders instead of choosing again for each one.</>,
  minutes: 3,
  before: ["agents/agents/declaring-an-agent", "agents/ask"],
  body: (
    <>
      <Syntax code={"agent Name {\n    model careful()\n    can Ai\n    …\n}"} />
      <P>
        Two agents share <C>careful()</C>, and a third has no <C>model</C>. The audit log names the model each{" "}
        <C>ask</C> used. This was a dry run, so no model was called and nothing cost:
      </P>
      <CodeBlock code={agentModel.files[0].code} file="src/main.kav" output={agentModel.output} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>model</C> takes a <C>Model</C> value: the model&apos;s name, and the settings in the table below.</li>
        <li>
          <strong className="text-ink">A function with no parameters makes a group.</strong> <C>Reviewer</C> and{" "}
          <C>Auditor</C> both call <C>careful()</C>, so changing that one function changes both.
        </li>
        <li><C>Greeter</C> has no <C>model</C>, so its <C>ask</C> uses the default, <C>claude-haiku-5-5</C>.</li>
        <li>The model is worked out when the program is built, so you can always read which model an agent uses.</li>
      </ul>
      <H2>What a Model takes</H2>
      <Syntax code={'Model(\n    name: "claude-opus-5-5",\n    effort: Effort.High,\n    instructions: "You review refunds for a shop. Be brief.",\n    timeout: 60.seconds,\n)'} />
      <FieldTable type="Model" means={fieldMeans} range={fieldRange} />
      <Note>
        A value marked &quot;checked when built&quot; is refused by the compiler, with a message. Everything else goes to
        the provider as it is, and a value the provider refuses fails the call with <C>Failed</C> and the
        provider&apos;s message. The ranges follow Anthropic&apos;s{" "}
        <a href="https://platform.claude.com/docs/en/api/messages/create">Messages API reference</a>.
      </Note>
      <P>
        <C>temperature</C> is gone: current models take only their own, and newer API versions refuse it. A{" "}
        <C>Model</C> that sets it is refused, with what to use instead:
      </P>
      <Ex name="agents/model-temperature.fails" />
      <P>The compiler also refuses what a model it knows can&apos;t do:</P>
      <Ex name="agents/model-always-thinks.fails" />
      <P>
        One <C>ask</C> can choose another model for itself, and the key, the address calls go to, and what each
        call costs are on <Link href="/docs/agents/ask/choosing-the-model/">Choosing the model</Link>.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/08-capabilities-and-agents/03-agents.md" label="spec 8.3 · model" />
        <SrcRef file="docs/decisions/0056-the-model-is-chosen-in-code.md" label="decision 0056" />
        <SrcRef file="docs/decisions/0059-model-settings-follow-the-providers.md" label="decision 0059" />
        <SrcRef file="stdlib/ai/ai.kav" label="std.ai: Model, as declared" />
      </div>
    </>
  ),
};
