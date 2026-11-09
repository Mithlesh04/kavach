import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { typeChoices } from "@/lib/ai-model";
import { loadProject } from "@/lib/samples";
import { FieldTable } from "./field-table";
import { H2, Note, P, Syntax } from "./parts";

const shared = loadProject("agents/agent-settings", ["src/main.kav"]);
const moved = loadProject("agents/agents-toml.fails", ["kavach.toml"]);

// What each field is for, in plain words, and the values it takes. The
// fields, types and defaults come from std.ai (stdlib/ai/ai.kav).
const settingsMeans: Record<string, React.ReactNode> = {
  mode: <>How the agent treats its limits and approvals: <Link href="/docs/agents/agent-modes/">Agent Modes</Link>.</>,
  audit: <>Turns on the <Link href="/docs/agents/audit-log/">audit log</Link>, a record of every capability call.</>,
  state: <>Keeps limit totals across restarts, so a daily limit stays daily: <Link href="/docs/agents/durable-agents/limits-across-restarts/">Limits across restarts</Link>.</>,
  journal: <>Finishes a handler cut off half way after a restart, without doing anything twice: <Link href="/docs/agents/durable-agents/">Durable Agents</Link>.</>,
  folder: <>Where the agent&apos;s files go: its state, journal and audit log.</>,
  asks_under_way: <>How a <C>tokens</C> limit counts the <C>ask</C>s still waiting for their answer: <Link href="/docs/agents/agents/limit/">limit</Link>, under &quot;Many asks at once&quot;.</>,
};
const settingsRange: Record<string, React.ReactNode> = {
  mode: <>{typeChoices("AgentMode").map((c, i) => <span key={c}>{i > 0 && ", "}<C>AgentMode.{c}</C></span>)}</>,
  audit: <>An <C>Audit</C>, below</>,
  state: <><C>true</C> or <C>false</C></>,
  journal: <><C>true</C> or <C>false</C></>,
  folder: <>A folder&apos;s path, relative to the folder of <C>kavach.toml</C></>,
  asks_under_way: <>{typeChoices("UnderWay").map((c, i) => <span key={c}>{i > 0 && ", "}<C>UnderWay.{c}</C></span>)}</>,
};
const auditMeans: Record<string, React.ReactNode> = {
  file: <>Where the log is written.</>,
  format: <><C>JsonLines</C> writes one JSON object per entry, which log shippers read without a parser: <Link href="/docs/agents/monitoring-agents/">Monitoring Agents</Link>.</>,
  otlp: <>Sends each entry to the OpenTelemetry collector named in <C>kavach.toml</C>&apos;s <C>[log.otlp]</C> too.</>,
};
const auditRange: Record<string, React.ReactNode> = {
  file: <>A file&apos;s path</>,
  format: <>{typeChoices("AuditFormat").map((c, i) => <span key={c}>{i > 0 && ", "}<C>AuditFormat.{c}</C></span>)}</>,
  otlp: <><C>true</C> or <C>false</C></>,
};

// ---------- Agents: settings ----------

export const agentSettingsDoc: LessonDoc = {
  lead: "settings says how an agent runs: its mode, its audit log, and what it keeps across restarts. Agents can share one, and change a field each.",
  simple: <>Settings are an agent&apos;s house rules, written once and shared, like a company handbook. Each agent follows the shared handbook, and one can have its own exception, such as a new trainee who only watches at first.</>,
  minutes: 4,
  before: ["agents/agents/model"],
  body: (
    <>
      <Syntax code={"agent Name {\n    model careful()\n    settings standard() with { mode: AgentMode.Observe }\n    can …\n}"} />
      <P>
        Two agents share <C>standard()</C>, which turns on the audit log. <C>Trainee</C> changes one field with{" "}
        <C>with</C>: it only observes. Both are asked for a refund over their limit:
      </P>
      <CodeBlock code={shared.files[0].code} file="src/main.kav" output={shared.output} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>
          <strong className="text-ink">A function with no parameters shares settings</strong>, as it shares a{" "}
          <Link href="/docs/agents/agents/model/">model</Link>: change <C>standard()</C> once, and every agent that
          calls it changes.
        </li>
        <li><C>with {"{ … }"}</C> changes fields for one agent and keeps the rest, as it does for any <Link href="/docs/types/records/">record</Link>.</li>
        <li>Each agent writes its own files, in its own folder: <C>agents/Refunder/</C> and <C>agents/Trainee/</C>, beside <C>kavach.toml</C>.</li>
        <li>The settings are worked out when the program is built, like the model. A field that doesn&apos;t exist is a compile error.</li>
      </ul>
      <H2>With no settings at all</H2>
      <P>
        An agent with no <C>settings</C> runs in <C>Enforce</C> mode, and in a project it keeps what it needs to
        survive a restart: its limit totals (<C>state.json</C>) and its journal, in <C>agents/&lt;Name&gt;/</C>. The
        audit log is off until the settings turn it on. <C>kavach test</C> and dry runs keep neither, so each starts
        fresh.
      </P>
      <Note>
        A journal runs a handler cut off half way again from its start, so a handler must be safe to run twice. It
        is, as long as it reaches the world only through capabilities: the journal gives back what each call already
        did. The <C>.gitignore</C> that <C>kavach new</C> writes leaves <C>agents/</C> out.
      </Note>
      <H2>Every setting</H2>
      <Syntax code={'AgentSettings(\n    mode: AgentMode.Observe,\n    audit: Audit(format: AuditFormat.JsonLines, otlp: true),\n    state: true,\n    journal: true,\n    folder: "data/refunds",\n)'} />
      <FieldTable type="AgentSettings" means={settingsMeans} range={settingsRange} />
      <P>An <C>Audit</C> turns the audit log on, and says how it is written:</P>
      <FieldTable type="Audit" means={auditMeans} range={auditRange} />
      <H2>Moved from kavach.toml</H2>
      <P>
        These settings used to be <C>[agents.&lt;Name&gt;]</C> sections of <C>kavach.toml</C>. Now an agent says
        how it runs where it is written, and review sees every change. An old section says where each key went:
      </P>
      <CodeBlock code={moved.files[0].code} file="kavach.toml" lang="toml" output={moved.output} outcome="refused" />
      <P>
        <C>kavach.toml</C> keeps one agent setting, <C>[agents] folder</C>, which moves every agent&apos;s folder; an
        agent&apos;s own <C>folder</C> wins over it. See <Link href="/docs/tools/settings/">Settings</Link>.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/08-capabilities-and-agents/05-agent-modes.md" label="spec 8.5 · agent settings" />
        <SrcRef file="docs/decisions/0060-agent-settings-are-chosen-in-code.md" label="decision 0060" />
        <SrcRef file="stdlib/ai/ai.kav" label="std.ai: AgentSettings, as declared" />
      </div>
    </>
  ),
};
