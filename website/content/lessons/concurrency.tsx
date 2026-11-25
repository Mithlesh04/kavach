import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { Explainer } from "@/components/explain/explainer";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import { Terminal } from "@/components/terminal";
import type { LessonDoc } from "@/lib/chapter";
import { loadProject, loadSample } from "@/lib/samples";
import benchmarks from "@/data/benchmarks.json";
import { Ex, H2, Note, P, Syntax } from "./parts";
import { Limits } from "./limits";

// How long `watch` waits before it returns, read from the sample.
const watchMs = (() => {
  const ms = loadSample("tasks/sub-cancel").code.match(/sleep_ms\((\d+)\)/)?.[1];
  if (!ms) throw new Error("tasks/sub-cancel: no time.sleep_ms(…) to name");
  return ms;
})();

/** How long slow and fast sleep in tasks/sub-turns, read from the sample. */
function turnSleeps(code: string) {
  const ms = (name: string) => {
    const found = code.match(new RegExp(`report\\("${name}", (\\d+)\\)`))?.[1];
    if (!found) throw new Error(`tasks/sub-turns: no report("${name}", …) to name`);
    return Number(found);
  };
  return { slow: ms("slow"), fast: ms("fast") };
}
const sleeps = turnSleeps(loadSample("tasks/sub-turns").code);

const SPEC = "docs/spec/09-concurrency.md";
const unread = loadProject("tasks/unread-error", ["src/main.kav"]);
const alone = loadProject("tasks/spawn-alone", ["src/main.kav"]);

// The tasks benchmark's Kavach times, read from its README by build-benchmarks.
const kavachMs = (id: "parallel" | "serial") => {
  const s = benchmarks.suites.tasks.benchmarks[id].series.find((x) => x.id === "kavach");
  if (!s || s.value === null) throw new Error(`benchmarks.json: tasks/${id} has no Kavach time`);
  return s.value.toLocaleString("en-US");
};

const itemPanic = loadProject("tasks/parallel-map-panic", ["src/main.kav"]);

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
      <P>
        <C>spawn</C> takes the call right after it. So <C>spawn count(4) * 10</C> is a task times a number, a compile
        error. Put the arithmetic inside the call, <C>spawn count(4 * 10)</C>, or multiply what <C>.wait()</C> gives:
      </P>
      <Ex name="tasks/spawn-times.fails" />
      <H2>What the task is given</H2>
      <P>
        The call&apos;s arguments are worked out at the spawn, before the task starts, as Go&apos;s{" "}
        <C>go f(x)</C> does. The task then makes the call with those values. A block after <C>spawn</C> runs whole in
        the task:
      </P>
      <Ex name="tasks/spawn-args" />
      <P>
        So a task gets only what its call is given. Working an argument out is part of the spawning code, so a panic
        there happens at the spawn&apos;s line, and no task starts:
      </P>
      <Ex name="tasks/spawn-arg.panics" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>In <C>spawn x.m(a, b)</C>, the value <C>x</C> is worked out at the spawn too, then <C>a</C> and <C>b</C>, in the order they are written.</li>
        <li>A <C>try</C> in an argument passes its error on from the spawning function. An error of the call itself comes out of <C>.wait()</C>.</li>
        <li><C>within d</C>: <C>d</C> is worked out at the spawn, after the arguments.</li>
        <li>
          Worked out in the task instead: a call through <C>?.</C>, a wait with a time limit, an <C>inout</C>{" "}
          argument, and the list, dict or set a changing method such as <C>push</C> is called on. These last two are
          places, not values: the task changes its own copy.
        </li>
      </ul>
      <P>
        A function worked out from an expression is worked out at the spawn too, first: in{" "}
        <C>spawn handler_for(kind)(job)</C>, <C>handler_for(kind)</C> and then <C>job</C>. Only the call runs in the
        task:
      </P>
      <Ex name="tasks/spawn-fn" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/09-concurrency.md" label="spec 9.1 · arguments at the spawn" />
        <SrcRef file="docs/decisions/0138-spawn-arguments-first.md" label="decision 0138" />
        <SrcRef file="tests/conformance/09-concurrency/a-spawned-call-gets-its-arguments-worked-out-first.kav" label="tested" />
        <SrcRef file="tests/conformance/09-concurrency/a-panic-in-a-spawned-argument-happens-at-the-spawn.kav" label="panic at the spawn, tested" />
        <SrcRef file="tests/conformance/09-concurrency/a-try-in-a-spawned-argument-is-the-spawners.kav" label="try, tested" />
        <SrcRef file="tests/conformance/09-concurrency/a-spawned-call-through-optional-runs-in-the-task.kav" label="?. in the task, tested" />
        <SrcRef file="tests/conformance/09-concurrency/a-spawned-function-is-worked-out-at-the-spawn.kav" label="the function first, tested" />
      </div>
      <H2>Two things at once</H2>
      <Ex name="tasks/sub-turns" />
      <P>The two sleeps overlap, so the program takes about {sleeps.slow} ms, not {sleeps.slow + sleeps.fast}. Why it prints in exactly this order is on <Link href="/docs/concurrency/taking-turns/">Taking Turns</Link>.</P>
      <H2>What the machine does</H2>
      <P>A plain <C>spawn</C> starts no thread. In native code the task runs on the thread that spawned it, with a stack of its own:</P>
      <Explainer id="spawn" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="runtime/native/src/rt_tasks.rs" line={246} symbol="fn spawn" />
        <SrcRef file="runtime/native/src/rt_coroutines.rs" line={194} symbol="pub(crate) fn start" />
        <SrcRef file="runtime/core/src/tasks/pick.rs" line={35} symbol="fn pick" />
      </div>
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
      <P>
        A <C>spawn</C> on a line of its own starts a task nobody waits for, as <C>go f()</C> does in Go.{" "}
        <C>spawn save(r)</C> is the same as <C>const _t = spawn save(r)</C>:
      </P>
      <CodeBlock code={alone.files[0].code} file="src/main.kav" />
      <Terminal session={alone.output ?? ""} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>The tasks are still cancelled when <C>main</C> returns. A task cancelled that way stops at its next wait; these two have none, so they finish first.</li>
        <li>An error in such a task is still printed, with where it was spawned.</li>
      </ul>
      <P>
        A <C>Task</C> kept in a name that is never used gets a warning. A name starting with <C>_</C>, as in{" "}
        <C>_saving</C> above, says nobody waits for it on purpose and silences the warning.
      </P>
      <Ex name="tasks/task-unused.warns" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={SPEC} label="spec 9.1 · tasks" />
        <SrcRef file="tests/conformance/09-concurrency/errors-nobody-reads-are-printed.kav" label="an unread error, tested" />
        <SrcRef file="tests/conformance/09-concurrency/a-spawn-alone-is-waited-for-by-nobody.kav" label="a spawn alone, tested" />
      </div>
      <Note>
        A panic is not an error, but a panic in a task stops only that task: it is printed when it happens, and{" "}
        <C>.wait()</C> fails with <C>panicked: &lt;message&gt; at &lt;file&gt;:&lt;line&gt;</C>. When that error
        ends <C>main</C>, the last line doesn&apos;t print the panic again: it says{" "}
        <C>error: the panic above ended the program</C>. See{" "}
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
        <C>watch</C> spawns a ticker that would tick forever, then returns after {watchMs} ms. The ticker is cancelled at
        that moment, so nothing keeps running in the background by mistake.
      </P>
      <H2>What the machine does</H2>
      <P>Sleeping tasks hold no core, and the ticker&apos;s first tick waits one interval:</P>
      <Explainer id="ticker" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="compiler/kir/src/lower_every.rs" line={76} symbol="Stmt::Let(next, clock)" label="every_ms: the first tick" />
        <SrcRef file="compiler/kir/src/lower_tasks.rs" line={272} symbol="join_tasks" label="the wait at return" />
        <SrcRef file="runtime/core/src/tasks.rs" line={369} symbol="pub fn cancel" />
        <SrcRef file="runtime/core/src/tasks/pick.rs" line={144} symbol="wait_timeout" label="the thread waits" />
      </div>
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
        so in an LLVM build (<C>kavach build</C>) the items compute on several cores at once. In{" "}
        <Link href="/docs/benchmarks/#tasks">the benchmark</Link>, eight sums of 150 million rounds each take{" "}
        {kavachMs("parallel")} ms with <C>parallel_map</C>, a thread for each sum, as the C++ version has. The same
        sums one after another, with plain <C>map</C>, are being measured again on one core. A loop that can be
        split is written <Link href="/docs/speed/loops-on-all-cores/"><C>on cores for</C></Link> instead;{" "}
        <C>parallel_map</C> is for items that can&apos;t be split like that: downloads, files, work of uneven sizes.
      </P>
      <div><SrcRef file="docs/spec/09-concurrency.md" line={68} symbol="Each task runs as `spawn thread` makes it" label="spec 9.3" /></div>
      <H2>When an item fails or panics</H2>
      <P>
        Given a function that can fail, <C>parallel_map</C> fails with the first error in the items&apos; order. A panic
        in an item&apos;s task is printed when it happens, as any task&apos;s is, and is then a panic where{" "}
        <C>parallel_map</C> was called: the call can&apos;t fail by its type, so there is no error to give back.
      </P>
      <CodeBlock code={itemPanic.files[0].code} file="src/main.kav" />
      <Terminal session={itemPanic.output ?? ""} />
      <SrcRef file="tests/conformance/09-concurrency/parallel-map-passes-a-panic-on.kav" label="tested" />
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
        <li>Each value goes in as a value of its own: changing yours afterwards never changes what the taker gets. The runtime makes no copy unless one is needed. With one worker, an object goes in shared, its count raised; with several workers it is copied, as tasks on other threads must not share counts.</li>
      </ul>
      <Note>Channels are how tasks talk without sharing anything: one task produces, another consumes. There is no lock you can see, hold or wait on: a channel&apos;s own bookkeeping takes a few nanoseconds inside the runtime, and a full or empty channel parks only the task, never a thread or a core.</Note>
      <H2>What the machine does</H2>
      <P>A producer and a consumer, with room for only 2 values between them:</P>
      <Explainer id="channel" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="runtime/native/src/rt_channels.rs" line={137} symbol="struct ChanState" />
        <SrcRef file="runtime/native/src/rt_channels.rs" line={317} symbol="fn put" />
        <SrcRef file="runtime/native/src/rt_channels.rs" line={395} symbol="fn next" label="take" />
      </div>
      <H2>Putting without waiting</H2>
      <P>
        <C>send.try_put(v)</C> adds <C>v</C> if the channel has room and gives <C>true</C>. When it is full or
        closed, it gives <C>false</C> at once, and <C>v</C> is let go:
      </P>
      <Ex name="channels/try-put" />
      <P>It never waits, so one slow taker can&apos;t hold up a task that sends to many, such as a server sending news to every listener.</P>
      <SrcRef file="tests/conformance/09-concurrency/try-put-never-waits.kav" label="tested" />
      <H2>When the receivers are gone</H2>
      <P>
        A receiver is gone once nothing holds it: every copy of the <C>Receiver</C> was let go, as when the task that
        held it ended. Then <C>try_put</C> gives <C>false</C>, so a task that sends to many, such as a chat room,
        sees at once that a member left:
      </P>
      <Ex name="channels/receivers-gone" />
      <P>
        <C>members.filter(m =&gt; m.try_put(…))</C> keeps the members that took the message: it drops those who left,
        and those too slow to keep up. A <C>put</C> nobody could ever take panics, and so does a <C>put</C> waiting
        for room when the last receiver goes:
      </P>
      <Ex name="channels/put-nobody-takes.panics" />
      <P>A channel&apos;s ends live to the end of their block, so the receiver above is gone once its block ends.</P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="tests/conformance/09-concurrency/a-channel-with-no-receivers-takes-nothing.kav" label="tested" />
        <SrcRef file="tests/conformance/09-concurrency/a-waiting-put-ends-when-the-receivers-go.kav" label="a waiting put, tested" />
      </div>
      <H2>Limits and sizes</H2>
      <Limits id="tasks" />
    </>
  ),
};

export const putTakeDoc: LessonDoc = {
  lead: "take() gives the next value as Taken.Item(value), or Taken.Closed once the channel is closed and empty.",
  simple: <>Each <C>take()</C> hands you the next item, or tells you plainly that there is none, once the channel is closed and empty. That is how the receiver knows the work is over.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="channels/sub-take" />
      <P>
        <C>{"Taken<T> = Item(value: T) | Closed"}</C> keeps a value and the channel&apos;s end apart, even when the
        values are optional. A <C>{"Channel<Int?>"}</C> can carry <C>none</C>, and <C>none</C> taken is{" "}
        <C>Item(value: none)</C>, not <C>Closed</C>:
      </P>
      <Ex name="channels/take-none" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>for v in receive</C> takes values until <C>take()</C> gives <C>Closed</C>.</li>
        <li>Putting into a closed channel is a bug, so it panics, as does putting into one whose receivers are all gone.</li>
      </ul>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="tests/conformance/09-concurrency/a-sent-none-is-not-the-end.kav" label="tested" />
        <SrcRef file="docs/decisions/0097-take-gives-taken.md" label="decision 0097" />
      </div>
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
        anyway, and it stays a loop the compiler can vectorize or, with{" "}
        <Link href="/docs/speed/loops-on-all-cores/"><C>on cores</C></Link>, run on all cores.
      </Note>
      <H2>Cleanup however a task ends</H2>
      <P>
        Cleanup that must run however a task ends, cancelled, failed or done, is a value whose type
        implements <Link href="/docs/memory/automatic-memory/freed-immediately/#cleanup-of-your-own-drop">Drop</Link>.
        There is no <C>defer</C> or <C>on_cancel</C> block: one way covers every exit. Here a guest is cancelled while
        it waits, and its seat still comes back:
      </P>
      <Ex name="tasks/cancel-cleanup" />
      <P>
        A panic is the exception: it runs no <C>drop</C>, and what the task held is never dropped. See{" "}
        <Link href="/docs/memory/automatic-memory/freed-immediately/#cleanup-of-your-own-drop">Drop</Link>.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="tests/conformance/09-concurrency/cancelled-tasks-run-their-cleanup.kav" label="tested" />
        <SrcRef file="tests/conformance/09-concurrency/cancelled-tasks-drop-at-once.kav" label="native code too, tested" />
      </div>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={SPEC} label="spec 9.6 · timeouts and cancellation" />
        <SrcRef file="docs/decisions/0075-cancelling-tasks.md" label="decision 0075" />
        <SrcRef file="tests/conformance/09-concurrency/a-time-limit-stops-a-stuck-task.kav" label="a time limit, tested" />
      </div>
    </>
  ),
};
