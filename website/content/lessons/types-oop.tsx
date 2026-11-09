import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, H2, Note, P } from "./parts";

// ---------- Method Chains ----------

export const chainsDoc: LessonDoc = {
  lead: "Call one method on the result of the last: text.trim().lower(). Each step gives a new value for the next.",
  simple: <>Like an assembly line: each step takes what the last step made and passes something new to the next. What you started with stays as it was.</>,
  minutes: 5,
  before: ["types/methods", "functions/lambdas"],
  body: (
    <>
      <Ex name="types/chain-text" />
      <P>
        Read a chain left to right: <C>trim()</C> gives a new string, <C>lower()</C> works on that one, and so on.
        Nothing is changed in place, so <C>name</C> is still as it was.
      </P>
      <H2>One step per line</H2>
      <P>A line that starts with <C>.</C> carries on the line before it, so a long chain reads as a list of steps:</P>
      <Ex name="types/chain-lines" />
      <H2>Chains of your own</H2>
      <P>
        A method that gives back its own type can be chained too. Each step here returns a new{" "}
        <C>Query</C>, made with <C>with</C>, so the chain builds the value one piece at a time:
      </P>
      <Ex name="types/chain-builder" />
      <Note>
        <C>.paid</C> and <C>.amount</C> are the <Link href="/docs/functions/lambdas/dot-shorthand/">dot shorthand</Link>:{" "}
        <C>.paid</C> means <C>o =&gt; o.paid</C>.
      </Note>
    </>
  ),
  deeper: (
    <P>
      A chain over a list doesn&apos;t build a list at every step. The compiler fuses <C>filter</C>, <C>map</C> and{" "}
      <C>sum</C> into a single loop, and a builder chain on a value nobody else holds changes it in place instead of
      copying it each time. You write the readable version and get the fast one.
    </P>
  ),
};

// ---------- Extend ----------

export const extendDoc: LessonDoc = {
  lead: "extend adds methods to a type you didn't write: a built-in type, a union, or a type from another module.",
  simple: <>Like adding new buttons to a machine you bought, without opening it up: <C>extend</C> gives a type new actions, even a type you didn&apos;t make. It adds actions only, never new data.</>,
  minutes: 4,
  before: ["types/methods"],
  body: (
    <>
      <Ex name="records/extend" />
      <Ex name="types/extend-more" />
      <P>
        Inside <C>extend</C>, methods are written as in a record, with <C>self</C>. A union&apos;s methods go in an{" "}
        <C>extend</C> too, since a union has no body of its own.
      </P>
      <H2>Keeping a promise for another type</H2>
      <P>
        <C>extend Rect: Describe {"{ … }"}</C> says that <C>Rect</C>, declared elsewhere, keeps the{" "}
        <C>Describe</C> interface:
      </P>
      <Ex name="types/interfaces" />
      <Note>
        <C>extend</C> adds methods, never fields: a type&apos;s data is always where the type is declared.
      </Note>
    </>
  ),
};

// ---------- Classes and Inheritance ----------

const mapping: [string, React.ReactNode, string][] = [
  ["A class with fields", <>a <C>type</C> (a record)</>, "/docs/types/records/"],
  ["Methods", <>methods inside the <C>type</C>, with <C>self</C></>, "/docs/types/methods/"],
  ["A constructor", <><C>User(name: …)</C>, with defaults for optional fields</>, "/docs/types/records/default-values/"],
  ["A static method", <>a method without <C>self</C>: <C>Rect.square(2.0)</C></>, "/docs/types/methods/"],
  ["An abstract class or interface", <>an <C>interface</C></>, "/docs/types/interfaces/"],
  ["Implementing it", <><C>type Circle: Shape</C>, or <C>extend Rect: Shape</C></>, "/docs/types/extend/"],
  ["A list of different subclasses", <>a list of an interface type: <C>[Shape]</C></>, "/docs/types/interface-values/"],
  ["Inheriting fields", <>a field holding the other value (composition)</>, "/docs/types/classes-and-inheritance/"],
  ["A fixed set of subclasses", <>a union, <C>type Shape = Circle | Square</C>, checked by <C>match</C></>, "/docs/types/unions/"],
  ["Adding methods to a class you didn't write", <><C>extend</C></>, "/docs/types/extend/"],
];

export const classesDoc: LessonDoc = {
  lead: "Kavach has no classes and no inheritance. Records, methods, interfaces and composition do the same jobs, with fewer surprises.",
  simple: <>Many languages build types as family trees, where a child inherits from its parent. Kavach builds with blocks instead: a value can hold another value, and every action a type has is written where you can find it.</>,
  minutes: 6,
  before: ["types/interfaces", "types/interface-values"],
  body: (
    <>
      <H2>From classes to Kavach</H2>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <thead>
            <tr className="border-b border-line text-left text-soft">
              <th className="py-2 pr-4 font-bold">In Java, C#, Python…</th>
              <th className="py-2 font-bold">In Kavach</th>
            </tr>
          </thead>
          <tbody>
            {mapping.map(([theirs, ours, href]) => (
              <tr key={theirs} className="border-b border-line align-top">
                <td className="py-2 pr-4 text-muted">{theirs}</td>
                <td className="py-2"><Link href={href} className="text-ink no-underline hover:underline">{ours}</Link></td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <H2>All of it together</H2>
      <Ex name="types/classes" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>Shape</C> is the interface: what every shape can do.</li>
        <li><C>Circle</C> and <C>Square</C> keep that promise, each its own way.</li>
        <li>A <C>[Shape]</C> holds both kinds, and each call runs the right method.</li>
        <li><C>Label</C> <em>has</em> a shape instead of <em>being</em> one: composition.</li>
      </ul>
      <H2>Extending a record is refused</H2>
      <P>Writing a subclass out of habit gets an error that names both ways to do it in Kavach:</P>
      <Ex name="habits/inherit.fails" />
    </>
  ),
  deeper: (
    <>
      <P>
        Inheritance ties a class to everything above it: change a base class, and every subclass may break, even
        ones you have never seen (the &quot;fragile base class&quot; problem). Deep hierarchies also hide where a method
        really comes from.
      </P>
      <P>
        Kavach keeps the useful half. Interfaces give polymorphism (one call, many types), and composition gives
        reuse (a value that holds another). Every method a type has is written in its own declaration or in an{" "}
        <C>extend</C>, so it is always easy to find.
      </P>
      <div><SrcRef file="docs/decisions/0029-interface-values.md" label="decision 0029 · polymorphism without inheritance" /></div>
    </>
  ),
};
