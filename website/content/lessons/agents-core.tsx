import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { RollingWindow } from "@/components/rolling-window";
import { SrcRef } from "@/components/src-ref";
import { Terminal } from "@/components/terminal";
import type { LessonDoc } from "@/lib/chapter";
import { rustConst, rustExpected } from "@/lib/rust-tests";
import { loadProject, loadSample } from "@/lib/samples";
import { specTables } from "@/lib/spec-tables";
import { AsksUnderWay } from "./agents-under-way";
import { Ex, H2, Note, P, Syntax } from "./parts";

const AI_TESTS = "tools/cli/tests/ai.rs";

const handlerPanic = loadProject("agents/handler-panic", ["src/main.kav"]);

/** A spec cell with its `code` spans shown as code. */
const cell = (text: string) => text.split("`").map((part, i) => (i % 2 ? <C key={i}>{part}</C> : part));

/** Spec 8.3's table of limit rules, as the spec writes it. */
function LimitRules() {
  const t = specTables("08-capabilities-and-agents/03-agents.md", "8.3").find((x) => x.header.join("|") === "Rule|Meaning");
  if (!t) throw new Error("spec 8.3: no Rule|Meaning table");
  return (
    <div className="overflow-x-auto">
      <table className="w-full border-collapse text-[15px]">
        <thead>
          <tr className="border-b border-line text-left text-soft">
            {t.header.map((h) => (
              <th key={h} className="py-2 pr-4 font-bold">{h}</th>
            ))}
          </tr>
        </thead>
        <tbody>
          {t.rows.map(([rule, meaning]) => (
            <tr key={rule} className="border-b border-line align-top">
              <td className="py-2 pr-4 whitespace-nowrap">{cell(rule)}</td>
              <td className="py-2 text-muted">{cell(meaning)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

// The window lengths of runtime/core/src/agents.rs (`fn window`).
const windows: [string, string, number][] = [
  ["minute", "60 seconds", 60],
  ["hour", "60 minutes", 3_600],
  ["day", "24 hours", 86_400],
  ["week", "7 days", 604_800],
  ["month", "30 days", 2_592_000],
];

// The calls of agents/sub-limit, in order; what happened to each is
// read from the sample's checked output, so the table can't drift.
const limitCalls: [order: number, amount: number, check: string][] = [
  [1, 400, "400 ≤ 500, and the day's total 0 + 400 = 400 ≤ 700"],
  [2, 900, "900 > 500: the rule amount <= 500 is broken"],
  [3, 250, "250 ≤ 500, and 400 + 250 = 650 ≤ 700"],
  [4, 300, "300 ≤ 500, but 650 + 300 = 950 > 700"],
];

function LimitTrace() {
  const lines = (loadSample("agents/sub-limit").output ?? "").split("\n");
  if (lines.length !== limitCalls.length) throw new Error("agents/sub-limit: the limit table needs one output line per call");
  return (
    <div className="overflow-x-auto">
      <table className="w-full border-collapse text-[15px]">
        <thead>
          <tr className="border-b border-line text-left text-soft">
            <th className="py-2 pr-4 font-bold">Call</th>
            <th className="py-2 pr-4 font-bold">The checks</th>
            <th className="py-2 font-bold">Printed</th>
          </tr>
        </thead>
        <tbody>
          {limitCalls.map(([order, amount, check], i) => (
            <tr key={order} className="border-b border-line align-top">
              <td className="py-2 pr-4 whitespace-nowrap"><C>{`refund(${order}, ${amount})`}</C></td>
              <td className="py-2 pr-4 text-muted">{check}</td>
              <td className="py-2"><C>{lines[i]}</C></td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

// ---------- Agents ----------

export const agentsDoc: LessonDoc = {
  lead: "An agent is a program that reacts to events and acts on the world. Its declaration says what it may touch, how much, and when a person decides; the compiler and the runtime hold it to that.",
  simple: <>An agent is like a new employee with a written job description: what they may touch, how much they may do, and when they must ask a manager. The system holds them to it; nobody relies on their good judgement.</>,
  minutes: 6,
  before: ["agents/your-own-capabilities"],
  body: (
    <>
      <Syntax
        code={
          'agent Name {\n    model careful()\n    can Capability.method, Ai\n\n    limit Capability.method {\n        amount <= 500\n        count <= 100 per day\n    }\n\n    approve Capability.method when amount > 100 by "role"\n\n    on event(param: Type) ! Error {\n        …\n    }\n}'
        }
      />
      <Ex
        name="agents/greeter"
        notes={[
          { target: "can Email.send", chip: "ALLOWED", kind: "new", what: "The only thing this agent's handlers may touch.", why: "anything else refuses to compile." },
          { target: "count <= 2 per day", chip: "HARD LIMIT", kind: "why", what: "Checked by the runtime on every call.", why: "the third send is refused with a reason, not by luck." },
          { target: "on welcome", chip: "EVENT", kind: "new", what: "An event the agent handles; it becomes a method, bot.welcome(…).", why: "every way into the agent is listed in one place." },
        ]}
      />
      <P>
        Two emails go out; the third is <strong className="text-ink">refused by the limit</strong>, as an ordinary{" "}
        <C>Denied</C> error the caller handles. No prompt asked the agent to behave: the boundary is in the code.
      </P>
      <H2>What an agent declares</H2>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <thead>
            <tr className="border-b border-line text-left text-soft">
              <th className="py-2 pr-4 font-bold">Part</th>
              <th className="py-2 pr-4 font-bold">Says</th>
              <th className="py-2 font-bold">Checked</th>
            </tr>
          </thead>
          <tbody>
            {[
              ["model", "which AI model its ask calls use", "when compiling"],
              ["can", "what the handlers may touch", "when compiling"],
              ["limit", "how much: per call, and totals and counts over a rolling minute, hour, day, week or 30-day month", "on every call"],
              ["approve … by … within", "when a person in a named role must say yes first, and how long to wait", "on every call"],
              ["on", "the events the agent handles", "when compiling"],
            ].map(([part, says, when]) => (
              <tr key={part} className="border-b border-line align-top">
                <td className="py-2 pr-4 whitespace-nowrap"><C>{part}</C></td>
                <td className="py-2 pr-4 text-muted">{says}</td>
                <td className="py-2 text-muted">{when}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <Note>
        An AI model inside an agent can be confused, tricked or just wrong. None of that moves the boundary: the
        compiler checks <C>can</C>, and the runtime checks <C>limit</C> and <C>approve</C>, outside anything the
        model can influence.
      </Note>
    </>
  ),
  tryIt: {
    task: <>Change the limit so the agent may send 5 emails a day, and add a fourth call to <C>bot.welcome</C>. What prints?</>,
    answer: <P>With <C>count &lt;= 5 per day</C>, all four sends go through: four <C>sent to …</C> lines, no refusal.</P>,
  },
  deeper: (
    <>
      <P>
        Limit totals live in the agent&apos;s state, so they can last across restarts (see{" "}
        <Link href="/docs/agents/durable-agents/limits-across-restarts/">Limits across restarts</Link>). Every call,
        refused or not, is written to the <Link href="/docs/agents/audit-log/">audit log</Link>.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/08-capabilities-and-agents/03-agents.md" label="spec 8.3 · agents" />
        <SrcRef file="examples/05-refund-bot/src/main.kav" label="a full agent: the refund bot example" />
      </div>
    </>
  ),
  quiz: [
    {
      type: "choice",
      prompt: <>The AI inside an agent is tricked into trying a bank transfer the agent has no <C>can</C> for. What stops it?</>,
      options: [
        "The compiler: a handler that could make that call never compiles",
        "A system prompt asking the model not to",
        "A code review later",
      ],
      answer: 0,
      explain: "can is checked when compiling. A call outside it can't be in the program at all.",
    },
  ],
};

export const declareAgentDoc: LessonDoc = {
  lead: "agent Name { … } declares an agent: its model, its can, its limits and approvals, and its on handlers.",
  simple: <>Declaring an agent is like filling in its job card: what it may use, which AI model it works with, its limits, who must approve what, and the events it answers to, all written in one place.</>,
  minutes: 2,
  body: (
    <>
      <Ex name="agents/greeter" />
      <P>
        The agent uses a capability, here <C>Email</C>, and gets an implementation of it when it starts (see{" "}
        <Link href="/docs/agents/running-an-agent/">Running an Agent</Link>).
      </P>
      <P>
        Like any declaration, an agent is private to its module. Write <C>pub agent</C> and other modules can start it
        and call its handlers: <C>bots.Picker.start()</C>. See{" "}
        <Link href="/docs/tools/modules/pub/">Modules › pub</Link>.
      </P>
    </>
  ),
};

export const canDoc: LessonDoc = {
  lead: "can lists what the agent's handlers may touch. A handler that needs anything else is a compile error.",
  simple: <><C>can</C> is like the list of rooms a building pass opens. If a handler tries to enter a room that is not on the list, the program is refused before it ever runs.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="agents/sub-can.fails" />
      <P>Built-in capabilities count too. A handler that prints needs <C>Console</C> in <C>can</C>:</P>
      <Ex name="agents/agent-console.fails" />
      <Note>The compiler works out what each handler uses, including through the functions it calls, so nothing slips past.</Note>
    </>
  ),
};

export const limitDoc: LessonDoc = {
  lead: "limit sets hard boundaries on a capability method: on each call, on totals, and on the number of calls in a window.",
  simple: <>A limit works like the caps on a bank card: a cap for each purchase, a cap on the total, and how many purchases are allowed in a period of time. A call past a cap is turned down and never runs.</>,
  minutes: 4,
  body: (
    <>
      <Ex name="agents/sub-limit" />
      <P>The kinds of rule, as spec 8.3 lists them:</P>
      <LimitRules />
      <P>The method&apos;s parameters are in scope by name: <C>amount</C> is the <C>amount</C> of each call. Call by call:</P>
      <LimitTrace />
      <Note>A refused call never runs, so it doesn&apos;t count toward the totals: after order 2, the day&apos;s total is still 400.</Note>
      <H2>How long each window is</H2>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <thead>
            <tr className="border-b border-line text-left text-soft">
              <th className="py-2 pr-4 font-bold">Written</th>
              <th className="py-2 pr-4 font-bold">Means the last</th>
              <th className="py-2 font-bold">In seconds</th>
            </tr>
          </thead>
          <tbody>
            {windows.map(([w, means, secs]) => (
              <tr key={w} className="border-b border-line">
                <td className="py-2 pr-4 whitespace-nowrap"><C>{`per ${w}`}</C></td>
                <td className="py-2 pr-4 text-muted">{means}</td>
                <td className="py-2"><C>{secs.toLocaleString("en-US")}</C></td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <P>Move &quot;now&quot; along two days and watch which calls count:</P>
      <RollingWindow />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><strong className="text-ink">A rolling window, not the calendar.</strong> <C>per day</C> counts the calls of the last 24 hours before each new call, not the calls since midnight.</li>
        <li><strong className="text-ink">A month is always 30 days.</strong> February, a 31-day month or a leap year don&apos;t change it. For calendar months, with their real lengths, use <Link href="/docs/standard-library/time/">dates</Link>: <C>add_months</C>.</li>
        <li><strong className="text-ink">No time zones, no daylight saving.</strong> Time is measured on a clock that never jumps, so a day is always exactly 24 hours, in every country.</li>
        <li><strong className="text-ink">Across restarts</strong>, with a <Link href="/docs/agents/durable-agents/limits-across-restarts/">state file</Link>, each call&apos;s time is saved in UTC, so the window carries on where it was.</li>
      </ul>
      <H2>A token budget for AI calls</H2>
      <P>
        In <C>limit Ai</C>, <C>tokens</C> counts the tokens, in and out, that the window&apos;s earlier <C>ask</C>s
        took. Once they pass the limit, the next <C>ask</C> is refused and never sent. Here each answer takes 60 tokens
        in and 10 out, against a budget of 100 a day:
      </P>
      <CodeBlock code={rustConst(AI_TESTS, "BUDGET")} file="main.kav, run against a test model" output={rustExpected(AI_TESTS, "a_token_budget_refuses_asks_past_it")} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>The first call starts at 0 tokens, the second at 70: both are under 100, so both run.</li>
        <li>The second takes the total to 140. The third is refused with <C>Denied</C> before it is sent, so it costs nothing.</li>
        <li><strong className="text-ink">The call that crosses the limit still runs</strong>, because an answer&apos;s size is known only after it. So the total can end above the budget by one call&apos;s tokens, as it does here.</li>
        <li>The tokens counted are the ones in the audit log&apos;s <C>model:</C> line, and with a state file they are kept across restarts, like <C>sum</C> and <C>count</C>.</li>
      </ul>
      <P>Tokens are counted only for AI calls, so on any other capability <C>tokens</C> is a compile error:</P>
      <Ex name="agents/limit-tokens.fails" />
      <AsksUnderWay />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/08-capabilities-and-agents/03-agents.md" label="spec 8.3 · limit windows" />
        <SrcRef file="runtime/core/src/agents.rs" line={108} symbol="window" label="the window lengths, in the runtime" />
        <SrcRef file="tools/cli/tests/ai.rs" line={401} symbol="a_token_budget_refuses_asks_past_it" label="the token budget's test" />
      </div>
    </>
  ),
};

export const checksOrderDoc: LessonDoc = {
  lead: "Every capability call goes through the same checks, in the same order: can, dry run, limits, approval, then the call.",
  simple: <>Like security at an airport, every call passes the same checkpoints in the same order. Limits come before approval, so nobody is asked to approve something that would be refused anyway.</>,
  minutes: 3,
  body: (
    <>
      <ol className="m-0 flex list-decimal flex-col gap-1.5 pl-5">
        <li><C>can</C>, already checked when compiling: a call the agent may not make is not in the program.</li>
        <li><C>--dry-run</C>: the call is not made, and gets a made-up answer (see <Link href="/docs/agents/agent-modes/dry-runs/">Dry runs</Link>).</li>
        <li>Limits: a broken rule refuses the call.</li>
        <li>Approval: if the condition is true, a person decides.</li>
        <li>The call runs, and counts toward the totals.</li>
      </ol>
      <P>
        Limits come before approval, so no one is asked to approve a call that would be refused anyway. Here 900
        needs approval and breaks the limit, and nobody is asked:
      </P>
      <Ex name="agents/agent-order" />
    </>
  ),
};

export const onDoc: LessonDoc = {
  lead: "on name(params) { … } is an event the agent handles. An agent can have as many as it needs.",
  simple: <>Each <C>on</C> is like a button on a machine: it names one thing the agent can be asked to do, and says what happens when it is pressed. An agent can have as many buttons as it needs.</>,
  minutes: 2,
  body: (
    <>
      <Ex name="agents/agent-events" />
      <P>
        Each <C>on</C> becomes a method of the running agent, with the handler&apos;s parameters, giving the
        handler&apos;s value. It can always fail with an <C>Error</C>:{" "}
        <Link href="/docs/agents/running-an-agent/">see Running an Agent</Link>.
      </P>
    </>
  ),
};

// ---------- Running an Agent ----------

export const runningDoc: LessonDoc = {
  lead: "Start an agent with Name.start(…), giving one implementation for each capability it can use. Each on handler is then a method you call, and that call can always fail.",
  simple: <>Starting an agent is like a worker&apos;s first day: the job description says &quot;may send email&quot;, and now you hand over the actual mailbox to use. After that, you give it work by calling its handlers.</>,
  minutes: 5,
  before: ["agents/agents"],
  body: (
    <>
      <Ex name="agents/agent-events" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>Concierge.start(Email: Outbox())</C> names each implementation by its capability.</li>
        <li>Built-in capabilities, like <C>Ai</C> or <C>Console</C>, need none.</li>
        <li>While a handler runs, its capability calls, and those of every function it calls, go to these implementations, checked against the agent&apos;s limits.</li>
      </ul>
      <P>A missing implementation, an extra one, or a value of the wrong type is a compile error:</P>
      <Ex name="agents/run-missing.fails" />
      <H2>Every send can fail</H2>
      <P>
        Calling a handler sends the agent an event. The send can fail with an <C>Error</C>, whatever the handler
        declares: the handler&apos;s own error, a refused call, or a panic in the handler. So every send needs{" "}
        <C>try</C>, <C>catch</C> or <C>match</C>, even for a handler with no <C>!</C>:
      </P>
      <Ex name="agents/send-plain.fails" />
      <P>A handler that panics stops there, and the agent goes on serving other events:</P>
      <CodeBlock code={handlerPanic.files[0].code} file="src/main.kav" />
      <Terminal session={handlerPanic.output ?? ""} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>
          The send fails with <C>panicked: &lt;message&gt; at &lt;file&gt;:&lt;line&gt;</C>, as <C>.wait()</C> does for{" "}
          <Link href="/docs/errors/panics/">a task that panicked</Link>.
        </li>
        <li>
          The panic is printed to standard error when it happens, and written to the{" "}
          <Link href="/docs/agents/audit-log/">audit log</Link>.
        </li>
        <li>
          In a durable agent the event stays unfinished in its journal, so it{" "}
          <Link href="/docs/agents/durable-agents/">runs again</Link> at the next start.
        </li>
        <li>The code that sent the event goes on with all its own values as they were, however many sends fail.</li>
      </ul>
      <Note>
        For now an agent is started from your own code, as here. How events reach it in production (web requests,
        queues, schedules) is still being designed.
      </Note>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/08-capabilities-and-agents/08-running-an-agent.md" label="spec 8.8 · running an agent" />
        <SrcRef file="docs/decisions/0076-every-send-to-an-agent-can-fail.md" label="decision 0076" />
        <SrcRef file="tests/conformance/08-capabilities/a-handler-that-panics-fails-its-send.kav" label="a handler that panics, tested" />
        <SrcRef file="tests/conformance/08-capabilities/a-caught-panic-leaves-the-senders-values.kav" label="the sender's values, tested" />
      </div>
    </>
  ),
};
