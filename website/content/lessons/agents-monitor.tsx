import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { loadProject } from "@/lib/samples";
import { H2, Note, P } from "./parts";

const monitor = loadProject("agents/monitor", ["src/main.kav"]);
const auditJson = loadProject("agents/audit-json.dry", ["src/main.kav"]);
const otlpWrong = loadProject("agents/audit-otlp.fails", ["src/main.kav"]);

// ---------- Monitoring Agents ----------

export const monitorDoc: LessonDoc = {
  lead: "Two records say what an agent did: the audit log the runtime writes for every capability call, and the events the agent logs itself, which go on to the monitoring tools you already run.",
  simple: <>Like a shop with two records: the till roll that prints every sale by itself, and the manager&apos;s notes on what each sale meant. Both can be sent on to the monitoring tools your team already watches.</>,
  minutes: 5,
  before: ["agents/audit-log", "standard-library/logging"],
  body: (
    <>
      <CodeBlock code={monitor.files[0].code} file="src/main.kav" output={monitor.output} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><strong className="text-ink">The audit log</strong>, turned on by <C>Audit()</C> in the agent&apos;s <Link href="/docs/agents/agents/settings/">settings</Link>, is written by the runtime, so no call can be left out: each one with its arguments, the verdict and the result, refused calls included.</li>
        <li>An <C>ask</C> in the audit log also names the model that answered and the tokens it took, so what each agent costs is there too: see <Link href="/docs/agents/ask/choosing-the-model/">Choosing the model</Link>.</li>
        <li><strong className="text-ink">The agent&apos;s own events</strong>, <C>RefundIssued</C> and <C>RefundBlocked</C>, say what each call meant for the business, in fields you choose.</li>
      </ul>
      <H2>Sending them to your tools</H2>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <thead>
            <tr className="border-b border-line text-left text-soft">
              <th className="py-2 pr-4 font-bold">Record</th>
              <th className="py-2 pr-4 font-bold">Holds</th>
              <th className="py-2 font-bold">Goes to, today</th>
            </tr>
          </thead>
          <tbody>
            <tr className="border-b border-line align-top">
              <td className="py-2 pr-4 text-ink">The audit log</td>
              <td className="py-2 pr-4 text-muted">every capability call: arguments, verdict, result; secrets as <C>&lt;secret&gt;</C></td>
              <td className="py-2 text-muted">a file in the agent&apos;s folder, as text or as JSON lines, and with <C>otlp: true</C> also any OpenTelemetry collector</td>
            </tr>
            <tr className="border-b border-line align-top">
              <td className="py-2 pr-4 text-ink">The agent&apos;s log events</td>
              <td className="py-2 pr-4 text-muted">records you define, one line each</td>
              <td className="py-2 text-muted">the terminal, files that rotate by size, and any OpenTelemetry collector: Grafana, Datadog, Elastic, Honeycomb and others read from there</td>
            </tr>
          </tbody>
        </table>
      </div>
      <P>To send the log events to a collector, name it in <C>kavach.toml</C>. The program&apos;s owner chooses where events go, not its code:</P>
      <CodeBlock
        code={'[log]\nlevel = "info"\nfile = "logs/agents.log"      # also kept on disk, a new file every 10 MB\n\n[log.otlp]\nendpoint = "http://localhost:4318"   # OTLP over HTTP\nservice = "refunds"'}
        file="kavach.toml"
        lang="toml"
      />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Each event becomes an OpenTelemetry log record: its level is the severity, its printed form the body, and its type&apos;s name the attribute <C>event.name</C>.</li>
        <li>Events are sent in batches by a thread of their own, so a slow collector never slows the agent.</li>
        <li>Logging needs no capability, so an agent&apos;s <C>can</C> doesn&apos;t change when you add it.</li>
      </ul>
      <H2>The audit log as JSON lines</H2>
      <P>
        A log shipper reads JSON without a parser written for it. With <C>Audit(format: AuditFormat.JsonLines)</C> in the
        agent&apos;s settings, each entry is one JSON object on a line of its own:
      </P>
      <CodeBlock code={auditJson.files[0].code} file="src/main.kav" output={auditJson.output} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Every entry has <C>time</C> (UTC), <C>agent</C>, <C>request</C>, <C>call</C>, <C>shown</C>, <C>verdict</C> and <C>result</C>. A refused call is there too, with the rule it broke.</li>
        <li>An <C>ask</C> adds <C>model</C>, and in a real run <C>input_tokens</C> and <C>output_tokens</C>. A dry run, as here, has no token fields, and its verdict is <C>dry run</C>.</li>
        <li>The setting is per agent: an agent without it keeps the text format, in the same run.</li>
        <li><C>Text</C> is the default format.</li>
      </ul>
      <P>A log shipper (Vector, Fluent Bit, the OpenTelemetry Collector&apos;s file receiver) can carry the file on from there.</P>
      <H2>The audit log to a collector</H2>
      <P>
        With <C>otlp: true</C> in the agent&apos;s <C>Audit</C>, each entry also goes to the collector named in{" "}
        <C>kavach.toml</C>&apos;s <C>[log.otlp]</C>, the same one the log events go to. Where the collector is depends
        on where the program runs, so it stays in <C>kavach.toml</C>:
      </P>
      <CodeBlock code={'settings AgentSettings(audit: Audit(otlp: true))'} file="src/main.kav, in the agent" />
      <CodeBlock code={'[log.otlp]\nendpoint = "http://localhost:4318"'} file="kavach.toml" lang="toml" />
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <thead>
            <tr className="border-b border-line text-left text-soft">
              <th className="py-2 pr-4 font-bold">In the log record</th>
              <th className="py-2 font-bold">Holds</th>
            </tr>
          </thead>
          <tbody>
            {[
              ["event.name", "AgentAudit, so audit records are easy to pick out from your own events"],
              ["severity", "WARN for a refused call, INFO otherwise: an alert on WARN finds every refusal"],
              ["body", "the call as shown in the file, such as Payments.refund(order_id: 2, amount: 150)"],
              ["agent, request, call, verdict, result", "the JSON line's fields, as attributes"],
              ["model, input_tokens, output_tokens", "for an ask, so cost per agent is a query"],
              ["service.name", "service in [log.otlp], or the package's name"],
            ].map(([what, holds]) => (
              <tr key={what} className="border-b border-line align-top">
                <td className="py-2 pr-4 whitespace-nowrap"><C>{what}</C></td>
                <td className="py-2 text-muted">{holds}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Records go in batches, on the thread that sends the log events, so a slow collector never holds up an agent.</li>
        <li><strong className="text-ink">The file stays the record that counts.</strong> It is still written, and a collector that is down loses nothing from it.</li>
        <li>The setting is per agent, and <C>false</C> by default.</li>
        <li>Without a collector to send to, the program is refused before it runs:</li>
      </ul>
      <CodeBlock code={otlpWrong.files[0].code} file="src/main.kav" output={otlpWrong.output} outcome="refused" />
      <H2>For analytics</H2>
      <P>
        Because an event is a record, every field is a column you can query: refunds per day, the total refunded per
        customer, how often a limit stopped an agent, from <C>RefundBlocked</C>&apos;s <C>reason</C>. A secret field
        is written as <C>&lt;secret&gt;</C> wherever the event goes.
      </P>
      <Note>
        Keep the durable{" "}
        <Link href="/docs/agents/durable-agents/">journal</Link> out of your analytics: it is state, and holds secret
        values as they are.
      </Note>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/08-capabilities-and-agents/06-audit-log.md" label="spec 8.6 · audit log" />
        <SrcRef file="docs/spec/14-standard-library/12-log.md" label="spec 14.12 · std.log" />
        <SrcRef file="tools/cli/tests/log.rs" line={230} symbol="audit_entries_go_to_the_collector_too" label="the collector's test" />
      </div>
    </>
  ),
};
