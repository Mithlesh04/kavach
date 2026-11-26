import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import { Terminal } from "@/components/terminal";
import { MemorySteps } from "@/components/memory-steps";
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
  ["3. Arenas", "arena { … }", "many short-lived values, freed together", "/docs/memory/arenas/"],
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
        in the other. That &quot;who else changed my list?&quot; bug can&apos;t happen here. Each case, side by side:{" "}
        <Link href="/docs/memory/values-not-references/coming-from-javascript-and-python/">coming from JavaScript and Python</Link>.
      </Note>
      <P>
        Does <C>b = a</C> take new memory, then? No: inside, <C>b</C> shares <C>a</C>&apos;s object until one of
        them changes, so nothing is copied until it must be. Every case is on{" "}
        <Link href="/docs/memory/automatic-memory/memory-step-by-step/">Memory, Step by Step</Link>; here is the
        moment a copy is made:
      </P>
      <MemorySteps name="copy" />
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
        The compiler finds each last use while compiling (<Link href="/docs/memory/automatic-memory/freed-immediately/#how-it-knows-when">how it knows when</Link>).
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
        <li><strong className="text-ink">Copied only on change.</strong> A value is copied only when it is changed while someone else still holds it. If nobody else does, it is changed in place. So <C>text += part</C> or <C>items.push(x)</C> in a loop copies only what is added, not the whole value each time.</li>
        <li><strong className="text-ink">Chains fused into one loop.</strong> A chain such as <C>xs.map(f).filter(g).sum(h)</C> may run as one loop that builds no list between its steps. Its value is the one the steps give in turn, over the list as it was when the chain ran.</li>
      </ol>
      <P>
        <strong className="text-ink">No optimization changes what a program does.</strong> Moving, sharing, changing
        in place, fusing loops, keeping a record in registers or spreading work over cores gives the same output, the
        same panics at the same lines, and the same values in every copy as running each step on its own copies
        would. Here a fused chain&apos;s value stays as it was after the list changes:
      </P>
      <Ex name="memory/fused-chain" />
      <P>
        This is checked, not hoped for: native code and the interpreter run every conformance test and must agree, and
        the memory tests change one copy through each optimized path, then check the others.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/06-memory.md" label="spec 6.1 · how it stays fast" />
        <SrcRef file="tests/conformance/06-memory/a-fused-chain-reads-the-list-as-it-was.kav" label="fused chain, tested" />
        <SrcRef file="tests/conformance/06-memory/a-record-copy-in-a-loop-is-its-own.kav" label="record copy in a loop" />
        <SrcRef file="tests/conformance/06-memory/a-slice-is-its-own-copy.kav" label="slice copy" />
        <SrcRef file="tests/conformance/06-memory/setting-a-slice-leaves-earlier-copies.kav" label="setting a slice" />
        <SrcRef file="tests/conformance/06-memory/a-callback-field-copy-is-its-own.kav" label="callback field copy" />
        <SrcRef file="tests/conformance/06-memory/a-tensor-product-is-its-own.kav" label="tensor product" />
      </div>
      <P>
        Step through what happens in memory, line by line. Every case, for every type, is on{" "}
        <Link href="/docs/memory/automatic-memory/memory-step-by-step/">Memory, Step by Step</Link>:
      </P>
      <MemorySteps name="copy" />
      <P>Here <C>b.push(6)</C> is the moment the list is really copied: <C>a</C> still holds the old one.</P>
      <P>
        Sharing makes comparing quick too: before the copy, <C>a == b</C> is <C>true</C> without looking at the items
        (see <Link href="/docs/basics/operators/comparison/#contents-not-places">Comparison</Link>).
      </P>
      <H2>Growing in a loop</H2>
      <P>
        Text built up in a loop has one holder, so each <C>+=</C> adds to it in place. A copy taken before keeps its
        old value:
      </P>
      <Ex name="memory/grows-in-place" />
      <SrcRef file="tests/conformance/06-memory/text-grows-in-place-copies-stay.kav" label="tested" />
      <P>
        The same holds for a list or dict in a field or in another list, and through{" "}
        <Link href="/docs/functions/inout-parameters/">inout</Link>: <C>cart.items.push(x)</C>,{" "}
        <C>cart.counts[k] = v</C> and <C>shelves[i].push(x)</C> change it where it is. A copy taken before keeps
        what it had:
      </P>
      <Ex name="memory/field-grows" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="tests/conformance/06-memory/a-list-in-a-field-grows-in-place.kav" label="lists, tested" />
        <SrcRef file="tests/conformance/06-memory/a-map-in-a-field-changes-in-place.kav" label="dicts and inout, tested" />
      </div>
      <H2>Numbers, text and the rest</H2>
      <P>Not every value is counted. Numbers stay where they are; text, lists and the rest are one object each:</P>
      <Table header={counted().header} rows={counted().rows} wrapFirst />
      <P>
        A holder is anything that keeps a value: a variable, a list or record it is inside, a lambda that uses it, a
        task. A record a function never shares may never become an object at all: its fields stay in place, like
        numbers. A program can&apos;t tell the difference.
      </P>
      <H2>Why numbers aren&apos;t shared</H2>
      <P>
        <C>let t = s</C> shares text instead of copying it, so why does <C>let b = a</C> copy a number? Because a
        number is as small as the address that would point to it. Think of a phone number on a sticky note: writing
        the number itself on your note is quicker than writing &quot;see the note in drawer 7&quot; and walking to
        the drawer every time you need it.
      </P>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <thead>
            <tr className="border-b border-line text-left text-soft">
              <th className="py-2 pr-4 font-bold"><C>b = a</C>, where <C>a</C> is…</th>
              <th className="py-2 pr-4 font-bold">A number, copied</th>
              <th className="py-2 font-bold">Text or a list, shared</th>
            </tr>
          </thead>
          <tbody className="text-muted">
            <tr className="border-b border-line"><td className="py-2 pr-4 text-ink">What is copied</td><td className="py-2 pr-4">the number: 8 bytes</td><td className="py-2">its address: 8 bytes</td></tr>
            <tr className="border-b border-line"><td className="py-2 pr-4 text-ink">Memory it needs</td><td className="py-2 pr-4">those 8 bytes</td><td className="py-2">the address, and an object: a 24-byte header with the count, then the value</td></tr>
            <tr className="border-b border-line"><td className="py-2 pr-4 text-ink">Reading it</td><td className="py-2 pr-4">at once, from where it is</td><td className="py-2">follow the address to memory first</td></tr>
            <tr className="border-b border-line"><td className="py-2 pr-4 text-ink">Changing <C>b</C></td><td className="py-2 pr-4">just change it</td><td className="py-2">check the count first, and copy if someone else holds it</td></tr>
            <tr className="border-b border-line"><td className="py-2 pr-4 text-ink">When it is no longer used</td><td className="py-2 pr-4">nothing to free</td><td className="py-2">count down, and free at 0</td></tr>
          </tbody>
        </table>
      </div>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Sharing a number would still copy 8 bytes, the address, and then add an object, a count and a trip to memory on every read. It costs more and saves nothing.</li>
        <li>Text, lists and dicts can be any size. Copying a long text means copying all of it; sharing it copies 8 bytes. That is why they are shared until one side changes.</li>
        <li>The rule: <strong>as small as an address, copied; bigger, shared</strong>. <C>Bool</C> and <C>Char</C> are copied like numbers.</li>
        <li>
          Short text follows the same idea inside a list: in a built program, text of up to 15 bytes is kept right in
          the list&apos;s item, with no object and no count.
        </li>
      </ul>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/06-memory.md" label="spec 6.1 · where values live" />
        <SrcRef file="runtime/native/src/heap.rs" label="the 24-byte header" />
        <SrcRef file="runtime/native/src/text_items.rs" label="short text in a list" />
      </div>
      <P>What others say, if you want to read more:</P>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>
          <strong className="text-ink">Python</strong> makes every number an object with a count: at least 28 bytes
          for one whole number, and it keeps -5 to 256 ready-made to soften the cost.{" "}
          <a href="https://towardsdatascience.com/why-does-an-integer-need-28-bytes-in-python-e8adf91f821f/">Why an integer needs 28 bytes</a>
          {" · "}
          <a href="https://arpitbhayani.me/blogs/python-caches-integers">Python&apos;s integer cache</a>
        </li>
        <li>
          <strong className="text-ink">Java</strong>&apos;s boxed <C>Integer</C> takes about 16 bytes where an{" "}
          <C>int</C> takes 4, and reading it takes two trips to memory instead of one.{" "}
          <a href="https://riptutorial.com/java/example/10864/memory-and-computational-overhead-of-autoboxing">The cost of autoboxing</a>
          {" · "}
          <a href="https://dzone.com/articles/whats-wrong-with-java-boxed-numbers">What&apos;s wrong with boxed numbers</a>
        </li>
        <li>
          <strong className="text-ink">Java&apos;s Project Valhalla</strong> is adding value classes so that more
          types can be copied freely like numbers, &quot;just their bits&quot;, with no object identity.{" "}
          <a href="https://openjdk.org/projects/valhalla/design-notes/state-of-valhalla/02-object-model">State of Valhalla</a>
        </li>
        <li>
          <strong className="text-ink">JavaScript&apos;s V8</strong> keeps small whole numbers right in the value,
          &quot;not allocated in the heap&quot;, as Kavach does.{" "}
          <a href="https://chromium.googlesource.com/v8/v8.git/+/refs/heads/main/src/objects/smi.h">V8&apos;s smi.h</a>
          {" · "}
          <a href="https://medium.com/fhinkel/v8-internals-how-small-is-a-small-integer-e0badc18b6da">How small is a small integer?</a>
        </li>
        <li>
          <strong className="text-ink">Rust</strong>&apos;s advice is to pass small values by copy, and its linter,
          Clippy, warns when a tiny value is passed by reference instead.{" "}
          <a href="https://users.rust-lang.org/t/what-is-faster-copy-or-passing-reference/80733">Copy or reference?</a>
          {" · "}
          <a href="https://rust-lang.github.io/rust-clippy/master/index.html#trivially_copy_pass_by_ref">Clippy: trivially_copy_pass_by_ref</a>
        </li>
        <li>
          <strong className="text-ink">Counting across threads</strong> gets slow when many cores update one count,
          as they fight over the same piece of cache. Numbers that are copied have no count to fight over.{" "}
          <a href="https://blocklisted.github.io/blog/arc_str_vs_string_is_it_really_faster">Arc&lt;str&gt; vs String, measured</a>
        </li>
      </ul>
      <H2>Nested lists and maps</H2>
      <P>
        The same holds at every depth. A list of lists, a list three deep, or a dict of lists: change the copy anywhere
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

// ---------- Shared State ----------

export const sharedDoc: LessonDoc = {
  lead: "When several parts of a program must change the same thing, say so with Shared<T>: the one value that is shared.",
  simple: <>Usually everyone gets their own copy. When several parts truly need one thing, like a single scoreboard everyone updates, <C>Shared</C> marks it, and only one part changes it at a time, so no update is lost.</>,
  minutes: 4,
  before: ["memory/values-not-references"],
  body: (
    <>
      <Ex name="memory/shared" />
      <P>In memory, a <C>Shared</C> is one object that both names hold, so a change through one is seen through the other:</P>
      <MemorySteps name="shared" />
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
  lead: "An arena frees every value made inside it together, when it ends. Only its block's value comes out.",
  simple: <>Like paper plates at a big party: you hand out many, then throw them all away together at the end, instead of washing each one.</>,
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
      <H2>What may leave</H2>
      <P>
        Numbers, <C>Bool</C> and <C>Char</C> are copied, so they may leave by any way, and so may new text built
        only from them. Here the longest word&apos;s length goes out, and so does <C>summary</C>, as only{" "}
        <C>words.len()</C> in it comes from the arena:
      </P>
      <Ex name="memory/arena-numbers" />
      <P>
        A value that holds memory and is made from a variable declared in the arena may not: the variable itself, a
        part of it, a call&apos;s result on it, or a lambda or task that uses it. It can&apos;t be assigned or stored
        into anything declared before the arena (a variable, a list, a <C>Shared</C>, an <C>inout</C> argument),
        nor leave with <C>return</C> or <C>fail</C>:
      </P>
      <Ex name="memory/arena-return.fails" />
      <Ex name="memory/arena-lambda.fails" />
      <P>That holds for every arena the code is in, not only the innermost:</P>
      <Ex name="memory/arena-nested.fails" />
      <H2>Tasks in an arena</H2>
      <P>
        A task spawned in an arena may use its values. When the arena reaches its end, it waits for every task
        spawned in it, then frees its values, so a task never sees them freed:
      </P>
      <Ex name="memory/arena-tasks" />
      <P>
        A function&apos;s leftover tasks are <Link href="/docs/concurrency/tasks-never-leak/">cancelled</Link> when it
        returns; an arena&apos;s are waited for, as their work is usually what the arena is for.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/06-memory.md" label="spec 6.3 · arenas" />
      </div>
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
