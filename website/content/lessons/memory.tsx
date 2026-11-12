import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import { Terminal } from "@/components/terminal";
import { MemoryShare } from "@/components/memory-share";
import type { LessonDoc } from "@/lib/chapter";
import { loadProject } from "@/lib/samples";
import { specTables } from "@/lib/spec-tables";
import { Ex, H2, Note, P } from "./parts";
import { Table } from "./spec-table";

const pointers = loadProject("memory/pointers.native", ["kavach.toml", "src/main.kav"]);
const leak = loadProject("memory/leak", ["src/main.kav"]);

const levels: [string, string, string, string][] = [
  ["1. Automatic", "nothing", "almost all code", "/docs/memory/automatic-memory/"],
  ["2. Shared state", "Shared<T>", "state several parts of a program change", "/docs/memory/shared-state/"],
  ["3. Arenas", "arena { … }", "many short-lived values in a hot path", "/docs/memory/arenas/"],
  ["4. Unsafe", "unsafe { … }", "raw memory and C", "/docs/memory/unsafe/"],
];

// ---------- Values, Not References ----------

export const valuesDoc: LessonDoc = {
  lead: "Every value in Kavach is a value: assigning or passing it never makes two names share it.",
  simple: <>In Kavach, giving someone a value is like giving them a photocopy, not your original. Whatever they write on their copy, yours stays the same.</>,
  minutes: 4,
  before: ["collections/lists", "types/records"],
  body: (
    <>
      <Ex name="memory/values" />
      <P>
        <C>b = a</C> gives <C>b</C> its own list. Changing <C>b</C> can&apos;t change <C>a</C>. The same holds when you
        pass a value to a function:
      </P>
      <Ex name="memory/values-fn" />
      <Note>
        In Python, Java or JavaScript, <C>b = a</C> makes two names for one list, and a change through one shows up
        in the other. That &quot;who else changed my list?&quot; bug can&apos;t happen here.
      </Note>
      <H2>Four levels of control</H2>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <thead>
            <tr className="border-b border-line text-left text-soft">
              <th className="py-2 pr-4 font-bold">Level</th>
              <th className="py-2 pr-4 font-bold">You write</th>
              <th className="py-2 font-bold">Use it for</th>
            </tr>
          </thead>
          <tbody>
            {levels.map(([level, write, use, href]) => (
              <tr key={level} className="border-b border-line">
                <td className="py-2 pr-4"><Link href={href}>{level}</Link></td>
                <td className="py-2 pr-4"><C>{write}</C></td>
                <td className="py-2 text-muted">{use}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </>
  ),
};

// What is counted and freed, as spec 6.1 lists it.
const counted = () => {
  const t = specTables("06-memory.md", "6.1").find((x) => x.header.join("|") === "Values|Where they live");
  if (!t) throw new Error("spec 6.1: no Values|Where they live table");
  return t;
};

// ---------- Automatic Memory ----------

export const automaticDoc: LessonDoc = {
  lead: "You never allocate or free memory yourself, and there is no garbage collector that pauses your program.",
  simple: <>Memory is the space a program uses to hold its values. Kavach tidies it for you as soon as a value is no longer needed, like a waiter who clears each plate when you finish, instead of stopping everything for a big clean-up.</>,
  minutes: 4,
  before: ["memory/values-not-references"],
  body: (
    <>
      <P>
        Kavach frees a value the moment its last use ends. There is no collector running in the background, so a
        program never stops for one, and memory comes back at a point you can predict.
      </P>
      <P>
        Copying every value would be slow, so the compiler avoids it without you noticing. The sub-pages show how.
      </P>
      <div><SrcRef file="docs/decisions/0003-memory-model.md" label="decision 0003 · the memory model" /></div>
    </>
  ),
};

export const fastDoc: LessonDoc = {
  lead: "Values look copied, but the compiler moves them, shares them and copies only when it must.",
  simple: <>Copying everything would be slow, so Kavach is clever about it. It is like sharing one book until someone wants to write in it, and only then giving that person their own copy.</>,
  minutes: 6,
  body: (
    <>
      <ol className="m-0 flex list-decimal flex-col gap-2 pl-5">
        <li><strong className="text-ink">Moved, not copied.</strong> When a value is used for the last time, it is handed over instead of copied.</li>
        <li><strong className="text-ink">Shared and counted.</strong> A large value is shared inside, with a count of who holds it. The compiler removes most count updates while compiling.</li>
        <li><strong className="text-ink">Copied only on change.</strong> A value is copied only when it is changed while someone else still holds it. If nobody else does, it is changed in place.</li>
      </ol>
      <P>Step through what happens in memory, line by line:</P>
      <MemoryShare />
      <P>Its first lines as a program you can run:</P>
      <Ex name="memory/values" />
      <P>Here <C>b.push(4)</C> is the moment the list is really copied: <C>a</C> still holds the old one.</P>
      <H2>Numbers, text and the rest</H2>
      <P>Not every value is counted. Numbers stay where they are; text, lists and the rest are one object each:</P>
      <Table header={counted().header} rows={counted().rows} wrapFirst />
      <P>
        A holder is anything that keeps a value: a variable, a list or record it is inside, a lambda that uses it, a
        task. A record a function never shares may never become an object at all: its fields stay in place, like
        numbers. A program can&apos;t tell the difference.
      </P>
      <H2>Nested lists and maps</H2>
      <P>
        The same holds at every depth. A list of lists, a list three deep, or a map of lists: change the copy anywhere
        inside, and only the copy changes:
      </P>
      <Ex name="memory/nested" />
      <P>
        Only what changes is copied. When <C>b[1].push(9)</C> runs, the inner list <C>[4, 5]</C> is copied first, and
        9 goes onto the copy. Then the outer list is copied, but only as a list of pointers to the same inner lists.{" "}
        <C>[2, 3]</C> and <C>[5]</C> stay shared by <C>a</C> and <C>b</C>. If nothing else held <C>b</C>, nothing
        would be copied at all: it would change in place.
      </P>
      <P>
        A list holds one type, so <C>[2, 3, [4, 5, [2, 1, 4]], 5]</C>, numbers and lists mixed, is refused:
      </P>
      <Ex name="memory/mixed.fails" />
      <P>
        To nest numbers and lists freely, name the two kinds of item in a{" "}
        <Link href="/docs/types/unions/">union</Link> that holds a list of itself. Data from outside, such as JSON,
        comes as a <Link href="/docs/standard-library/json/json-values/">Json</Link> value, which works the same way:
      </P>
      <Ex name="memory/nested-mixed" />
    </>
  ),
};

// Instead of delete: what to write for each wish (spec 6.1).
const noDelete: { want: string; write: React.ReactNode; why: React.ReactNode }[] = [
  { want: "Free a big value early", write: "just stop using it", why: "it is freed after its last use" },
  { want: "Free it, then keep the name", write: <C>rows = []</C>, why: "a new value drops the old one at once, unless something else still holds it" },
  { want: "Clean up a file, socket or lock at a known point", write: <>put it in an inner <C>{"{ … }"}</C> block</>, why: <>a value with <C>drop</C> lives to the end of its block, then <C>drop</C> runs</> },
  { want: "Free many small values at once", write: <Link href="/docs/memory/arenas/"><C>arena {"{ … }"}</C></Link>, why: "everything made inside goes when the block ends; only its result is copied out" },
  { want: "Manage raw memory yourself, like C", write: <Link href="/docs/memory/unsafe/"><C>free(p)</C> inside <C>unsafe</C></Link>, why: "nothing is checked, as in C" },
];

export const freedDoc: LessonDoc = {
  lead: "A value is freed when its last use ends: no garbage collector, no pauses. A type can run its own cleanup with Drop.",
  simple: <>Like a hotel guest who hands back the key the moment they leave, not some time later. Kavach frees a value right after its last use, and a type can add its own last step, such as closing a file.</>,
  minutes: 7,
  body: (
    <>
      <P>
        Because nothing waits for a collector, files, sockets and locks close at a known point, not &quot;eventually&quot;.
        Long-running servers keep steady response times, and games keep steady frame times.
      </P>
      <H2>How it knows when</H2>
      <P>
        Nothing runs in the background to look for memory that is no longer used. While it compiles, the compiler
        finds the last place each variable is read, and makes that read hand the value over instead of sharing it.
        Each value counts who holds it. When the last holder is done with it, the count reaches zero and the memory
        is freed at that moment. So a value nothing else holds is freed right after its last use:
      </P>
      <Ex name="memory/last-use" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>The last read of <C>rows</C> is the <C>sum</C>: it gets the list, and lets go of it when it is done. Nothing else holds it, so it is freed there, and <C>send</C> runs without it.</li>
        <li>A value that something else still holds, such as a copy kept in a list, is freed when its last holder is done with it.</li>
        <li>Anything a function still holds when it returns is let go there.</li>
        <li>The compiler works out where values are handed over; the count, which decides the moment memory is freed, is kept while the program runs. That is why a copy costs nothing until one of them is changed.</li>
      </ul>
      <H2>Cleanup of your own: Drop</H2>
      <P>
        A type that holds something to give back, such as a file, a connection or a lock, implements <C>Drop</C>.
        Its <C>drop</C> method runs when the block holding the value ends:
      </P>
      <Ex name="memory/drop" />
      <P>When a block holds several, the last one made goes first, as with stacked plates:</P>
      <Ex name="memory/drop-order" />
      <P>
        A value with <C>drop</C> lives until the end of its block even when its name is never read again, so a
        guard protects everything after it:
      </P>
      <Ex name="memory/drop-guard" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>A value made without a name drops at the end of its statement.</li>
        <li>Giving a variable a new value drops the old one there.</li>
        <li>Copies are one value: <C>drop</C> runs once, when the last holder&apos;s block ends.</li>
        <li>
          A panic runs no <C>drop</C>. In the main task it ends the program; in another task, or in an agent&apos;s
          handler, it stops just that, and what it held is never dropped.
        </li>
      </ul>
      <P>A value bound by <C>if let</C> drops at the end of the <C>if let</C>&apos;s block, before the next statement:</P>
      <Ex name="memory/drop-if-let" />
      <H2>Who holds a value</H2>
      <P>
        Freeing can&apos;t be seen in what a program prints, but <C>drop</C> can. Each sample here uses a{" "}
        <C>Note</C> whose <C>drop</C> prints <C>freed …</C>; a value with <C>drop</C> lives to the end of the block
        of its last holder.
      </P>
      <P>
        <strong className="text-ink">An inout parameter</strong> is a copy while the call runs, so the caller&apos;s
        variable still holds the old value. When the function gives it a new value, the old one goes when the call
        returns:
      </P>
      <Ex name="memory/drop-inout" />
      <P>
        <strong className="text-ink">A lambda</strong> that uses a value holds a copy of it, so the value lives as
        long as the lambda&apos;s block, not only to its last call. An interface value made from one holds it the same
        way:
      </P>
      <Ex name="memory/drop-lambda" />
      <Ex name="memory/drop-interface" />
      <P>
        <strong className="text-ink">Nested lambdas</strong> are one more holder each. The inner lambda takes its copy
        from the outer one, and the value goes when the last of them is done:
      </P>
      <Ex name="memory/drop-nested" />
      <P>
        <strong className="text-ink">A list</strong> holds each of its items. An item lives as long as the list holds
        it, and goes with the list, in the order of the list:
      </P>
      <Ex name="memory/drop-list" />
      <P>
        <strong className="text-ink">Shared</strong> is one cell, and every copy of it, in a variable, a lambda, a list
        or a task, is a holder. The cell and its value go when the last holder lets go. Like a value with{" "}
        <C>drop</C>, a <C>Shared</C> lives until the end of the block that holds it. A cycle through <C>Shared</C> is
        never freed, and is reported: see <Link href="/docs/memory/shared-state/">Shared State</Link>.
      </P>
      <Ex name="memory/drop-shared" />
      <H2>Deleting a variable</H2>
      <P>
        There is no <C>delete</C> (C++), <C>del</C> (Python) or <C>= null</C> (JavaScript). The compiler already
        frees a value at the earliest safe moment, so a <C>delete x</C> would only add a way to make mistakes, such as
        using <C>x</C> after deleting it, with nothing gained. To free memory sooner, stop using the value, give the
        variable a new value, or end the block that holds it:
      </P>
      <Ex name="memory/no-delete" />
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <thead>
            <tr className="border-b border-line text-left text-soft">
              <th className="py-2 pr-4 font-bold">You want</th>
              <th className="py-2 pr-4 font-bold">Write</th>
              <th className="py-2 font-bold">Why</th>
            </tr>
          </thead>
          <tbody className="text-muted">
            {noDelete.map((r) => (
              <tr key={r.want} className="border-b border-line align-top">
                <td className="py-2 pr-4 text-ink">{r.want}</td>
                <td className="py-2 pr-4">{r.write}</td>
                <td className="py-2">{r.why}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <P>
        A value with <C>drop</C> is the one exception to &quot;freed after its last use&quot;: it lives until its block
        ends, even after its last use, so a guard such as a lock stays held while the code below it runs.
      </P>
    </>
  ),
};

// ---------- Shared State ----------

export const sharedDoc: LessonDoc = {
  lead: "When several parts of a program must change the same thing, say so with Shared<T>: the one value that is shared.",
  simple: <>Usually everyone gets their own copy. When several parts truly need one thing, like a single scoreboard everyone updates, <C>Shared</C> marks it, and only one part changes it at a time, so no update is lost.</>,
  minutes: 4,
  before: ["memory/values-not-references"],
  body: (
    <>
      <Ex name="memory/shared" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>Shared(0)</C> makes a shared value. <C>update</C> changes it, <C>get</C> reads it.</li>
        <li><C>update</C> holds the value&apos;s lock while its function runs, so updates from many tasks, on any number of cores, never lose one.</li>
        <li>Seeing <C>Shared</C> in code tells every reader: this one is shared.</li>
      </ul>
      <H2>What update gives</H2>
      <P>
        <C>update</C> gives the new value, read while the lock is still held, so no other task can change it in
        between. It may be left unused. <C>update_with</C> takes a function that gives a pair, the new value and a
        result, and gives the result: take something and know what you took, in one step:
      </P>
      <Ex name="memory/update-gives" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/06-memory.md" label="spec 6.2 · shared state" />
        <SrcRef file="tests/conformance/06-memory/shared-update-gives-a-result.kav" label="tested" />
        <SrcRef file="docs/decisions/0095-shared-update-gives-a-result.md" label="decision 0095" />
      </div>
      <H2>The function given to update can&apos;t wait</H2>
      <P>
        While <C>update</C>&apos;s function runs, every other task that wants the value waits for it. A function that
        sleeps, reads a file, calls the network or waits for a task would hold them all up, or lock two tasks against
        each other for good. So a wait there is refused:
      </P>
      <Ex name="memory/update-wait.fails" />
      <P>The fix: get what the update needs first, then update. The lock is held only for the quick change:</P>
      <Ex name="memory/update-first" />
      <P>A wait hidden inside another function can&apos;t be seen while compiling, so it stops the program instead, with the same advice:</P>
      <Ex name="memory/update-through.panics" />
      <H2>A panic inside update</H2>
      <P>
        A panic in another task stops only that task. If it happens inside <C>update</C>&apos;s function, the value
        stays as it was, and other tasks can go on using it:
      </P>
      <Ex name="memory/update-panic" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>The panic is printed to standard error, with its place, so it isn&apos;t in the box above.</li>
        <li>The half-done change is never seen: <C>total</C> is still <C>10</C>, and the next <C>update</C> works.</li>
      </ul>
      <SrcRef file="tests/conformance/09-concurrency/a-panic-inside-update-lets-the-value-go.kav" label="a panic inside update, tested" />
      <H2>Links back: Weak</H2>
      <P>
        A <C>Shared</C> that holds a way back to itself, directly or through others, makes a cycle, and nothing in a
        cycle is freed. <C>Weak&lt;T&gt;</C> points to a <C>Shared</C> without keeping it alive: <C>s.weak()</C> makes
        one, and <C>w.shared()</C> gives the <C>Shared</C> back while it lives, and <C>none</C> once it was freed:
      </P>
      <Ex name="memory/weak" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>A door points back to its room with a <C>Weak</C>, so the room and its doors make no cycle.</li>
        <li>A <C>Shared</C> lives until the end of the block that holds it. <C>hall</C> isn&apos;t read after the update, but lives to the end of <C>main</C>, so <C>back.shared()</C> still finds it. What <C>w.shared()</C> gives depends on the blocks, as written, not on where a <C>Shared</C> was last used.</li>
        <li>A <C>Weak</C> prints as <C>Weak(…)</C>, or <C>Weak(none)</C> once its value was freed.</li>
      </ul>
      <H2>A cycle is reported</H2>
      <P>
        There is no cycle collector: a leaked cycle is a bug to fix. <C>kavach run</C> and <C>kavach test</C> report
        each <C>Shared</C> still alive when the program ends, which only a cycle can keep, with the line that made it:
      </P>
      <CodeBlock code={leak.files[0].code} file="src/main.kav" />
      <Terminal session={leak.output ?? ""} />
      <P>
        The report goes to standard error. A built program does the same when run with{" "}
        <C>KAVACH_CHECK_LEAKS=1</C>, and otherwise pays nothing for it.
      </P>
    </>
  ),
};

// ---------- Arenas ----------

export const arenasDoc: LessonDoc = {
  lead: "An arena makes many short-lived values quickly from one block of memory, and frees them all at once at its end.",
  simple: <>Like paper plates at a big party: you hand out many quickly, then throw them all away together at the end, instead of washing each one.</>,
  minutes: 4,
  before: ["memory/automatic-memory"],
  body: (
    <>
      <Ex name="memory/arena" />
      <P>
        All the words made inside the <C>arena</C> are freed together when it ends; only the block&apos;s value, the
        total, comes out. A value made inside can&apos;t be kept outside, and the compiler checks that:
      </P>
      <Ex name="memory/arena-escape.fails" />
      <Note>Reach for an arena when measuring shows that making and freeing values is the slow part, not by default.</Note>
    </>
  ),
};

// ---------- Unsafe ----------

/** Spec 6.4's table of what a pointer can do. */
const pointerOps = () => {
  const t = specTables("06-memory.md", "6.4").find((x) => x.header[0] === "Operation");
  if (!t) throw new Error("spec 6.4: no table of pointer operations");
  return t;
};

export const unsafeDoc: LessonDoc = {
  lead: "unsafe { … } turns off the safety checks for a block, to work with raw memory and C. A package must allow it.",
  simple: <>Like taking the safety guard off a power tool: you can do special jobs, such as working directly with C, but nothing stops a slip. A package must say it allows this, and every risky place can be listed.</>,
  minutes: 5,
  before: ["memory/automatic-memory"],
  body: (
    <>
      {pointers.files.map((f) => (
        <CodeBlock key={f.path} code={f.code} file={f.path} lang={f.path.endsWith(".toml") ? "toml" : "kavach"} />
      ))}
      <figure className="m-0 rounded-[10px] border border-line px-4 py-[11px]">
        <span className="mb-1.5 block text-[11.5px] font-extrabold uppercase tracking-wider text-soft">Output</span>
        <pre className="m-0 w-0 min-w-full overflow-x-auto whitespace-pre font-mono text-[13px] text-ink">{pointers.output}</pre>
      </figure>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>A <C>Ptr&lt;Int32&gt;</C> is a C pointer. <C>alloc</C> and <C>free</C> are C&apos;s, so C can free what Kavach allocated.</li>
        <li>C&apos;s syntax works inside <C>unsafe</C>: <C>*p</C>, <C>p[1]</C>, <C>p + 2</C>.</li>
        <li>A package uses <C>unsafe</C> only with <C>unsafe = true</C> in its <C>kavach.toml</C>, and <C>kavach audit</C> lists every place.</li>
      </ul>
      <H2>Every operation</H2>
      <Table header={pointerOps().header} rows={pointerOps().rows} />
      <Note>
        Nothing is checked inside <C>unsafe</C>: an index past the end or a pointer used after <C>free</C> is as
        dangerous as in C. Keep it in one small function with a safe signature. Pointers run in native code.
      </Note>
    </>
  ),
};
