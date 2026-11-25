import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, H2, P } from "./parts";

// ---------- Records › Updating with with ----------
//
// Spec 2.4 "Updating a record": `value with { field: new }` makes a copy
// with some fields changed. Also spec 2.20 (with on a frozen value gives
// one that isn't) and 10.1 (pub(read) fields can't be set with with outside
// their module). On an anonymous record it isn't built yet (to-do 30), so no
// sample runs it there.

const SPEC = "docs/spec/02-types/04-records.md";

export const withDoc: LessonDoc = {
  lead: "record with { field: value } makes a copy of a record with some fields changed. The record you started from stays as it was.",
  simple: <>Like photocopying a filled-in form and changing one box on the copy: you get a new form, and the original stays as it was.</>,
  minutes: 5,
  before: ["types/records", "memory/values-not-references"],
  body: (
    <>
      <Ex name="records/sub-record" />
      <P>
        <C>asha with {"{ age: 32 }"}</C> is a new <C>User</C>, and <C>asha</C> is still 31. The fields you don&apos;t
        name are copied as they are. Through a <C>let</C>, you can also assign a field directly, as{" "}
        <C>copy.city = &quot;Delhi&quot;</C> does: that changes only <C>copy</C>.
      </P>
      <H2>Why it exists</H2>
      <P>
        Records are values: passing one to a function or storing it somewhere gives a copy, never a shared object (
        <Link href="/docs/memory/values-not-references/">Values, Not References</Link>). So a function can&apos;t
        change the record it was given. It gives back a new one instead, and <C>with</C> writes that new one in a
        line: the fields that change, and nothing else.
      </P>
      <H2>New state from old</H2>
      <P>
        The usual shape: a function takes the state as it is and gives back the next state. The caller keeps the
        latest one in a <C>let</C>. The same shape runs a game loop, a sound&apos;s next sample, or a running total:
      </P>
      <Ex name="records/with-state" />
      <P>
        Each new value of a field can use the old ones, as <C>t.total + n</C> does: every value on the right is read
        from <C>t</C> as it was before the change.
      </P>
      <H2>Several fields, and fields inside fields</H2>
      <P>
        Name as many fields as you need, separated by commas. A field that is itself a record takes a{" "}
        <C>with</C> of its own:
      </P>
      <Ex name="records/with-nested" />
      <H2>with or an assignment?</H2>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <thead>
            <tr className="border-b border-line text-left text-soft">
              <th className="py-2 pr-4 font-bold">You want</th>
              <th className="py-2 font-bold">Write</th>
            </tr>
          </thead>
          <tbody className="text-muted">
            <tr className="border-b border-line align-top"><td className="py-2 pr-4 text-ink">A changed copy, the original kept</td><td className="py-2"><C>const older = asha with {"{ age: 32 }"}</C></td></tr>
            <tr className="border-b border-line align-top"><td className="py-2 pr-4 text-ink">To give back the next state from a function</td><td className="py-2"><C>{"s with { phase: phase }"}</C> as its result</td></tr>
            <tr className="border-b border-line align-top"><td className="py-2 pr-4 text-ink">To change your own variable, field by field</td><td className="py-2">a <C>let</C> and <C>user.age = 32</C></td></tr>
            <tr className="border-b border-line align-top"><td className="py-2 pr-4 text-ink">The caller&apos;s variable changed by a function</td><td className="py-2">an <Link href="/docs/functions/inout-parameters/">inout</Link> parameter</td></tr>
          </tbody>
        </table>
      </div>
      <H2>What it checks</H2>
      <P>Only fields the type has, with values of their types. A field it doesn&apos;t have is a compile error:</P>
      <Ex name="records/with-unknown.fails" />
      <P>
        The copy is a new value of the type, so its <Link href="/docs/errors/contracts/invariant/">invariants</Link> are
        checked as for any new value. A <C>with</C> that breaks one panics:
      </P>
      <Ex name="records/with-invariant.panics" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>On a <Link href="/docs/types/frozen-values/">frozen</Link> value, <C>with</C> gives a value that isn&apos;t frozen, since it is a new value.</li>
        <li>A <Link href="/docs/tools/modules/pub/#fields-others-only-read-pub-read">pub(read)</Link> field can&apos;t be set with <C>with</C> outside the module that declares its type, as it can&apos;t be assigned there.</li>
      </ul>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={SPEC} label="spec 2.4 · updating a record" />
        <SrcRef file="docs/spec/02-types/20-frozen-values.md" label="spec 2.20 · with on a frozen value" />
      </div>
    </>
  ),
};
