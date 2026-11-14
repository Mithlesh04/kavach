import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { loadProject } from "@/lib/samples";
import { specBlockWith } from "@/lib/spec-tables";
import { Limits } from "./limits";
import { Ex, H2, P } from "./parts";
import { ModuleTable } from "./spec-table";

const LOG = "12-log.md";
const LOG_TEST_LINE = 301;

// ---------- Logging ----------

const levels = loadProject("logging/levels", ["kavach.toml", "src/main.kav"]);

export const loggingDoc: LessonDoc = {
  lead: "A log event is a record you define, not a string. log.info, warn, error and debug write it, and the program's owner decides where it goes.",
  simple: <>A log is like a ship&apos;s logbook: the program writes down what happened as it runs. Each entry is a form with the same boxes every time, so entries are easy to search and count later.</>,
  minutes: 5,
  before: ["types/records"],
  body: (
    <>
      <Ex name="agents/sub-log" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>type OrderPlaced: LogEvent</C> makes the record an event. Only events can be logged: <C>log.info(&quot;order placed&quot;)</C> is a compile error (below).</li>
        <li>Each event is one line: the level, then the event as it prints. Every entry of a kind has the same fields, so logs can be searched and counted.</li>
        <li>A <C>secret</C> field is written as <C>&lt;secret&gt;</C>.</li>
      </ul>
      <P>Logging plain text out of habit is refused, and the message shows the record to declare:</P>
      <Ex name="habits/log-text.fails" />
      <H2>Levels</H2>
      <ModuleTable file="12-log.md" number="14.12" first="Function" />
      <P>Events below the lowest level written are left out. <C>kavach.toml</C> sets it; here only warnings and errors are written:</P>
      <CodeBlock code={levels.files[0].code} file="kavach.toml" lang="toml" />
      <CodeBlock code={levels.files[1].code} file="src/main.kav" output={levels.output} />
      <P>The same settings send lines to a file, which starts a new one at a size you choose, or to an OpenTelemetry collector. See <Link href="/docs/tools/settings/">Settings</Link>.</P>
      <H2>No capability needed</H2>
      <Ex name="logging/no-uses" />
      <P>
        <C>refund</C> is <C>pub</C> and logs, yet has no <C>uses</C>: where events go is the owner&apos;s choice, not
        the code&apos;s, and the code can&apos;t read them back.
      </P>
      <H2>Traces</H2>
      <P>
        A trace follows one piece of work, such as a request, through the program and the services it calls. It is
        made of spans: <C>log.span(name) {"{ … }"}</C> runs its block as one span, timed from its start to its end. The
        block is a plain block: see <Link href="/docs/basics/blocks-given-to-a-call/">Blocks Given to a Call</Link>.
      </P>
      <Ex name="logging/span" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>log.trace_id()</C> is the trace&apos;s id, 32 hexadecimal digits, or <C>none</C> outside a span: for an error page or a header in a reply.</li>
        <li><C>log.traceparent()</C> is the span&apos;s W3C <C>traceparent</C> header, and <C>from:</C> continues the trace one names. Together they carry a trace over any connection: a queue, gRPC metadata, a file.</li>
        <li>A span inside another is its child. A task keeps the trace of the code that spawned it.</li>
        <li>Each event written inside a span carries the span&apos;s trace and span ids. A span whose block fails is marked as an error.</li>
        <li>HTTP joins traces with no code: <C>http.serve</C> runs each handler in a span that continues the caller&apos;s trace, and <C>http.request</C> sends it. The span is named by the method (<C>GET</C>), never the path, which can hold ids and tokens. See <Link href="/docs/networks/http/serving/">Serving</Link>.</li>
        <li><C>log.name_span(name)</C> renames the current span, once the code knows what it is doing; outside a span it does nothing. Give it one of a few names, such as a route&apos;s pattern (<C>{'"GET /orders/\\{id}"'}</C>), not text with ids in it.</li>
        <li>With <C>[log.otlp]</C> set, each span goes to the collector when it ends. Without it, spans are sent nowhere.</li>
      </ul>
      <H2>Metrics</H2>
      <P>
        Numbers a program counts or measures go to the collector every 10 seconds as OpenTelemetry metrics, when{" "}
        <C>[log.otlp]</C> is set. Without it, they are kept nowhere and cost almost nothing:
      </P>
      <CodeBlock code={specBlockWith(`14-standard-library/${LOG}`, "14.12", "log.count(")} file="from spec 14.12" />
      <ModuleTable file="12-log.md" number="14.12" first="Function" heading="Metrics" />
      <P>
        Each also takes <C>attributes</C>, a <C>[String: String]</C> map that is empty unless given. Each name and set
        of attributes is a series of its own, whatever order the attributes are written in:
      </P>
      <Ex name="logging/metric-attributes" />
      <P>
        Give attributes with few values, such as a route&apos;s pattern, a method or a status, never ids: each new
        value makes a new series to keep and send. Like events, spans and metrics need no capability.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={`docs/spec/14-standard-library/${LOG}`} label="spec 14.12 · traces and metrics" />
        <SrcRef file="tests/conformance/14-standard-library/spans-carry-the-trace.kav" label="spans, tested" />
        <SrcRef file="tools/cli/tests/log.rs" symbol="spans_and_metrics_go_to_the_collector" line={LOG_TEST_LINE} label="sent to a collector, tested" />
        <SrcRef file="tools/cli/tests/log.rs" symbol="server_spans_are_named_by_method_not_path" line={375} label="named by method, tested" />
        <SrcRef file="docs/decisions/0107-traces-and-metrics.md" label="decision 0107" />
      </div>
      <H2>Limits and sizes</H2>
      <Limits id="logging" />
    </>
  ),
};
