import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import { Terminal } from "@/components/terminal";
import type { LessonDoc } from "@/lib/chapter";
import { loadProject } from "@/lib/samples";
import { Ex, H2, P } from "./parts";

// ---------- Approvals ----------

const approveWait = loadProject("agents/approve-deadline", ["src/main.kav"]);
const approveRestart = loadProject("agents/approve-restart", ["src/main.kav"]);
const approveJoin = loadProject("agents/approve-join", ["src/main.kav"]);

export const approveDoc: LessonDoc = {
  lead: "approve … when … by \"role\" makes the handler wait for a person in that role to say yes.",
  simple: <>Like a shop where a cashier can give small refunds alone but needs a manager to agree to big ones. When the condition is true, the agent waits until a person in the named role says yes.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="agents/sub-approve" />
      <P>
        Small refunds go through. Above 100, the handler pauses and asks a person in the <C>finance</C> role, at the
        terminal. This program runs outside a project and no one is at a terminal, so no one can be asked: the call is
        not approved, the safe answer.
      </P>
      <P>The condition can test any parameter, text included:</P>
      <Ex name="agents/uc-deploy" />
      <H2>Waiting for an answer</H2>
      <P>
        In a project, the agent has a folder, and a request with no terminal waits there for an answer.{" "}
        <C>within</C> says how long; without it, 7 days. Here the request may wait 1 second:
      </P>
      <CodeBlock code={approveWait.files[0].code} file="src/main.kav" />
      <Terminal session={approveWait.output ?? ""} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>
          The <C>approval needed</C> line goes to standard error, while the request waits. It is shown last here, but
          the program printed it first.
        </li>
        <li>
          Each request has a number: the agent, the event&apos;s number and the call&apos;s place in it.{" "}
          <C>Refunder-1-1</C> is the first call of event 1, and the number stays the same across restarts.
        </li>
        <li>
          <C>kavach approvals</C>, in the project, lists the requests that wait: their number, the call, the role,
          and when each was asked and runs out, in UTC.
        </li>
        <li>
          <C>kavach approve Refunder-1-1 --as asha</C> approves one, and{" "}
          <C>kavach deny Refunder-1-1 --as asha --reason &quot;duplicate&quot;</C> refuses it. <C>--as</C> is
          needed, because the answer and who gave it go in the audit log.
        </li>
        <li>A running program sees an answer within a second.</li>
      </ul>
      <P>
        In a durable handler the request is kept in the journal, so the program may end while it waits: killed, by{" "}
        <C>cli.exit</C> or by a panic. Here it ends, the request is approved while it is down, and the next start
        finishes the event:
      </P>
      <CodeBlock code={approveRestart.files[0].code} file="src/main.kav" />
      <Terminal session={approveRestart.output ?? ""} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>The refund waits in its own task, so <C>main</C> goes on, and ends the program.</li>
        <li>An event ends only when its handler returns, with a value or an error. This one never did, so the journal keeps it.</li>
        <li>At the next start the answer is there, so the event doesn&apos;t wait: the refund is paid, once, before <C>start</C> returns.</li>
        <li>A request can be answered once. Answering again, like a request that ran out of time, says it doesn&apos;t wait.</li>
      </ul>
      <P>
        If no one has answered yet, <C>start</C> doesn&apos;t wait for a person: it returns, and the event goes on
        inside the agent. Here the second run starts while the refund still waits, sends the same refund again, and
        answers it in code:
      </P>
      <CodeBlock code={approveJoin.files[0].code} file="src/main.kav" />
      <Terminal session={approveJoin.output ?? ""} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>In the second run, <C>started</C> comes before anyone answers: <C>start</C> returned while the event waited.</li>
        <li>
          Sending the same event again (the same handler, with equal arguments) joins it. There is still one request,{" "}
          <C>Refunder-1-1</C>, and the refund is paid once.
        </li>
        <li>
          The first run&apos;s sender is gone, so how the event ended goes in the{" "}
          <Link href="/docs/agents/audit-log/">audit log</Link>, with its number. The program that sent it again gets
          the outcome too. If the handler panics, its send fails with <C>panicked: &lt;message&gt; at
          &lt;file&gt;:&lt;line&gt;</C>, and the log line ends with <C>error panicked: …</C>.
        </li>
        <li>
          The event goes on like a task spawned where <C>start</C> was called. If that function returns first, the
          event is cancelled and stays unfinished in the journal, for the next start.
        </li>
      </ul>
      <P>
        How a request ended goes in the audit log, and a refusal&apos;s reason in its <C>Denied</C> error. For{" "}
        <C>when amount &gt; 10 by &quot;finance&quot;</C>:
      </P>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Approved: <C>approved by asha for finance (amount &gt; 10)</C></li>
        <li>Denied with a reason: <C>not approved by finance (amount &gt; 10): duplicate</C></li>
        <li>Out of time: <C>not approved by finance (amount &gt; 10) within 1 second</C></li>
      </ul>
      <P>
        A program can answer requests too, from a web page, a chat or an email: see{" "}
        <Link href="/docs/standard-library/agents/">std.agents</Link>.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/08-capabilities-and-agents/03-agents.md" label="spec 8.3 · approvals" />
        <SrcRef file="docs/decisions/0072-approvals-that-wait-across-restarts.md" label="decision 0072" />
        <SrcRef file="tools/cli/tests/approvals.rs" label="approve, deny, deadline, restart and joining, tested" />
      </div>
    </>
  ),
};
