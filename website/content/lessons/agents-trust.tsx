import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import { Terminal } from "@/components/terminal";
import type { LessonDoc } from "@/lib/chapter";
import { loadProject, loadSample } from "@/lib/samples";
import { Ex, H2, Note, P } from "./parts";

const SPEC = "docs/spec/08-capabilities-and-agents/06-audit-log.md";

/** A project's main file and what it printed. */
const main = (name: string) => {
  const p = loadProject(name, ["src/main.kav"]);
  return { code: p.files[0].code, output: p.output };
};

const enforce = main("agents/audit");
const observe = main("agents/observe");
// Not a project: there a call with no terminal waits for an answer, and here it is refused at once.
const suggest = loadSample("agents/suggest");
const secretAudit = loadProject("agents/audit-secret", ["src/main.kav"]);
const restartOutcome = main("agents/restart-outcome");

/** The head of a sample's agent: its first line and its settings, the rest left out. */
const agentHead = (code: string) => {
  const lines = code.split("\n");
  const start = lines.findIndex((l) => l.startsWith("agent "));
  const settings = lines.findIndex((l, i) => i > start && l.trim().startsWith("settings "));
  if (start < 0 || settings < 0) throw new Error("agentHead: no agent with settings");
  return [...lines.slice(start, settings + 1), "    …", "}"].join("\n");
};

/** One program's agent settings, with what the shared program printed under them. */
function ModeRun({ run }: { run: { code: string; output?: string } }) {
  return <CodeBlock code={agentHead(run.code)} file="the agent" output={run.output} />;
}

// ---------- Agent Modes ----------

export const modesDoc: LessonDoc = {
  lead: "Each agent runs in a mode, set in its settings: Observe only reports, Suggest asks a person every time, Enforce acts. Trust is earned step by step.",
  simple: <>Modes let you trust a new agent step by step, like a new employee: at first you only note what its rules would have stopped, then a person approves every action, and finally it works on its own within its rules.</>,
  minutes: 5,
  before: ["agents/agents"],
  body: (
    <>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <thead>
            <tr className="border-b border-line text-left text-soft">
              <th className="py-2 pr-4 font-bold">Mode</th>
              <th className="py-2 pr-4 font-bold">Capability calls</th>
              <th className="py-2 font-bold">Limits and approvals</th>
            </tr>
          </thead>
          <tbody>
            {[
              ["Observe", "run normally", "only reported: \"would be refused\""],
              ["Suggest", "each one waits for a person to approve it", "shown with each request"],
              ["Enforce (the default)", "run when allowed", "enforced: a broken limit refuses, an approval waits"],
            ].map(([mode, calls, rules]) => (
              <tr key={mode} className="border-b border-line align-top">
                <td className="py-2 pr-4 whitespace-nowrap"><C>{mode}</C></td>
                <td className="py-2 pr-4 text-muted">{calls}</td>
                <td className="py-2 text-muted">{rules}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <P>One program, a refund agent with <C>amount &lt;= 500</C>, asked for 400 and then 900. It prints its audit log at the end:</P>
      <CodeBlock code={enforce.code} file="src/main.kav" />
      <P>The same program with <C>mode: AgentMode.Observe</C> in its settings: both refunds run, and the log says which one the rules would have refused:</P>
      <ModeRun run={observe} />
      <Note>
        A company can run a new agent beside the old process in <C>Observe</C> for a week, read what it would have
        blocked, then switch to <C>Suggest</C> or <C>Enforce</C>: one word in the agent&apos;s{" "}
        <Link href="/docs/agents/agents/settings/">settings</Link>, seen in review. The mode is worked out when the
        program is built, so an executable keeps the mode it was built with.
      </Note>
    </>
  ),
  quiz: [
    {
      type: "choice",
      prompt: <>In <C>observe</C> mode, a call breaks a limit. What happens?</>,
      options: ["It runs, and the audit log says it would be refused", "It is refused, as in enforce", "The program stops"],
      answer: 0,
      explain: "observe only reports. It is how an agent earns trust before its rules act.",
    },
  ],
};

export const observeDoc: LessonDoc = {
  lead: "observe: calls run normally, and limits and approvals are only reported.",
  simple: <>In observe mode the agent works as usual, and its rules only take notes, like a referee who writes down fouls without stopping the game. The notes show what the rules would have stopped.</>,
  minutes: 2,
  body: (
    <>
      <ModeRun run={observe} />
      <P>The 900 refund ran. Its verdict, <C>would be refused</C>, is the evidence for deciding when to enforce.</P>
    </>
  ),
};

export const suggestDoc: LessonDoc = {
  lead: "suggest: every capability call waits for a person to approve it.",
  simple: <>In suggest mode the agent is like a trainee who must ask before every action: nothing happens until a person says yes. A call that breaks a limit is refused without asking anyone.</>,
  minutes: 2,
  body: (
    <>
      <ModeRun run={suggest} />
      <P>
        Each call asks a person. This program runs outside a project, with no terminal, so no one can be asked and
        the 400 refund is refused at once. The 900 refund breaks the limit, so it is refused before anyone would be
        asked.
      </P>
      <P>
        In a project, a request waits for an answer instead, at the terminal or from{" "}
        <C>kavach approve</C>, as an <Link href="/docs/agents/agents/approve/">approval</Link> does.
      </P>
    </>
  ),
};

export const enforceDoc: LessonDoc = {
  lead: "enforce, the default: allowed calls run by themselves, a broken limit refuses, and an approval waits for a person.",
  simple: <>This is the mode an agent gets when you choose none: like a trusted employee, it acts on its own, but its rules still hold every time.</>,
  minutes: 2,
  body: (
    <>
      <ModeRun run={enforce} />
      <P>An agent whose settings name no <C>mode</C> runs in <C>Enforce</C>: its limits and approvals always act.</P>
    </>
  ),
};

export const dryRunDoc: LessonDoc = {
  lead: "kavach run --dry-run runs a program without making any capability call, and answers each one with a made-up value.",
  simple: <>A dry run is a rehearsal: the program goes through every step, but nothing real happens. Each outside call gets a pretend answer, so you see what would happen without touching anything or paying for an AI.</>,
  minutes: 3,
  body: (
    <>
      <Terminal commands session="kavach run --dry-run refunds.kav" />
      <Ex name="agents/dry-refund.dry" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>The bank&apos;s code never ran: no <C>the bank moves real money here</C> line.</li>
        <li>The refund got a made-up <C>Receipt</C>, generated from its type. The same seed gives the same values every run.</li>
        <li>Limits and approvals are still checked: the 900 refund fails as it would for real, and its <C>[dry run]</C> line says <C>refused</C> and why.</li>
        <li><C>ask</C> is answered the same way, so no model is called and nothing costs.</li>
      </ul>
      <Note>
        Each <C>[dry run]</C> line goes to the error output, apart from what the program prints. These pages show
        them after the program&apos;s own lines.
      </Note>
    </>
  ),
};

// ---------- Audit Log ----------

export const auditDoc: LessonDoc = {
  lead: "The runtime writes down every capability call an agent makes, with its arguments, its verdict and its result. No line of your code has to remember to.",
  simple: <>The audit log is like a ship&apos;s logbook that writes itself: every outside action the agent takes is noted, with what was asked and what happened. Entries are only ever added, never taken out.</>,
  minutes: 3,
  before: ["agents/agent-modes"],
  body: (
    <>
      <P>
        It is off until the agent&apos;s <Link href="/docs/agents/agents/settings/">settings</Link> turn it on with an{" "}
        <C>Audit</C>. The file is <C>audit.log</C> in the agent&apos;s folder, <C>agents/&lt;Name&gt;/</C>, unless the{" "}
        <C>Audit</C> names another:
      </P>
      <ModeRun run={enforce} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Each entry starts with the time (left out here, so the output is the same on every run), the agent and the event.</li>
        <li><C>verdict</C> says what the checks decided, and why; <C>result</C> says what happened.</li>
        <li>Refused calls are written too. The log only grows: nothing is ever taken out.</li>
      </ul>
      <H2>Secrets stay out</H2>
      <P>An argument of a <C>secret</C> parameter is written as <C>&lt;secret&gt;</C>:</P>
      <CodeBlock code={secretAudit.files[0].code} file="src/main.kav" output={secretAudit.output} />
      <H2>A log that can&apos;t be written</H2>
      <P>
        An agent never goes on unrecorded. Here the log is a folder (<C>file: &quot;.&quot;</C>), so no entry can be
        written to it, as on a full disk:
      </P>
      <Ex name="agents/audit-unwritable" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>
          An entry is written after its call, so the first call has run. Its entry is printed to standard error
          instead, with a line saying the audit log can&apos;t be written.
        </li>
        <li>
          From then on, each call first tries to write to the log. Until that works, the call is refused with{" "}
          <C>Denied</C>, without running, and the rest of the program goes on.
        </li>
        <li>
          The reason ends with the system&apos;s own words: here Windows&apos;s, and{" "}
          <C>No space left on device (os error 28)</C> for a full disk on Linux.
        </li>
        <li>
          When the log can be written again, its first line says so:{" "}
          <C>audit log writable again (1 entry printed to standard error)</C>.
        </li>
      </ul>
      <H2>A handler that panics</H2>
      <P>
        A handler that panics is stopped there, and the log gets a line naming the agent and the handler:{" "}
        <C>Picker pick panicked: &lt;message&gt; at &lt;file&gt;:&lt;line&gt;</C>. The send fails with the same{" "}
        <C>panicked: …</C>: see <Link href="/docs/agents/running-an-agent/">Running an Agent</Link>.
      </P>
      <P>
        In a durable agent the event runs again at the next start, and each run that panics writes the line again.
        After it comes the outcome line below, ending <C>error panicked: …</C>. The sender is gone after a restart, so
        nothing else is told, unless a program sent the same event again: that send fails with the same{" "}
        <C>panicked: …</C>.
      </P>
      <H2>An event that finishes after a restart</H2>
      <P>
        An event that runs again after a restart has no sender: the program that sent it is gone. So however it ends,
        its outcome goes in the log with the event&apos;s number. Here the program stops in the middle of an event,
        twice. The first event then succeeds, and the second is declined:
      </P>
      <CodeBlock code={restartOutcome.code} file="src/main.kav" />
      <Terminal session={restartOutcome.output ?? ""} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>
          The line is written for every such event, whether <C>start</C> waited for it, as here, or it went on after{" "}
          <Link href="/docs/agents/agents/approve/">asking a person</Link>.
        </li>
        <li>
          An error nothing read also goes to standard error, so it is seen with the audit log off too. That is the last
          line above. The program goes on, and its exit code doesn&apos;t change.
        </li>
        <li>
          A program that sent the same event again gets the outcome too, and reads the error, so then nothing is
          printed.
        </li>
      </ul>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={SPEC} label="spec 8.6 · audit log" />
        <SrcRef file="tools/cli/tests/durable_restarts.rs" label="outcomes after a restart, tested" />
        <SrcRef file="docs/decisions/0073-durable-agents-across-restarts.md" label="decision 0073" />
        <SrcRef file="docs/decisions/0072-approvals-that-wait-across-restarts.md" label="decision 0072" />
      </div>
    </>
  ),
};
