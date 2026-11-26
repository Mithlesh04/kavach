import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import { Terminal } from "@/components/terminal";
import type { LessonDoc } from "@/lib/chapter";
import { guideProject } from "./guide-parts";
import { H2, Note, P } from "./parts";

// ---------- Guides › Build an Agent with Limits and Approvals ----------
//
// One task from start to finish: guides/refund-desk.project, an agent that
// refunds orders within limits, asks finance before a large refund, and
// keeps an audit log. A test bank stands in for the payment provider, and
// main answers the approval as a person would with `kavach approve`.

const desk = guideProject("refund-desk", ["src/main.kav", "src/main.test.kav", "kavach.toml"]);
const main = "src/main.kav";

export const guideAgentDoc: LessonDoc = {
  lead: "One task from start to finish: an agent that refunds orders. It may only refund, never more than a limit a person set, and it asks finance before a large refund. Every call it makes is in an audit log.",
  simple: <>An agent is a program that acts for you, here by refunding customers. You decide, in its code, what it may do and how much, and which actions wait for a person to say yes. It can&apos;t do anything else.</>,
  minutes: 14,
  before: ["agents/agents", "agents/your-own-capabilities"],
  body: (
    <>
      <P>
        The refunds go through a payment provider. In this guide a test bank stands in for it, so the code runs
        anywhere and touches no money; a real program gives the agent the real provider instead.
      </P>
      <H2>1. Say what may be done: a capability</H2>
      <P>
        A capability names actions in the world. Every action can be refused, so its error has a{" "}
        <C>Denied(reason)</C> case:
      </P>
      <CodeBlock code={desk.piece(main, "type PayError")} file={main} />
      <CodeBlock code={desk.piece(main, "capability Payments")} file={main} />
      <P>The test bank implements it, and only prints:</P>
      <CodeBlock code={desk.piece(main, "type TestBank")} file={main} />
      <H2>2. The agent and its rules</H2>
      <CodeBlock code={desk.piece(main, "agent Refunder")} file={main} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>can Payments.refund</C> is everything it may do. A call to anything else is a compile error, not a check that might be skipped.</li>
        <li><C>limit</C> sets the most for one refund, and for all refunds in a day. A refund past either is refused before the bank is called. See <Link href="/docs/agents/agents/limit/">limit</Link>.</li>
        <li><C>approve</C> makes a refund over 100 wait for someone in finance, for up to 2 days. See <Link href="/docs/agents/agents/approve/">approve</Link>.</li>
        <li><C>settings AgentSettings(audit: Audit())</C> writes every call, its verdict and its result to <C>agents/Refunder/audit.log</C>. See <Link href="/docs/agents/audit-log/">Audit Log</Link>.</li>
      </ul>
      <H2>3. Run it</H2>
      <P>
        <C>main</C> starts the agent with the test bank and asks for three refunds: a small one, one past the limit,
        and one that needs finance. It answers the approval in code, as a person in finance would with{" "}
        <C>kavach approve</C>, then prints the audit log:
      </P>
      <CodeBlock code={desk.piece(main, "fn main")} file={main} />
      <Terminal session={desk.said("check")} />
      <Terminal session={desk.said("run src/main.kav")} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>The refund of 40 went through at once: it is under the limit, and under 100.</li>
        <li>The refund of 900 was refused, and the bank never saw it.</li>
        <li>The refund of 300 waited, then went through once <C>asha</C> approved it for finance.</li>
        <li>The last line, which says how to answer the request, went to standard error, so the recorded session shows it last. In a terminal it comes when the request starts to wait.</li>
      </ul>
      <P>
        From another terminal, <C>kavach approvals</C> lists what waits, and{" "}
        <C>kavach approve Refunder-3-1 --as asha</C> or <C>kavach deny</C> answers it. Here nothing is left:
      </P>
      <Terminal session={desk.said("approvals")} />
      <H2>4. Test the rules</H2>
      <P>
        A test starts the agent with the test bank and checks that a refund past the limit is refused. A test runs with
        no capabilities unless it says so, so this one says <C>uses Payments</C>:
      </P>
      <CodeBlock code={desk.file("src/main.test.kav")} file="src/main.test.kav" />
      <Terminal session={desk.said("test --seed 7")} />
      <Note>
        The test bank is listed as weakly tested: it only prints, so there is nothing to check in it. The rules that
        matter are the agent&apos;s, and the test checks them.
      </Note>
      <H2>5. Before it touches real money</H2>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Run it with <C>kavach run --dry-run</C> first: every call is simulated and written down, and nothing reaches the bank. See <Link href="/docs/agents/agent-modes/dry-runs/">Dry runs</Link>.</li>
        <li>Give the agent the real provider in <C>Refunder.start(Payments: …)</C>. The rules don&apos;t change.</li>
        <li>An agent that must keep its counts and waiting approvals across a restart is a durable agent. See <Link href="/docs/agents/durable-agents/">Durable Agents</Link>.</li>
      </ul>
      <H2>The whole program</H2>
      <CodeBlock code={desk.file(main)} file={main} />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/08-capabilities-and-agents/README.md" label="spec 8 · capabilities and agents" />
        <SrcRef file="website/content/samples/guides/refund-desk.project/runs" label="the commands this page runs" />
      </div>
    </>
  ),
};
