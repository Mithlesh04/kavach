import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { specTables } from "@/lib/spec-tables";
import { Ex, Files, H2, P, SpecCode } from "./parts";
import { Table } from "./spec-table";

// ---------- Reflection ----------
//
// Spec 14.32: what a type or a module offers, its members. The calls
// table and the types block come from the spec itself.

const FILE = "14-standard-library/32-reflect.md";

function CallsTable() {
  const t = specTables(FILE, "14.32")[0];
  if (!t || !t.rows[0][0].includes("members()")) throw new Error("spec 14.32: the first table no longer lists members()");
  return <Table header={t.header} rows={t.rows} wrapFirst />;
}

export const reflectDoc: LessonDoc = {
  lead: "std.reflect tells a program what a type or a module offers: its members, each a field, method, function, type or constant, with its parameters, what it gives, and where it comes from.",
  simple: <>A type&apos;s members are the things you can write after a dot: <C>order.total</C>, <C>name.upper()</C>. Reflection hands a program that list, so a tool, plugin or library can adapt to code it has never seen.</>,
  minutes: 5,
  before: ["types/records", "types/extend"],
  body: (
    <>
      <P>
        <C>T.members()</C> lists a type&apos;s fields and methods, sorted by name. Each is a <C>Member</C>: its{" "}
        <C>name</C>, its <C>kind</C>, its <C>params</C> and its <C>result</C>:
      </P>
      <Ex name="reflect/members" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>members()</C>, <C>has_member</C> and module <C>members()</C> need no <C>use</C>: the compiler gives them to every type and module.</li>
        <li><C>use std.reflect.{"{MemberKind}"}</C> is only for naming the types, as <C>MemberKind.Method</C> here. They are declared in <C>stdlib/reflect/reflect.kav</C>.</li>
        <li>A field&apos;s <C>result</C> is its type, and it has no <C>params</C>. A method&apos;s <C>params</C> leave out <C>self</C>.</li>
        <li>Every list is worked out while compiling, so asking costs nothing at run time.</li>
      </ul>
      <CallsTable />
      <H2>Where a member comes from</H2>
      <P>
        <C>origin</C> says whether Kavach gives a member (<C>BuiltIn</C>), the type declares it (<C>Declared</C>),
        or an <Link href="/docs/types/extend/"><C>extend</C></Link> adds it, with the module that adds it:
      </P>
      <Ex name="reflect/extended" />
      <P>
        The module is the file&apos;s name: <C>extended</C> for <C>extended.kav</C>. This finds every method a
        program added to a built-in type, for a tool that lists them or checks two modules don&apos;t clash.
      </P>
      <H2>Does it have this member?</H2>
      <P>
        <C>x.has_member(&quot;name&quot;)</C> answers for <C>x</C>&apos;s type, fields and methods alike, built in
        too: a list has <C>len</C>.
      </P>
      <Ex name="reflect/has-member" />
      <P>
        The name must be a text literal, as the compiler answers while compiling. A name from a variable is a
        compile error:
      </P>
      <Ex name="reflect/has-member-name.fails" />
      <P>
        On an <Link href="/docs/types/interface-values/">interface value</Link>, the answer is about the interface,
        not the value inside it. <C>s</C> holds a <C>Square</C>, which has <C>side</C>, but a <C>Shape</C> has only{" "}
        <C>area</C>:
      </P>
      <Ex name="reflect/interface" />
      <H2>Generic types and collections</H2>
      <P>
        A generic type&apos;s members show its type parameters by name: <C>Box&lt;T&gt;</C>&apos;s <C>get</C> gives{" "}
        <C>T</C>. For the collections, write <C>List</C>, <C>Dict</C>, <C>Set</C> or <C>Ring</C>: <C>T</C> is the
        item type, and <C>K</C> and <C>V</C> a dict&apos;s key and value. The compiler knows a built-in method&apos;s
        parameter types but not their names, so they are named <C>a</C>, <C>b</C> and on:
      </P>
      <Ex name="reflect/generic" />
      <H2>What a module offers</H2>
      <P>
        <C>m.members()</C> lists a module&apos;s <C>pub</C> functions, types and constants. For a standard library
        module, that is the part written in Kavach: <C>math.members()</C> has <C>gcd</C> but not yet{" "}
        <C>sin</C>, which the compiler provides.
      </P>
      <Ex name="reflect/module" />
      <P>
        Your own modules work the same way. Here <C>src/shop.kav</C> declares a constant, a type and two functions;
        <C>with_tax</C> isn&apos;t <C>pub</C>, so the module doesn&apos;t offer it:
      </P>
      <Files name="reflect/own-module" first={["src/shop.kav", "src/main.kav"]} />
      <H2>The types</H2>
      <P>Every call gives these types, declared in <C>stdlib/reflect/reflect.kav</C>:</P>
      <SpecCode file={FILE} number="14.32" text={"pub type Member"} />
      <P>
        Types are written as in code: <C>[Int]</C>, <C>String?</C>, <C>fn(Int) -&gt; Bool</C>. A member that
        isn&apos;t <C>pub</C> is listed only inside its own package, where it can be used.
      </P>
      <P>
        Not yet: the functions the compiler provides, such as <C>math.sin</C>, in <C>m.members()</C>, and a
        member&apos;s documentation comment.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={`docs/spec/${FILE}`} label="spec 14.32 · std.reflect" />
        <SrcRef file="stdlib/reflect/reflect.kav" label="reflect.kav" />
        <SrcRef file="compiler/checker/src/reflect.rs" label="compiler/checker/src/reflect.rs" />
        <SrcRef file="tests/conformance/14-standard-library/reflect-lists-members.kav" label="members, tested" />
        <SrcRef file="tests/conformance/14-standard-library/has-member-takes-a-literal.kav" label="has_member's literal, tested" />
        <SrcRef file="tests/conformance/10-modules/a-modules-members-are-what-it-offers/src/main.kav" label="your own module, tested" />
      </div>
    </>
  ),
};
