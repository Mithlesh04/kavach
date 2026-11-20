import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, Files, H2, P } from "./parts";

// ---------- Frozen Values ----------
//
// Spec 2.20 (decision 0141): `x.freeze()` gives a `Frozen<T>` that never
// changes, even in a `let`; `unfreeze()` gives a copy that can.

const SPEC = "docs/spec/02-types/20-frozen-values.md";
const TESTS = "tests/conformance/02-types";

export const frozenDoc: LessonDoc = {
  lead: "x.freeze() gives a Frozen<T>: a value whose fields and items never change, even in a let, all the way down. unfreeze() gives a copy that can change.",
  simple: <>Freezing a value is like laminating a printed page: you can still read every word, but nobody can write on it. To change it, you make a fresh copy.</>,
  minutes: 7,
  before: ["basics/variables", "types/records", "memory/values-not-references"],
  body: (
    <>
      <Ex name="frozen/basics" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>freeze</C> and <C>unfreeze</C> are methods every value has: lists, dicts, records, and types a library didn&apos;t design. They need no <C>use</C>.</li>
        <li>
          Values are already copies (<Link href="/docs/memory/values-not-references/">values, not references</Link>), so no
          other code changes what a variable holds. Freezing adds that the code holding the value can&apos;t change it
          either, so a library can hand out a value and know it stays as it was made.
        </li>
        <li>
          For one field of a type the library declares, <C>pub(read)</C> does the same:{" "}
          <Link href="/docs/tools/modules/pub/">fields others only read</Link>.
        </li>
      </ul>
      <H2>Reading and changing</H2>
      <P>
        Reading works as on a <C>T</C>: fields, items, the methods that don&apos;t change it, <C>for</C>,{" "}
        <C>print</C>, comparisons and <C>==</C>. Every change is a compile error that says to call{" "}
        <C>unfreeze()</C>, even in a <C>let</C>:
      </P>
      <Ex name="frozen/item.fails" />
      <Ex name="frozen/change.fails" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Refused the same way: <C>+=</C> and the other <C>op=</C>, and passing the value, or a part of it, to an <C>inout</C> parameter.</li>
        <li>A part read out, such as <C>order.lines</C>, is a plain copy of the part.</li>
      </ul>
      <P>
        A <C>let</C> holding a frozen value may be given another frozen value of its type: the variable is still a
        variable. <C>with</C> makes a new value, and it isn&apos;t frozen:
      </P>
      <Ex name="frozen/let-again" />
      <P>A plain value doesn&apos;t go where a <C>Frozen&lt;T&gt;</C> is written, so the code shows where a value becomes frozen:</P>
      <Ex name="frozen/plain.fails" />
      <P>Freezing a frozen value, and unfreezing a plain one, are compile errors too:</P>
      <Ex name="frozen/twice.fails" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={`${TESTS}/a-frozen-value-reads-as-its-type.kav`} label="reading, tested" />
        <SrcRef file={`${TESTS}/a-frozen-list-cant-change-in-place.kav`} label="no change in place, tested" />
        <SrcRef file={`${TESTS}/a-frozen-let-may-hold-another-frozen-value.kav`} label="a let, tested" />
        <SrcRef file={`${TESTS}/with-on-a-frozen-record-gives-a-plain-value.kav`} label="with, tested" />
        <SrcRef file={`${TESTS}/unfreeze-gives-a-copy-that-can-change.kav`} label="unfreeze, tested" />
      </div>
      <H2>Nested values</H2>
      <P><C>freeze()</C> freezes the whole value, every list, dict and record inside it:</P>
      <Ex name="frozen/deep.fails" />
      <P>
        <C>freeze(nested: false)</C> freezes only the top level. Its own fields and items can&apos;t be assigned, and
        nothing is added or taken away, but the values inside may still change in place:
      </P>
      <Ex name="frozen/nested" />
      <Ex name="frozen/top-level.fails" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>nested:</C> is written <C>true</C> or <C>false</C>, as it is part of the value&apos;s type.</li>
        <li>A <C>Frozen&lt;T&gt;</C> written in a signature is frozen all the way down; one frozen only at its top level fits there too.</li>
        <li><C>unfreeze()</C> gives a copy that can change all the way down.</li>
      </ul>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={`${TESTS}/freeze-is-deep-by-default.kav`} label="deep, tested" />
        <SrcRef file={`${TESTS}/nested-false-freezes-only-the-top-level.kav`} label="nested: false, tested" />
      </div>
      <H2>Passing a frozen value</H2>
      <P>
        A <C>Frozen&lt;T&gt;</C> goes where a <C>T</C> is written only as an argument, and only to a function that{" "}
        <strong>only reads</strong> that parameter. The compiler works that out from the body, so reading functions
        stay plain and still take frozen values:
      </P>
      <Ex name="frozen/passing" />
      <P>
        A function only reads a parameter when every use of it reads from it: <C>.field</C>, <C>[i]</C>, a method
        that doesn&apos;t change it, <C>for</C>, an operator, text, <C>print</C>, or passing it on to another function
        that only reads. Keeping it in a <C>let</C>, a record or a list, or giving it back, is not only reading:
      </P>
      <Ex name="frozen/keep.fails" />
      <P>
        Where a type is worked out, as in <C>let copy = order</C> or a generic function given a frozen value, the
        type worked out is the frozen one, so nothing is lost:
      </P>
      <Ex name="frozen/worked-out.fails" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={`${TESTS}/a-frozen-value-goes-to-a-function-that-only-reads.kav`} label="only reads, tested" />
        <SrcRef file={`${TESTS}/a-function-that-keeps-a-parameter-refuses-a-frozen-value.kav`} label="keeps it, tested" />
        <SrcRef file={`${TESTS}/a-frozen-value-is-not-a-plain-value-elsewhere.kav`} label="elsewhere, tested" />
      </div>
      <P>
        A library can also seal a frozen value with a key, so only the library unfreezes it:{" "}
        <Link href="/docs/types/frozen-values/sealed-with-a-key/">Sealed with a key</Link>. Coming from JavaScript?{" "}
        <Link href="/docs/start/coming-from-another-language/javascript/">Object.freeze compared</Link>.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={SPEC} label="spec 2.20 · frozen values" />
        <SrcRef file="docs/decisions/0141-freeze.md" label="decision 0141" />
        <SrcRef file="compiler/checker/src/collect.rs" line={461} symbol="Frozen" />
      </div>
    </>
  ),
};

export const frozenKeyDoc: LessonDoc = {
  lead: "x.freeze(key: k) seals a value: only unfreeze(key: k), with an equal key, opens it. A library makes the key a type of its own, so only the library unfreezes what it froze.",
  simple: <>A sealed value is like a locked display case: everyone can look inside, but only the shop that has the key can open it.</>,
  minutes: 5,
  before: ["types/frozen-values", "tools/modules/pub"],
  body: (
    <>
      <P>
        Here <C>session</C> hands out sessions that other modules read but never change. <C>SessionKey</C> has no{" "}
        <C>pub</C>, so no other module can name it, and only <C>session</C> can unfreeze a session:
      </P>
      <Files name="frozen/key" first={["src/session.kav", "src/main.kav"]} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>The key&apos;s type is part of the frozen value&apos;s type, written second: <C>Frozen&lt;Session, SessionKey&gt;</C>.</li>
        <li>A type that is <C>pub</C> but has a private field can&apos;t be made outside its module either, so it seals too.</li>
        <li>A sealed value carries its key while the program runs. A value frozen without a key is the plain value, and freezing and unfreezing it costs nothing.</li>
      </ul>
      <H2>A wrong key</H2>
      <P>
        <strong>While compiling, by its type.</strong> When the key&apos;s type is private to its module, code in any
        other module that calls <C>unfreeze(key: …)</C> is a compile error that names the module, even with a key of
        that type:
      </P>
      <Files name="frozen/key-elsewhere.fails" first={["src/main.kav", "src/session.kav"]} outcome="refused" />
      <P><C>unfreeze()</C> without a key on a sealed value is a compile error too:</P>
      <Ex name="frozen/no-key.fails" />
      <P>
        <strong>While running, by its value.</strong> A key of the right type that isn&apos;t equal (<C>==</C>) to
        the one the value was sealed with is a panic:
      </P>
      <Ex name="frozen/wrong-key.panics" />
      <H2>Matching a sealed value</H2>
      <P>
        A <C>match</C> reads a sealed value&apos;s parts in its arms, or names the whole value in every arm, which
        keeps it sealed. Both in one <C>match</C> is a compile error, as the name would hold the value without its key:
      </P>
      <Ex name="frozen/match" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={`${TESTS}/a-key-seals-a-frozen-value.kav`} label="a key, tested" />
        <SrcRef file={`${TESTS}/a-private-key-seals-for-other-modules/src/main.kav`} label="other modules, tested" />
        <SrcRef file={`${TESTS}/a-sealed-value-needs-its-key.kav`} label="needs its key, tested" />
        <SrcRef file={`${TESTS}/a-wrong-key-panics.kav`} label="a wrong key, tested" />
        <SrcRef file={`${TESTS}/a-match-reads-a-frozen-union.kav`} label="match, tested" />
        <SrcRef file={SPEC} label="spec 2.20 · sealed with a key" />
      </div>
    </>
  ),
};
