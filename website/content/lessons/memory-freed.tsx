import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, H2, P } from "./parts";

// ---------- When Memory Is Freed ----------

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
      <P>
        <C>_</C> names nothing, so a value given to it drops at the end of its statement, like a value made without a
        name. That is almost never what was meant, so a warning says so. A name that starts with <C>_</C>, such as{" "}
        <C>_guard</C>, keeps the value to the end of the block:
      </P>
      <Ex name="memory/drop-underscore.warns" />
      <P>
        Leaving a block early, with <C>return</C>, <C>break</C>, <C>continue</C> or <C>fail</C>, drops its values right
        then, the last made first, as at its end:
      </P>
      <Ex name="memory/drop-early" />
      <P>
        A union can implement <C>Drop</C> too, with <C>drop</C> in an <C>extend</C>: it runs for every variant. So can
        a generic type, <C>{"type Box<T>: Drop"}</C>, where every instance runs the same <C>drop</C>:
      </P>
      <Ex name="memory/drop-union" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="tests/conformance/06-memory/drop-given-to-underscore-warns.kav" label="_, tested" />
        <SrcRef file="tests/conformance/06-memory/drop-runs-when-fail-leaves.kav" label="leaving early, tested" />
        <SrcRef file="tests/conformance/06-memory/drop-runs-for-unions.kav" label="unions, tested" />
        <SrcRef file="tests/conformance/06-memory/drop-runs-for-generic-types.kav" label="generic types, tested" />
      </div>
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
