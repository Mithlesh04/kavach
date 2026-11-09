import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import { TaskTurns } from "@/components/task-turns";
import type { LessonDoc } from "@/lib/chapter";
import { loadProject, loadSample } from "@/lib/samples";
import { fileFigure, fileTables, specBlock, specTables } from "@/lib/spec-tables";
import { Ex, H2, Note, P, Syntax } from "./parts";
import { Table } from "./spec-table";

const SPEC = "docs/spec/09-concurrency.md";
const ALL_CORES = "docs/architecture/tasks-on-all-cores.md";

// The look as it runs today, side by side with no look and C++, from the runs in the architecture doc.
function socketLook() {
  const table = fileTables(ALL_CORES).find((t) => t.header[0] === "Case");
  if (!table) throw new Error(`${ALL_CORES}: no table of the look that gives up`);
  return table;
}
const slowServerCpu = () => ({
  without: fileFigure(ALL_CORES, /one client 200 µs apart ([\d.–]+ s) without a look/),
  with: fileFigure(ALL_CORES, /without a look, ([\d.–]+ s) with it/),
});
const idleCoresCpu = () => fileFigure(ALL_CORES, /mostly idle `spawn cores` program does pay: (\d+% → \d+%) of a core/);

// ---------- Taking Turns ----------

const turns = loadSample("tasks/sub-turns");

export const turnsDoc: LessonDoc = {
  lead: "Tasks take turns in an order the language defines, so a program with tasks prints the same thing on every run.",
  simple: <>Tasks take turns by fixed rules, like players in a board game. Because the order never changes, the program prints the same thing every time, so a bug you see once you can see again, and fix.</>,
  minutes: 4,
  before: ["concurrency/tasks"],
  body: (
    <>
      <CodeBlock code={turns.code} file="turns.kav" output={turns.output} />
      <P>Step through it:</P>
      <TaskTurns output={(turns.output ?? "").split("\n")} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>spawn</C> makes the new task ready; the task that spawned it keeps running.</li>
        <li>A task runs until it waits. Then the task that became ready first runs.</li>
        <li>When no task is ready, the program waits for the earliest of them.</li>
      </ul>
      <Note>
        Why it matters: a bug in a program with threads often shows up once in a hundred runs. With fixed turns, the
        same program does the same thing every time, so a bug that happens once happens every time, and can be found.
      </Note>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={SPEC} label="spec 9.7 · taking turns" />
        <SrcRef file="docs/decisions/0020-tasks-take-turns.md" label="decision 0020" />
      </div>
    </>
  ),
};

// ---------- How Tasks Run ----------

const tasksToml = loadProject("tasks/tasks-toml.fails", ["kavach.toml"]);
/** The kinds of spawn, as spec 9.8 lists them. */
const kinds = () => {
  const t = specTables("09-concurrency.md", "9.8").find((x) => x.header[0] === "Written");
  if (!t) throw new Error("spec 9.8: no Written table");
  return t;
};

export const howTasksRunDoc: LessonDoc = {
  lead: "Each spawn says how its task runs: plain spawn is a coroutine, spawn thread gets a thread of its own, spawn cores runs on any core, spawn process gets a process of its own. The turns, and the output, stay the same.",
  simple: <>You choose, task by task, how the work is shared out: one worker switching between many small jobs, a worker of its own for a job that blocks, every core for heavy work, or a separate room for risky work that might get stuck. Either way the tasks take turns in the same order, so only the speed changes, never the result.</>,
  minutes: 6,
  before: ["concurrency/taking-turns"],
  body: (
    <>
      <Syntax code={specBlock("09-concurrency.md", "9.8")} />
      <Table header={kinds().header} rows={kinds().rows} />
      <P>Three of them in one program: a producer on the cores, a slow task on a thread of its own, a fast coroutine:</P>
      <Ex name="tasks/spawn-kinds" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>These three take the same things, a call or a block, and each gives a <C>Task&lt;T&gt;</C>.</li>
        <li><C>thread</C>, <C>cores</C> and <C>process</C> are words only right after <C>spawn</C>; anywhere else they are names like any other.</li>
        <li>
          Whichever you choose, tasks take turns in the same order, so the program prints the same: only its speed
          changes. A <C>spawn cores</C> task keeps a looser order: its own lines come out in order, whole, and a line
          it prints before it sends a value, spawns a task or ends comes before what the receiver prints after that.
        </li>
        <li>Only a release build (<C>kavach build</C>) runs <C>spawn cores</C> tasks on every core today. <C>kavach run</C> runs them on one core, in turn, and prints the same.</li>
      </ul>
      <H2>Waiting for the network</H2>
      <P>
        A thread with nothing to run looks at its sockets for a moment before it sleeps. A reply usually comes in that
        moment, so the thread skips going to sleep and being woken. Tasks of every kind do this, and there is nothing
        to write in your code. When the looks keep finding nothing, because replies come later than a look lasts, the
        thread gives up: it sleeps at once, and only looks again now and then. Measured on Linux in WSL, in ms, as
        ranges over several rounds:
      </P>
      <Table header={socketLook().header} rows={socketLook().rows} wrapFirst />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>
          One client beats C++ here only because C++&apos;s thread sleeps in each <C>recv</C>. A C++ program that
          polled would gain the same.
        </li>
        <li>With 100 clients on <C>spawn cores</C>, C++ is still ahead.</li>
        <li>
          A plain <C>spawn</C> server with 100 busy clients gains nothing: its one worker never has nothing to do. C++
          wasn&apos;t run that day, so its cell is empty. Plain <C>spawn</C> stays on one core, by design, so C++ on all
          cores is far ahead there.
        </li>
        <li>
          A slow plain <C>spawn</C> server&apos;s CPU doesn&apos;t change: with one client 200 µs apart,{" "}
          {slowServerCpu().without} without a look and {slowServerCpu().with} with it.
        </li>
        <li>
          A mostly idle <C>spawn cores</C> program does pay: {idleCoresCpu()} of a core, with 100 clients waiting 5 ms
          between trips.
        </li>
        <li>
          On Windows the look on <C>spawn cores</C>, measured before it learned to give up, moved neither the time nor
          the CPU. Plain <C>spawn</C> and giving up were not measured there on their own. What takes the time on
          Windows is something else, not yet profiled.
        </li>
      </ul>
      <div><SrcRef file={ALL_CORES} label="how tasks run on all cores: every run measured" /></div>
      <H2>A process of its own</H2>
      <P>
        <C>spawn process f(args)</C> starts the program again, and the copy runs only that call. It is for code
        Kavach can&apos;t check, such as a C or Python library that may hang: a process can be ended at once.
      </P>
      <Ex name="tasks/spawn-process" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>
          The arguments go in as copies, and the value or the error comes back through <C>.wait()</C>, as from any
          task. What the process prints comes out in turn, like any task&apos;s lines.
        </li>
        <li>
          <C>cancel()</C> or <C>within</C> ends the process at once. It shares nothing, so nothing is left
          half-changed. A thread is never ended this way: one stopped in the middle could leave what it shares
          half-changed.
        </li>
        <li>
          A panic in the call fails its task, as in any task: <C>.wait()</C> gives{" "}
          <C>panicked: &lt;message&gt; at &lt;file&gt;:&lt;line&gt;</C>, and the program goes on.
        </li>
        <li>Starting a process takes milliseconds, so it is for long or risky work.</li>
        <li>
          The copy knows its call from <C>KAVACH_PROCESS_CALL</C>, which <C>spawn process</C> sets. It is not for
          setting by hand.
        </li>
      </ul>
      <P>It takes a call of a function, not a block:</P>
      <Ex name="tasks/spawn-process-block.fails" />
      <P>
        The arguments and the result go between the processes as JSON, so each must have a JSON form. A{" "}
        <C>Shared</C> value, a channel, a task, a function or an agent has none, and the compiler says which one:
      </P>
      <Ex name="tasks/spawn-process-shared.fails" />
      <Note>
        Refused for now: a generic function, and <C>Bytes</C> going in or out, since <C>Bytes</C> has no JSON form
        yet. In the interpreter, <C>spawn process</C> works under <C>kavach run</C>; a tool that runs the interpreter
        inside itself stops with a panic that says to use <C>kavach run</C>.
      </Note>
      <P>How tasks run is chosen in the code, not in <C>kavach.toml</C>. The old <C>tasks =</C> setting says where it went:</P>
      <CodeBlock code={tasksToml.files[0].code} file="kavach.toml" lang="toml" output={tasksToml.output} outcome="refused" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={SPEC} label="spec 9.8 · how tasks run" />
        <SrcRef file="docs/decisions/0069-each-task-chooses-how-it-runs.md" label="decision 0069" />
        <SrcRef file="docs/decisions/0048-tasks-as-coroutines-per-core.md" label="decision 0048" />
        <SrcRef file="docs/decisions/0075-cancelling-tasks.md" label="decision 0075 · spawn process" />
        <SrcRef file="tests/conformance/09-concurrency/spawn-process-runs-a-call-in-its-own-process.kav" label="spawn process, tested" />
      </div>
    </>
  ),
};
