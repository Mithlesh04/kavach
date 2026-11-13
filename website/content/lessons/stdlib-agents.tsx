import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import { Terminal } from "@/components/terminal";
import type { LessonDoc } from "@/lib/chapter";
import { loadProject } from "@/lib/samples";
import { Ex, H2, P } from "./parts";
import { ModuleTable } from "./spec-table";

// ---------- Agents ----------

const answered = loadProject("agents/answer-in-code", ["src/main.kav"]);

export const agentsStdDoc: LessonDoc = {
  lead: "std.agents gives a capability's implementation the key of the call it is running, so a service can drop a repeated call, and lets a program answer agents' approval requests.",
  simple: <>Like an order number on a form: if the form is sent twice by mistake, the shop sees the same number and fills the order once. And like a manager&apos;s inbox: a program can read the requests that wait for a yes, and pass on a person&apos;s answer.</>,
  minutes: 4,
  before: ["agents/durable-agents"],
  body: (
    <>
      <ModuleTable file="16-agents.md" number="14.16" first="Function" />
      <P>
        A call that a crash cut off runs again after the restart, so it may run twice. A service that is told the key
        can drop the repeat, as Stripe and other payment APIs do with an <C>Idempotency-Key</C> header:
      </P>
      <CodeBlock code={'use std.agents\n\ntype Bank: Payments {\n    fn refund(self, order_id: Int) -> Receipt ! PaymentError {\n        // The bank keeps the keys it has seen and drops a second refund.\n        const key = agents.call_key() ?? "order-{order_id}"\n        try self.api.refund(order_id, key)\n    }\n}'} file="an implementation, from spec 14.16" />
      <P>
        Like reading the clock, <C>call_key</C> needs no capability. A whole run, crash and all, is on{" "}
        <Link href="/docs/agents/durable-agents/">Durable Agents</Link>.
      </P>
      <H2>Answering approvals in code</H2>
      <P>
        <C>waiting</C> and <C>answer</C> let a program take a person&apos;s answer from a web page, a chat or an
        email, and pass it on, as <Link href="/docs/agents/agents/approve/">kavach approve</Link> does. Here the
        program asks for a refund in one task, then answers the request for <C>asha</C>:
      </P>
      <CodeBlock code={answered.files[0].code} file="src/main.kav" />
      <Terminal session={answered.output ?? ""} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>agents.waiting()</C> gives the requests that wait in the project&apos;s agents folder, oldest first.</li>
        <li>The <C>approval needed</C> line goes to standard error, so it is shown last here.</li>
        <li>
          <C>by</C> names the person who answers, and goes to the audit log. Who may answer as <C>finance</C> is the
          program&apos;s to check, with its own sign-in.
        </li>
        <li>
          Answering again fails with <C>NotWaiting</C>: the request was answered. A request never made, or out of
          time, fails the same way. <C>Failed(id, reason)</C> is an answer that can&apos;t be written.
        </li>
        <li>
          The program that answers is usually another project, such as a chat bot. Its <C>kavach.toml</C> points at
          the worker&apos;s folder, with <C>folder = &quot;../agents&quot;</C> under <C>[agents]</C>.
        </li>
        <li>Under <C>kavach test</C> and in a dry run, agents keep nothing, so <C>waiting()</C> is empty and <C>answer</C> fails with <C>NotWaiting</C>.</li>
      </ul>
      <P>Each request is an <C>ApprovalRequest</C>, with these fields, all <C>String</C>:</P>
      <ModuleTable file="16-agents.md" number="14.16" first="Field" />
      <P>
        Both functions need the capability <C>Approvals</C>. An agent may never list it in its <C>can</C>, so an
        agent can&apos;t approve its own requests:
      </P>
      <Ex name="agents/approvals-in-can.fails" />
      <P>A function that lists its capabilities must list <C>Approvals</C> too:</P>
      <Ex name="agents/approvals-uses.fails" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/14-standard-library/16-agents.md" label="spec 14.16 · std.agents" />
        <SrcRef file="tools/cli/tests/approvals/in_code.rs" label="answering in code, tested" />
      </div>
    </>
  ),
};
