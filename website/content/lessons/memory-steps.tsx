import Link from "next/link";
import { C } from "@/components/inline-code";
import { MemoryLayouts } from "@/components/memory-layouts";
import { MemorySteps } from "@/components/memory-steps";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { H2, Note, P } from "./parts";

// ---------- Automatic Memory › Memory, Step by Step ----------
//
// Spec 6.1 ("How it stays fast", "Freed immediately") as pictures, one per
// line. The frames are in components/memory-scenarios.ts; each animation's
// code and output come from a checked sample, memory/step-*.kav.

const SPEC = "docs/spec/06-memory.md";

export const memoryStepsDoc: LessonDoc = {
  lead: "Does let b = a take new memory? Step through what memory holds at each line: sharing, moving, copying on change, changing in place, for numbers, text, lists, dicts, sets and Shared.",
  simple: <>Memory is where a program keeps its values. These pictures show its cells line by line, as on a real memory chip. Each value sits in cells, and a count says how many names hold it. When the count reaches 0, those cells are cleared for reuse.</>,
  minutes: 10,
  before: ["memory/values-not-references", "memory/automatic-memory/how-it-stays-fast"],
  body: (
    <>
      <P>
        Each picture is memory as it is: a memory stick, and one chip&apos;s cells close up, 8 bytes each. The{" "}
        <strong>stack</strong>&apos;s cells hold the function&apos;s names: a number sits right in its cell, a list
        or text holds the address of its object. The <strong>heap</strong>&apos;s cells hold the objects, each
        starting with its <strong>holder count</strong>: how many names, lists or other values hold it. Press Play,
        or step with Next.
      </P>
      <Note>
        The pictures follow spec 6.1 and how today&apos;s compiler and runtime lay values out. A program can&apos;t
        see a count, so a picture can&apos;t be checked by running it; what each program prints is checked.
      </Note>
      <H2>Does b = a take new memory?</H2>
      <P>
        No. <C>b = a</C> never allocates. Values are copies to your code (
        <Link href="/docs/memory/values-not-references/">values, not references</Link>), but in memory, <C>b</C>{" "}
        holds the same object, and its count goes up:
      </P>
      <MemorySteps name="share" />
      <P>
        If <C>a</C> is never used again, the compiler makes it a <strong>move</strong>: <C>b</C> takes the object,
        the count stays 1, and the name <C>a</C> just ends.
      </P>
      <MemorySteps name="move" />
      <H2>How does the program know it&apos;s the last use?</H2>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>The compiler knows, not the running program. While compiling, it reads each function backwards and finds the last read of each name.</li>
        <li>It turns that read into a hand-over (a move), or puts a let-go after it: count − 1, freed at 0. That step is already in the machine code, so while running nothing searches: the step just runs.</li>
        <li>In a loop, a name read again in the next round isn&apos;t at its last use until the loop is done. The code panels on this page mark each last use.</li>
      </ul>
      <div className="flex flex-wrap gap-2.5">
        <Link href="/docs/memory/automatic-memory/freed-immediately/#how-it-knows-when">Freed immediately: how it knows when</Link>
        <SrcRef file="compiler/kir/src/moves.rs" line={12} symbol="liveness" />
      </div>
      <P>
        <C>const</C> and <C>let</C> are the same in memory. <C>const</C> doesn&apos;t freeze anything there; it
        only means that name can&apos;t change.
      </P>
      <H2>Where each type lives</H2>
      <MemoryLayouts />
      <H2>Numbers</H2>
      <P>A number is never an object, so <C>b = a</C> copies its 8 bytes, and a change to <C>b</C> leaves <C>a</C>:</P>
      <P>
        Why not share a number, as text is shared? A number is as small as the address that would point to it, so
        sharing would save nothing. See{" "}
        <Link href="/docs/memory/automatic-memory/how-it-stays-fast/#why-numbers-aren-t-shared">why numbers aren&apos;t shared</Link>.
      </P>
      <MemorySteps name="numbers" />
      <H2>Records that never become objects</H2>
      <P>
        A record is one object with a count, like a list. But a record of numbers that a function only builds, copies
        and reads or sets one field at a time never needs to be one: native code splits it into a local per field, as
        if each field were a number of its own:
      </P>
      <MemorySteps name="record" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Every field must be a number, <C>Bool</C> or <C>Char</C>, and the type must have no <C>Drop</C>.</li>
        <li>Any other use keeps it whole: passed to a function, returned, kept in a list, dict or record, captured by a lambda, sent to a task, or printed or compared whole.</li>
        <li>Small functions are inlined first, so a record passed to one may still be split.</li>
        <li>This is what the compiler does today, not a promise: the spec says such a record <em>may</em> never become an object. Either way, the program gives the same output.</li>
      </ul>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="compiler/kir/src/sroa.rs" line={5} label="sroa.rs · when a record is split" />
        <SrcRef file="compiler/kir/src/sroa.rs" line={39} symbol="field_types" />
        <SrcRef file="compiler/kir/src/passes.rs" line={108} label="passes.rs · after inlining" />
      </div>
      <H2>A change while shared: copied then</H2>
      <P>
        A value is copied only when it changes <em>while</em> something else still holds it. This is{" "}
        <strong>copy-on-write</strong>:
      </P>
      <MemorySteps name="copy" />
      <P>Text behaves exactly as a list does:</P>
      <MemorySteps name="text" />
      <H2>A change with no other holder: in place</H2>
      <P>
        When nothing else holds a value, a change is made where it is, with no copy. So <C>items.push(x)</C> or{" "}
        <C>text += part</C> in a loop adds only the new part each round:
      </P>
      <MemorySteps name="loop" />
      <P>
        The same holds for a list or dict in a field or in another list, and through <C>inout</C>:{" "}
        <C>cart.items.push(x)</C> and <C>rows[i].push(x)</C> change it where it is (
        <Link href="/docs/memory/automatic-memory/how-it-stays-fast/">how it stays fast</Link>).
      </P>
      <H2>Dicts and sets</H2>
      <P>
        A dict keeps its entries in the order they were added, each key and value in place. Past 8 entries, it keeps
        an index beside them:
      </P>
      <MemorySteps name="dict" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>A dict with whole-number keys keeps an index from the start. Its keys first use a cheap hash that keeps nearby keys nearby; if probes get long, by accident or by attack, it switches to a strong hash for good.</li>
        <li>The hash&apos;s seed is chosen at random each run, so no one can work out in advance which keys collide.</li>
        <li>A value inside, such as the list in <C>{'{"a": [1, 2]}'}</C>, is an object of its own that the dict holds. A copy of the dict shares that list too, until one side changes it.</li>
        <li>A dict with a limit, <C>{"{String: Int}<max: 100>"}</C>, makes room for all its entries at once, so filling it never stops to grow.</li>
      </ul>
      <P>Copying a dict works as for a list: shared until one side changes it.</P>
      <MemorySteps name="dict-copy" />
      <P>A set is the same table with keys only:</P>
      <MemorySteps name="set" />
      <H2>Shared: two names, one changing value</H2>
      <P>
        <Link href="/docs/memory/shared-state/">Shared</Link> is the one way two names see one value change. Copying
        it copies the handle, not the value:
      </P>
      <MemorySteps name="shared" />
      <H2>Frozen values and freeing</H2>
      <P>Freezing a value, and holding it in a <C>const</C>, allocate nothing and copy nothing:</P>
      <MemorySteps name="freeze" />
      <div><SrcRef file="compiler/driver/src/frozen.rs" line={7} label="frozen.rs · x.freeze() becomes x" /></div>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>x.freeze()</C> with no key is the same object, so freezing and unfreezing it costs nothing. A value sealed with <C>freeze(key: k)</C> is the pair <C>(value, key)</C> (<Link href="/docs/types/frozen-values/">frozen values</Link>).</li>
        <li>An object is freed the moment its count reaches 0, when its last use ends. There is no garbage collector and no pause later.</li>
        <li>A value whose type implements <C>Drop</C> lives to the end of its block, so its cleanup runs at a place you can see (<Link href="/docs/memory/automatic-memory/freed-immediately/">freed immediately</Link>).</li>
      </ul>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={SPEC} label="spec 6.1 · how it stays fast" />
        <SrcRef file="docs/decisions/0003-memory-model.md" label="decision 0003 · the memory model" />
        <SrcRef file="runtime/native/src/heap.rs" label="objects and counts" />
        <SrcRef file="runtime/native/src/table.rs" label="dicts and sets" />
        <SrcRef file="runtime/native/src/text_items.rs" label="text in a list" />
        <SrcRef file="docs/architecture/flat-lists.md" label="flat lists" />
        <SrcRef file="docs/architecture/hash-maps.md" label="hash maps" />
      </div>
    </>
  ),
};
