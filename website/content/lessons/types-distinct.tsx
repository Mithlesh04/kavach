import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, H2, Note, P } from "./parts";
import { WhatIsTested } from "./tested-table";

// ---------- Distinct Types ----------
//
// Spec 2.21: `type NetId = distinct Int` holds an Int but is never mixed
// with an Int or with another distinct type. Made with NetId(7), read with
// .value, compared, sorted, used as a key, printed and written to JSON as
// the value it holds.

const SPEC = "docs/spec/02-types/21-distinct-types.md";
const TESTS = "tests/conformance/02-types";

export const distinctDoc: LessonDoc = {
  lead: "type NetId = distinct Int makes a new type that holds an Int but is never mixed with an Int or with another distinct type, so ids of different things can't be swapped by mistake.",
  simple: <>Two tickets can both carry the number 7: a cloakroom ticket and a bus seat. A distinct type puts the right label on the number, so the cloakroom never accepts a bus ticket, even though the numbers match.</>,
  minutes: 5,
  before: ["types/records", "basics/data-types"],
  body: (
    <>
      <Ex name="types/distinct" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>NetId(7)</C> makes one from the value it holds; <C>n.value</C> gives the value back.</li>
        <li>It prints as the value it holds: <C>print(n)</C> prints <C>7</C>.</li>
        <li><C>==</C> and <C>!=</C> compare two of the same type, and <C>&lt;</C>, <C>&lt;=</C>, <C>&gt;</C> and <C>&gt;=</C> work when the held value has an order.</li>
        <li>It costs nothing: a <C>NetId</C> is stored and passed as the <C>Int</C> it holds.</li>
      </ul>
      <H2>Never mixed</H2>
      <P>A <C>CellId</C> holds an <C>Int</C> too, but it isn&apos;t a <C>NetId</C>, so passing one where the other belongs is a compile error:</P>
      <Ex name="types/distinct-mixed.fails" />
      <P>Neither is a plain <C>Int</C>. A bare number has to say which kind of id it is:</P>
      <Ex name="types/distinct-int.fails" />
      <H2>Anywhere a type goes</H2>
      <P>
        A distinct type works in lists, dicts and sets (as keys too), optionals, records and generics. A list of them
        sorts by the values they hold, and JSON writes and reads the held value:
      </P>
      <Ex name="types/distinct-uses" />
      <P>
        A <C>let</C> can change one as a whole, <C>s = Seat(4)</C>, or through its value, as <C>s.value += 1</C> does
        above.
      </P>
      <H2>No arithmetic</H2>
      <P>
        Adding two ids rarely means anything, so a distinct type has no arithmetic and none of the held type&apos;s
        methods. Work on <C>.value</C> and make a new one:
      </P>
      <Ex name="types/distinct-math.fails" />
      <H2>What it can hold</H2>
      <P>
        A built-in number type (<C>Int</C>, <C>Float</C>, the sized and range numbers, <C>Decimal</C>,{" "}
        <C>BigInt</C>), <C>Bool</C>, <C>Char</C> or <C>String</C>. For more than one value, declare a{" "}
        <Link href="/docs/types/records/">record</Link>:
      </P>
      <Ex name="types/distinct-list.fails" />
      <P>It is made from the value alone, with no field name:</P>
      <Ex name="types/distinct-make.fails" />
      <H2>No methods yet</H2>
      <P>A distinct type has no methods, interfaces or invariants of its own yet. Write a function that takes one:</P>
      <Ex name="types/distinct-methods.fails" />
      <Note>
        A one-field record, <C>{"type NetId { id: Int }"}</C>, is distinct too, and can have methods and{" "}
        <Link href="/docs/errors/contracts/invariant/">invariants</Link>. But it is a record: native code makes it as an
        object of its own, where a distinct type is just the number.
      </Note>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={SPEC} label="spec 2.21 · distinct types" />
        <SrcRef file={`${TESTS}/distinct-types-hold-a-value.kav`} label="tested" />
        <SrcRef file={`${TESTS}/distinct-types-are-not-mixed.kav`} label="never mixed, tested" />
      </div>
      <WhatIsTested number="2.21" />
    </>
  ),
};
