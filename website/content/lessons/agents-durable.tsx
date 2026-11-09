import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import { Terminal } from "@/components/terminal";
import type { LessonDoc } from "@/lib/chapter";
import { loadProject } from "@/lib/samples";
import { H2, Note, P, Syntax } from "./parts";

const durable = loadProject("agents/durable", ["src/main.kav"]);
const state = loadProject("agents/state", ["src/main.kav"]);
const keyed = loadProject("agents/call-key", ["src/main.kav"]);
const setAside = loadProject("agents/set-aside", ["src/main.kav"]);

// ---------- Durable Agents ----------

export const durableDoc: LessonDoc = {
  lead: "An agent keeps a journal, so a handler that stopped half way finishes after a restart, without doing a finished call again.",
  simple: <>A journal is like a list where you tick off each step as you finish it, and write down what came of it. If the power goes out, you start again from the top, but a ticked step isn&apos;t done again: its result is read from the list, so a refund already paid isn&apos;t paid again.</>,
  minutes: 5,
  before: ["agents/running-an-agent"],
  body: (
    <>
      <P>
        There is nothing to set: in a project, every agent keeps its journal in its folder,{" "}
        <C>agents/&lt;Name&gt;/journal</C>, beside <C>kavach.toml</C>. Here the bot pays a refund, then the power
        goes out before it sends the email:
      </P>
      <CodeBlock
        code={durable.files[0].code}
        file="src/main.kav"
        notes={[
          { target: "cli.exit(3)", chip: "POWER CUT", kind: "why", what: "Stands in for the machine dying: the refund is paid, the email not sent.", why: "the worst moment to crash, half the work done." },
        ]}
      />
      <P>Run it twice, first with the power cut, then normally:</P>
      <Terminal session={durable.output ?? ""} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>The journal records each event, then the result of each capability call, then that the handler finished.</li>
        <li>At <C>start</C>, every event that never finished runs again from the beginning.</li>
        <li>A call already in the journal gives its recorded result without running: there is no second <C>[pay]</C> line, so the refund is paid once.</li>
        <li>The first call not in the journal, the email, runs for real.</li>
        <li>
          The first run&apos;s sender is gone, so no one gets the event&apos;s result. The outcome goes in the{" "}
          <Link href="/docs/agents/audit-log/">audit log</Link> if it is on, and an error nothing read is printed either
          way.
        </li>
        <li>
          Several events can be under way at once, each in its own task, as when one waits for a person while another
          runs. Each call goes in the journal under the event that made it, even a call from a task its handler
          spawned, so after a restart each event gets back its own results.
        </li>
        <li>When the agent starts, events that ended are dropped from the journal, so it stays small.</li>
        <li>
          A journal that can&apos;t be written, on a full disk, stops the program with a panic, as a database stops when
          its log fails: going on would break the promise.
        </li>
        <li>
          A call the crash cut off, before its result was written, runs again: each call runs at least once, and may
          run twice.
        </li>
      </ul>
      <Note>
        This works because a handler reaches the world only through capabilities, so a handler must be safe to run
        again from its start. If the program changed and a handler makes a different call than the journal recorded,
        the event stops with a panic. The journal holds secret values too, so keep it as safe as other secrets: the{" "}
        <C>.gitignore</C> that <C>kavach new</C> writes leaves <C>agents/</C> out. <C>journal: false</C> in the
        agent&apos;s <Link href="/docs/agents/agents/settings/">settings</Link> turns it off.
      </Note>
      <H2>Time is replayed too</H2>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>
          A handler&apos;s clock reads (<C>time.now_ms()</C>, <C>today()</C>, <C>now()</C>) go in the journal, so a
          run after a crash sees the same time and makes the same calls.
        </li>
        <li>
          <C>time.sleep</C> records when the wait ends. After a restart it waits only for what is left, and never more
          than it was asked to, so a clock set back can&apos;t stretch it.
        </li>
        <li>
          <C>every</C> records each round&apos;s wait the same way. After a restart, rounds already waited for pass at
          once, and the next one waits only for what is left.
        </li>
        <li>
          A <C>.timeout(d)</C> records when it ends, so after a restart it times out when the first run would have,
          not a whole <C>d</C> later.
        </li>
        <li>Inside a capability&apos;s own code, the clock isn&apos;t journaled: the call&apos;s result is.</li>
      </ul>
      <Syntax code={'on remind(order_id: Int) ! Error {\n    try Email.send("Your order {order_id} ships soon.")\n    time.sleep(1.days)            // a crash an hour in: after the restart, 23 hours more\n    try Email.send("Your order {order_id} has shipped.")\n}'} />
      <H2>Each call has a key</H2>
      <P>
        A call the crash cut off runs again, so a service may see it twice. Each call in a handler has a key, the same
        in the run after a crash, and the capability&apos;s implementation reads it with{" "}
        <Link href="/docs/standard-library/agents/">agents.call_key()</Link>. Here the email crashes, then runs again
        with the same key:
      </P>
      <CodeBlock code={keyed.files[0].code} file="src/main.kav" />
      <Terminal session={keyed.output ?? ""} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>A key is the agent, the event&apos;s number and the call&apos;s place in it: <C>Bot-1-2</C> is the second call of event 1.</li>
        <li>A service that is told the key can drop the repeat, as Stripe and other payment APIs do with an <C>Idempotency-Key</C> header.</li>
        <li>Outside a call from a handler, the key is <C>none</C>.</li>
      </ul>
      <H2>An event that never finishes</H2>
      <P>
        An event that stops the program each time it runs, from a bug, would stop the agent again after every
        restart. So after 3 starts it is set aside. Here the payment crashes the program 3 times, then the program
        starts normally and prints what the agent&apos;s folder holds:
      </P>
      <CodeBlock code={setAside.files[0].code} file="src/main.kav" />
      <Terminal session={setAside.output ?? ""} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Each start of an unfinished event is written to the journal: <C>{"{\"again\":1}"}</C> is event 1 starting again.</li>
        <li>
          The 4th start moves the event&apos;s lines from the journal to <C>set-aside</C> in the agent&apos;s folder,
          writes it in the audit log, and starts the agent with the rest.
        </li>
        <li>A person can read the event there, fix the program and send it again. Message queues call this a dead-letter queue.</li>
        <li>
          A handler that panics counts too. Its send fails, but the event stays unfinished, so it runs again at the
          next start, until it is set aside.
        </li>
      </ul>
      <H2>One program per agent</H2>
      <P>
        An agent with a state file or a journal keeps them in its folder, and the program that starts it locks the
        folder (the file <C>lock</C> in it) while it runs. A second program that starts the same agent waits up to 5
        seconds, so a restart can follow a program that is just ending. Then it stops with a panic, as a second
        database server refuses a data folder in use:
      </P>
      <Syntax code={"panic: the agent RefundBot already runs in another program: its folder agents/RefundBot is locked"} />
      <P>
        A real run names the folder&apos;s full path. The lock is the operating system&apos;s, so a program that
        crashed holds none.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/decisions/0025-agents-resume-by-replaying-a-journal.md" label="decision 0025" />
        <SrcRef file="docs/decisions/0073-durable-agents-across-restarts.md" label="decision 0073 · bad restarts" />
        <SrcRef file="tools/cli/tests/durable_restarts.rs" label="set aside and locked, tested" />
        <SrcRef file="tools/cli/tests/durable_overlap.rs" label="events at once, tested" />
        <SrcRef file="docs/decisions/0062-crashes-restarts-and-clocks.md" label="decision 0062 · crashes and clocks" />
        <SrcRef file="docs/architecture/crashes-and-restarts.md" label="every case, and its status" />
        <SrcRef file="tools/cli/tests/durable.rs" label="the same story, tested three ways" />
        <SrcRef file="tools/cli/tests/durable_time.rs" label="time after a crash, tested" />
      </div>
    </>
  ),
  quiz: [
    {
      type: "choice",
      prompt: <>After the restart, what happens to the refund that was paid before the power cut?</>,
      options: ["Its result comes from the journal: it is not paid again", "It is paid a second time", "The agent stops with an error"],
      answer: 0,
      explain: "A call in the journal gives its recorded result without running again.",
    },
  ],
};

export const stateDoc: LessonDoc = {
  lead: "An agent keeps its limit totals in a state file, so a daily limit stays daily even if the program starts again.",
  simple: <>A state file is like a notebook where the agent writes down how much it has used. When the program starts again, it reads the notebook, so a daily limit can&apos;t be reset just by restarting.</>,
  minutes: 2,
  body: (
    <>
      <P>
        There is nothing to set: in a project, the totals are kept in <C>agents/&lt;Name&gt;/state.json</C>. The agent
        may send 3 emails a day, and each run sends 2. Run it twice:
      </P>
      <Terminal session={state.output ?? ""} />
      <P>
        The second run knew about the first: its second email would have been the fourth that day. With{" "}
        <C>state: false</C> in the agent&apos;s settings, totals last only while the program runs.{" "}
        <C>kavach test</C> and dry runs keep no state, so each starts fresh. See also{" "}
        <Link href="/docs/agents/agents/limit/">limit</Link>.
      </P>
      <H2>Crashes, reboots and clock changes</H2>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>
          A call counts as soon as it passes its checks, and the state file is written then. A call that then fails
          stops counting; one a crash cut off stays counted after the restart. An <C>ask</C>&apos;s tokens are added
          once its answer is in.
        </li>
        <li>Each call is saved with its time in UTC, so the windows carry on across a reboot of the machine too.</li>
        <li>
          A clock set back while the program was down makes saved calls look still to come. They count as made just
          now, so a limit never loosens.
        </li>
        <li>
          A clock set forward does the opposite: saved calls look older and can leave their window early. Nothing on
          the machine can tell such a jump from time passing.
        </li>
      </ul>
      <SrcRef file="docs/decisions/0062-crashes-restarts-and-clocks.md" label="decision 0062 · crashes and clocks" />
    </>
  ),
};
