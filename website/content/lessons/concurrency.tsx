import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import { Terminal } from "@/components/terminal";
import type { LessonDoc } from "@/lib/chapter";
import { loadProject } from "@/lib/samples";
import benchmarks from "@/data/benchmarks.json";
import { Ex, H2, Note, P, Syntax } from "./parts";
import { Limits } from "./limits";

const SPEC = "docs/spec/09-concurrency.md";
const unread = loadProject("tasks/unread-error", ["src/main.kav"]);

// The tasks benchmark's Kavach times, read from its README by build-benchmarks.
const kavachMs = (id: "parallel" | "serial") => {
  const s = benchmarks.suites.tasks.benchmarks[id].series.find((x) => x.id === "kavach");
  if (!s) throw new Error(`benchmarks.json: tasks/${id} has no Kavach time`);
  return s.value.toLocaleString("en-US");
};

// ---------- Tasks ----------

export const tasksDoc: LessonDoc = {
  lead: "spawn starts a task that runs alongside the rest of the program; .wait() gets its result. There is no async and no await.",
  simple: <>A task is a job you start on the side while you keep working, like putting rice on the stove and chopping vegetables while it cooks. When you need the rice, you come back for it with <C>.wait()</C>.</>,
  minutes: 5,
  before: ["functions/functions", "errors/errors"],
  body: (
    <>
      <Syntax code={"const task = spawn work(…)\n// … other work meanwhile …\nconst result = try task.wait()"} />
      <Ex name="tasks/sub-spawn" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Any function can be spawned, and any function can wait: no function is marked <C>async</C>, so there are no two kinds of functions to keep apart.</li>
        <li>While a task waits, for the network, time or another task, the others run.</li>
        <li>Tasks are light: a server can run one per connection, many at once.</li>
      </ul>
      <H2>Two things at once</H2>
      <Ex name="tasks/sub-turns" />
      <P>The two sleeps overlap, so the program takes about 200 ms, not 300. Why it prints in exactly this order is on <Link href="/docs/concurrency/taking-turns/">Taking Turns</Link>.</P>
    </>
  ),
  tryIt: {
    task: <>Spawn three tasks that sleep 300, 100 and 200 ms and print their names. In what order do they print?</>,
    answer: <P>By how long each sleeps: the 100 ms one, then 200, then 300. A task whose wait ends first becomes ready first.</P>,
  },
  quiz: [
    {
      type: "choice",
      prompt: <>A function calls <C>time.sleep_ms(100)</C> inside a task. What do the program&apos;s other tasks do meanwhile?</>,
      options: ["They run: a waiting task gives up its turn", "They wait too", "Nothing: sleep needs an await"],
      answer: 0,
      explain: "Waiting lets other tasks run. Kavach has no await: any function can wait.",
    },
  ],
};

export const taskFailsDoc: LessonDoc = {
  lead: "A task's error comes out of .wait(), like the error of an ordinary call, and an error nobody reads is printed, not lost.",
  simple: <>If a job you handed off goes wrong, you find out when you come to collect its result, just as you would if you had done the job yourself.</>,
  minutes: 4,
  body: (
    <>
      <Ex name="tasks/wait-error" />
      <P><C>.wait()</C> gives the task&apos;s result or its error; <C>try</C> passes it on, <C>catch</C> handles it. Calling <C>.wait()</C> again gives the same.</P>
      <H2>An error nobody reads</H2>
      <P>
        A task that failed and is let go of before any <C>.wait()</C> read its error doesn&apos;t lose it: the error
        goes to standard error, with where the task was spawned. Then the program goes on, and its exit code
        doesn&apos;t change:
      </P>
      <CodeBlock code={unread.files[0].code} file="src/main.kav" />
      <Terminal session={unread.output ?? ""} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>
          A task is let go of when the function that spawned it returns. A <C>Task</C> handed out to other code is let
          go of when its last holder lets it go; in native code, at the latest when the program ends.
        </li>
        <li>An error that a <C>.wait()</C> read, as in the first example, is not printed again.</li>
        <li>
          An agent&apos;s event that runs again after a restart says so instead, with the event&apos;s number:{" "}
          <C>error in an event run again after a restart: …</C>. See{" "}
          <Link href="/docs/agents/audit-log/">the audit log</Link>.
        </li>
      </ul>
      <P>The compiler keeps such tasks in sight. A <C>spawn</C> whose <C>Task</C> is thrown away is an error, as for any value:</P>
      <Ex name="tasks/spawn-thrown-away.fails" />
      <P>
        A <C>Task</C> kept in a name that is never used gets a warning. A name starting with <C>_</C>, as in{" "}
        <C>_saving</C> above, says nobody waits for it on purpose and silences the warning; an error is still printed
        if it happens.
      </P>
      <Ex name="tasks/task-unused.warns" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={SPEC} label="spec 9.1 · tasks" />
        <SrcRef file="tests/conformance/09-concurrency/errors-nobody-reads-are-printed.kav" label="an unread error, tested" />
      </div>
      <Note>
        A panic is not an error, but a panic in a task stops only that task: it is printed when it happens, and{" "}
        <C>.wait()</C> fails with <C>panicked: &lt;message&gt; at &lt;file&gt;:&lt;line&gt;</C>. See{" "}
        <Link href="/docs/errors/panics/">Panics</Link>.
      </Note>
    </>
  ),
};

// ---------- Tasks Never Leak ----------

export const leakDoc: LessonDoc = {
  lead: "A task can't outlive the function that spawned it: when the function returns, its tasks still running are cancelled.",
  simple: <>A task belongs to the function that started it, like helpers who go home when the shift ends. When the function finishes, any helper still working is stopped, so nothing keeps running unseen.</>,
  minutes: 4,
  before: ["concurrency/tasks"],
  body: (
    <>
      <Ex name="tasks/sub-cancel" />
      <P>
        <C>watch</C> spawns a ticker that would tick forever, then returns after 350 ms. The ticker is cancelled at
        that moment, so nothing keeps running in the background by mistake.
      </P>
      <H2>When one part fails, the others stop</H2>
      <Ex name="tasks/siblings" />
      <P>
        <C>try b.wait()</C> passes the error on, so <C>both</C> returns, and its other task, still sleeping, is
        cancelled before it prints. No half-finished work goes on unseen.
      </P>
      <Note>
        Why it matters: in languages where tasks run on their own, a forgotten task can leak memory, hold a connection
        or write after the program thought it was done. Here the code&apos;s shape says how long every task lives.
      </Note>
    </>
  ),
};

// ---------- Many Things at Once ----------

export const manyDoc: LessonDoc = {
  lead: "list.parallel_map(f, max: n) runs f on every item in tasks of their own, at most n at a time, and gives the results in the list's order.",
  simple: <>Like a shop with a set number of counters: many items are handled at once, but never more than <C>max</C> at a time. However they finish, you get the results back in the list&apos;s order.</>,
  minutes: 3,
  before: ["concurrency/tasks", "collections/lists/map-filter-sum"],
  body: (
    <>
      <Ex name="tasks/sub-many" />
      <P>The results come back in the list&apos;s order, whichever task finishes first. Here the later pages finish first, yet the list is in order:</P>
      <Ex name="tasks/order" />
      <P>
        <C>max</C> keeps a program polite to a server: <C>urls.parallel_map(download, max: 10)</C> never has more
        than 10 downloads going at once. Without it, every task starts at once.
      </P>
      <P>
        Each task runs as <C>spawn thread</C> makes it (<Link href="/docs/concurrency/how-tasks-run/">How Tasks Run</Link>),
        so in a release build the items compute on several cores at once. In{" "}
        <Link href="/docs/benchmarks/#tasks">the benchmark</Link>, eight sums of 150 million rounds each take{" "}
        {kavachMs("parallel")} ms with <C>parallel_map</C> — and {kavachMs("serial")} ms with plain <C>map</C>,
        because each of these sums is a math loop that already{" "}
        <Link href="/docs/speed/loops-on-all-cores/">runs on all cores</Link> by itself. <C>parallel_map</C> is for
        items that can&apos;t spread themselves like that: downloads, files, work of uneven sizes.
      </P>
      <div><SrcRef file="docs/spec/09-concurrency.md" line={66} symbol="Each task runs as `spawn thread` makes it" label="spec 9.3" /></div>
    </>
  ),
};

// ---------- Channels ----------

export const channelsDoc: LessonDoc = {
  lead: "A channel passes values from one task to another, in order. put adds a value, take takes the next, and close says no more will come.",
  simple: <>A channel is like a conveyor belt between two workers: one puts things on, the other takes them off in the same order. Closing it says that nothing more is coming.</>,
  minutes: 5,
  before: ["concurrency/tasks"],
  body: (
    <>
      <Syntax code={"const (send, receive) = Channel<T>(capacity: n)"} />
      <Ex name="channels/sub-pipe" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>put</C> waits when the channel already holds <C>capacity</C> values: a fast producer can&apos;t run ahead and fill memory.</li>
        <li><C>for n in receive</C> takes values until the channel is closed and empty.</li>
        <li>Each value is copied into the channel, so the two tasks never share it.</li>
      </ul>
      <Note>Channels are how tasks talk without sharing anything: one task produces, another consumes, and neither waits for a lock.</Note>
      <H2>Putting without waiting</H2>
      <P>
        <C>send.try_put(v)</C> adds <C>v</C> if the channel has room and gives <C>true</C>. When it is full or
        closed, it gives <C>false</C> at once, and <C>v</C> is let go:
      </P>
      <Ex name="channels/try-put" />
      <P>It never waits, so one slow taker can&apos;t hold up a task that sends to many, such as a server sending news to every listener.</P>
      <SrcRef file="tests/conformance/09-concurrency/try-put-never-waits.kav" label="tested" />
      <H2>Limits and sizes</H2>
      <Limits id="tasks" />
    </>
  ),
};

export const putTakeDoc: LessonDoc = {
  lead: "take() gives the next value as an optional: none once the channel is closed and empty.",
  simple: <>Each <C>take()</C> hands you the next item, or tells you plainly that there is none, once the channel is closed and empty. That is how the receiver knows the work is over.</>,
  minutes: 2,
  body: (
    <>
      <Ex name="channels/sub-take" />
      <P>Putting into a closed channel is a bug, so it panics.</P>
    </>
  ),
};

export const sendersDoc: LessonDoc = {
  lead: "Several tasks can put into one channel; close it once all of them are done.",
  simple: <>Several cooks can put dishes on the same counter. Only when every cook has finished does someone say &quot;kitchen closed&quot;, so the waiter knows no more dishes are coming.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="channels/sub-workers" />
      <P>A third task waits for both senders, then closes the channel, so the loop that takes knows when to stop.</P>
    </>
  ),
};

export const deadlockDoc: LessonDoc = {
  lead: "If every task waits and none can go on, the program stops with a clear panic instead of hanging forever.",
  simple: <>It is like waiting at a stop for a bus that will never come. Kavach notices when no task can ever move on, and stops the program with a clear message instead of letting it wait forever.</>,
  minutes: 2,
  body: (
    <>
      <Ex name="channels/sub-deadlock.panics" />
      <P>The channel is never closed, so the loop waits for a value that can&apos;t come. Kavach sees that no task can go on, and says so. The fix: <C>send.close()</C> after the last <C>put</C>.</P>
    </>
  ),
};

// ---------- No Data Races ----------

export const racesDoc: LessonDoc = {
  lead: "Each task works on its own copy of a value, so two tasks can't change the same thing at once. To share on purpose, use Shared.",
  simple: <>Each task gets its own photocopy of a value, so two tasks never scribble on the same page at once. When they really must share, <C>Shared</C> is like one notebook that only one person may write in at a time.</>,
  minutes: 4,
  before: ["concurrency/tasks", "memory/values-not-references"],
  body: (
    <>
      <P>A lambda, and so each task, gets its own copy of <C>count</C>. Changing the copy doesn&apos;t change the outside one, and the compiler warns about it:</P>
      <Ex name="tasks/copy.warns" />
      <P>That is what makes data races impossible: there is nothing two tasks share by accident. To count across tasks, share the value on purpose with <C>Shared</C>, which locks for you:</P>
      <Ex name="tasks/sub-shared" />
      <Note>
        In most languages, two threads adding to one counter can lose updates, a bug that shows up only sometimes. In
        Kavach the only shared value is a <C>Shared</C>, and its <C>update</C> takes the lock itself: there is no
        &quot;remember to lock&quot;. See <Link href="/docs/memory/shared-state/">Shared State</Link>.
      </Note>
    </>
  ),
};

// ---------- Timeouts & Cancellation ----------

export const timeoutsDoc: LessonDoc = {
  lead: ".timeout(d) puts a time limit on a wait. task.cancel() and spawn … within d stop a task, even one stuck in a loop that never waits.",
  simple: <>A timeout is like deciding how long you will wait for a friend before you leave. If the wait runs past that time, you stop waiting and get an error you can handle. Cancelling is calling a helper off the job: they stop at once, even in the middle of it.</>,
  minutes: 5,
  before: ["concurrency/tasks"],
  body: (
    <>
      <Ex name="tasks/timeout" />
      <Ex name="channels/take-timeout" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>A timed-out <C>.wait()</C> leaves the task running; it is still cancelled when its function returns.</li>
        <li>A <C>put</C> that timed out puts nothing.</li>
      </ul>
      <H2>Cancelling a task</H2>
      <P><C>task.cancel()</C> stops a task, even one in a loop that never waits:</P>
      <Ex name="tasks/cancel" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>The task stops at the first of these: a wait, the start of a function call, or the start of a loop&apos;s round.</li>
        <li>
          It ends there, as if every function it is in returned at once: what they hold is released, and their
          cleanup (<C>Drop</C>) runs. The tasks it spawned are cancelled too.
        </li>
        <li>
          Cancelling is not an error: <C>catch</C> and <C>match</C> don&apos;t see it, so a task can&apos;t refuse to
          stop. Only <C>.wait()</C> on a cancelled task fails, with <C>the task was cancelled</C>.
        </li>
        <li>Cancelling a task that has ended does nothing.</li>
      </ul>
      <P>
        A cancel always wins, and nothing is left half-done all the same. Inside <C>Shared.update</C>, the task stops
        once the update&apos;s function returns, so the value is never half-changed. An agent&apos;s event that is cut
        short stays unfinished in its <Link href="/docs/agents/durable-agents/">journal</Link> and runs again after a
        restart. A durable write is all there or not at all, as after a crash.
      </P>
      <P>
        A function that returns cancels its tasks more gently: each stops at its next wait, so a task that only
        computes finishes its work first, and the function waits for it:
      </P>
      <Ex name="tasks/finish-at-return" />
      <H2>A time limit at spawn</H2>
      <P>
        A task stuck in a loop holds its turn, so no other task runs to cancel it. <C>within d</C> after the spawned
        call or block gives it <C>d</C> to finish. Then it stops itself, as if cancelled, and <C>.wait()</C> on it
        fails:
      </P>
      <Ex name="tasks/spawn-within" />
      <P>
        The error is the same as from <C>.timeout(d)</C>. The difference: <C>.timeout(d)</C> stops only the wait and
        leaves the task running, while <C>within d</C> stops the task.
      </P>
      <Note>
        Only a program that calls <C>cancel()</C> or gives a time limit somewhere gets these checks; others pay
        nothing. A counted loop whose body runs straight through, with no call and no inner loop, gets none: it ends
        anyway, and it stays a loop the compiler can vectorize or{" "}
        <Link href="/docs/speed/loops-on-all-cores/">run on all cores</Link>.
      </Note>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={SPEC} label="spec 9.6 · timeouts and cancellation" />
        <SrcRef file="docs/decisions/0075-cancelling-tasks.md" label="decision 0075" />
        <SrcRef file="tests/conformance/09-concurrency/a-time-limit-stops-a-stuck-task.kav" label="a time limit, tested" />
      </div>
    </>
  ),
};
