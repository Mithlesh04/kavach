import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import { Terminal } from "@/components/terminal";
import type { LessonDoc } from "@/lib/chapter";
import { loadProject } from "@/lib/samples";
import { Ex, H2, Note, P, Syntax } from "./parts";

const bareFix = loadProject("records/bare-fix", ["src/main.kav"]);

// ---------- Records ----------

export const recordsDoc: LessonDoc = {
  lead: "A record is a type of your own: named fields grouped under one name, such as a User with a name and an age.",
  simple: <>A record is like a form with labelled boxes: a <C>User</C> form has a box for the name and a box for the age. Each filled-in form is one user.</>,
  minutes: 5,
  before: ["basics/data-types", "functions/functions"],
  body: (
    <>
      <H2>Syntax</H2>
      <Syntax code={"type Name {\n    field: Type\n    other: Type = default\n}"} />
      <H2>Example</H2>
      <Ex name="records/sub-record" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>type User {"{ … }"}</C> declares the record; <C>User(name: …, age: …)</C> makes one, naming every field.</li>
        <li><C>asha.name</C> reads a field.</li>
        <li>A record is a value: <C>copy</C> changing its city leaves <C>asha</C> as she was.</li>
      </ul>
      <Note>Other languages call this a struct, or a class with only data.</Note>
    </>
  ),
  tryIt: {
    task: <>Declare a <C>Book</C> with a <C>title</C> and a number of <C>pages</C>, make one, and print its title.</>,
    answer: <P><C>type Book {"{"} title: String  pages: Int {"}"}</C> (one field per line), then <C>const b = Book(title: &quot;Gitanjali&quot;, pages: 103)</C> and <C>print(b.title)</C>.</P>,
  },
};

export const declareRecordDoc: LessonDoc = {
  lead: "type Name { field: Type … } declares a record; Name(field: value, …) makes one.",
  simple: <>First you design the form: which boxes it has, and what kind of thing goes in each. Then you fill one in to make a record. If a needed box is left out, Kavach tells you at once.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="records/sub-record" />
      <P>Every field must be given when a record is made, unless it has a default. A missing one is caught at once:</P>
      <Ex name="records/missing-field.fails" />
    </>
  ),
};

export const defaultsRecordDoc: LessonDoc = {
  lead: "A field with = value may be left out when the record is made, and then takes that value.",
  simple: <>A default is like a box on a form that is already filled in with the usual answer, such as the city Pune. Leave it alone and that answer is used, or write a different one.</>,
  minutes: 2,
  body: (
    <>
      <Ex name="records/sub-record" />
      <P><C>city: String = &quot;Pune&quot;</C> has a default, so <C>User(name: &quot;Asha&quot;, age: 31)</C> is enough.</P>
    </>
  ),
};

export const withDoc: LessonDoc = {
  lead: "record with { field: value } makes a copy with some fields changed.",
  simple: <>Like photocopying a filled-in form and changing one box on the copy: you get a new form, and the original stays as it was.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="records/sub-record" />
      <P>
        <C>asha with {"{ age: 32 }"}</C> is a new <C>User</C>; <C>asha</C> is still 31. Through a <C>let</C>, you
        can also assign a field directly, as <C>copy.city = &quot;Delhi&quot;</C> does: that changes only{" "}
        <C>copy</C>.
      </P>
    </>
  ),
};

// ---------- Methods ----------

export const methodsDoc: LessonDoc = {
  lead: "A method is a function that belongs to a type, written inside it. self is the value it is called on.",
  simple: <>A method is an action that belongs to a kind of thing, like a rectangle that can work out its own area. You ask the value itself to do it: <C>r.area()</C>.</>,
  minutes: 4,
  before: ["types/records"],
  body: (
    <>
      <Ex name="records/sub-methods" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>r.area()</C> calls the method on <C>r</C>, which inside it is <C>self</C>.</li>
        <li>A function without <C>self</C>, like <C>square</C>, is called on the type: <C>Rect.square(4.0)</C>.</li>
      </ul>
      <H2>Methods for any type</H2>
      <P><C>extend</C> adds methods to a type you didn&apos;t write, even a built-in one:</P>
      <Ex name="records/extend" />
    </>
  ),
};

// ---------- Unions ----------

export const unionsDoc: LessonDoc = {
  lead: "A union is a value that is exactly one of several cases: a Shape is a Circle or a Rect.",
  simple: <>A union is like a question on a form where you tick exactly one box: a shape is a circle or a rectangle, never both. Each choice can carry its own details, such as a circle&apos;s radius.</>,
  minutes: 5,
  before: ["types/records", "basics/match"],
  body: (
    <>
      <Ex name="records/sub-union" />
      <P>
        Each case, a <strong className="text-ink">variant</strong>, can carry its own fields. A <C>match</C>{" "}
        handles each variant, and must handle them all.
      </P>
      <H2>Enums</H2>
      <P>A union whose variants carry nothing is an enum. It can list its variants, name them, and read them back from text:</P>
      <Ex name="records/sub-enum" />
      <Note>Other languages call these enums, tagged unions, sum types or variants.</Note>
    </>
  ),
};

export const declareUnionDoc: LessonDoc = {
  lead: "type Name = A | B(field: Type) | … declares a union.",
  simple: <>Declaring a union means writing down the list of choices, once. Each choice gets a name, and can say which details come with it, like a radius for a circle.</>,
  minutes: 4,
  body: (
    <>
      <Ex name="records/sub-union" />
      <P>
        A variant is always written with its union&apos;s name, as a value and in a <C>match</C> case alike:{" "}
        <C>Shape.Circle(radius: 2.0)</C>, <C>Status.Paid</C>, <C>FsError.NotFound(path)</C>. So what a name means
        never depends on which other unions exist. Printing still shows the variant alone: <C>print(Status.Paid)</C>{" "}
        prints <C>Paid</C>.
      </P>
      <P>A bare variant name is an error that says what to write, and <C>kavach fix</C> writes it for you:</P>
      <CodeBlock code={bareFix.files[0].code} file="src/main.kav, before" />
      <Terminal session={bareFix.output ?? ""} />
      <Note>A variant may have the name of a built-in type, such as <C>List</C> or <C>Error</C>: <C>Kind.List</C> is the variant, and <C>List</C> stays the type.</Note>
    </>
  ),
};

export const matchUnionDoc: LessonDoc = {
  lead: "match on a union takes each variant apart, and the compiler checks that none is forgotten.",
  simple: <><C>match</C> works like sorting post: each kind of item goes to its own tray. If a new kind is added later, Kavach points to every place that has no tray for it yet.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="records/sub-union" />
      <P>Add a variant later, and every <C>match</C> that doesn&apos;t handle it becomes an error that names it:</P>
      <Ex name="records/sub-exhaustive.fails" />
    </>
  ),
};

// ---------- Optionals ----------

export const optionalsDoc: LessonDoc = {
  lead: "There is no null. A value that may be missing has a type ending in ?, and you must say what happens when it is.",
  simple: <>Sometimes a value may simply not be there, like a parcel that may or may not have arrived. Kavach marks such values with <C>?</C> and makes you decide what to do when one is missing, so an empty box never surprises you.</>,
  minutes: 5,
  before: ["types/records", "basics/if-let"],
  body: (
    <>
      <Ex name="types/optionals-basic" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>User?</C> is a <C>User</C> or <C>none</C>.</li>
        <li><C>?.</C> reads through: <C>found?.name</C> is the name, or <C>none</C> when there is no user.</li>
        <li><C>??</C> gives a value to use instead of <C>none</C>.</li>
        <li><C>if let</C> runs a block only when the value is there.</li>
      </ul>
      <Note>
        Using an optional as if it were surely there is a compile error. The &quot;null pointer exception&quot; of
        other languages can&apos;t happen.
      </Note>
    </>
  ),
};

export const optionalWhatDoc: LessonDoc = {
  lead: "T? means a T or none. Lookups that can find nothing give one.",
  simple: <><C>T?</C> is like a box that holds one thing or is empty. Looking up a name in a phone book gives such a box, because the name may not be listed.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="types/optionals-lookups" />
      <P>Using the value without saying what to do when it is missing is refused:</P>
      <Ex name="types/optionals-must-check.fails" />
      <H2>A value where an optional is wanted</H2>
      <P>
        A <C>T</C> can be used wherever a <C>T?</C> is wanted: in a variable, as a lambda&apos;s result, and as the value
        of a <C>Shared&lt;T?&gt;</C>:
      </P>
      <Ex name="types/optional-fill" />
      <P>
        A <C>Shared&lt;Node&gt;</C> that is already made can&apos;t be stored where a <C>Shared&lt;Node?&gt;</C> is wanted.
        Every copy of a <C>Shared</C> sees one value, and through the new type, <C>none</C> could be put in it. Make it
        with the optional type from the start:
      </P>
      <Ex name="types/optional-fill-shared.fails" />
    </>
  ),
};

export const optionalReadDoc: LessonDoc = {
  lead: "Read an optional with ?? for a default, ?. to read through, or if let to act only when it is there.",
  simple: <>Three ways to open a box that might be empty: use a spare when it is empty (<C>??</C>), look inside only if something is there (<C>?.</C>), or do a job only when it is full (<C>if let</C>).</>,
  minutes: 3,
  body: (
    <>
      <Ex name="types/optionals-basic" />
      <P>
        There is also <C>.unwrap()</C>: the value, or a panic when it is <C>none</C>. Outside <C>test</C> blocks and{" "}
        <C>example</C> clauses it gets a warning, because the panic would say nothing of why:
      </P>
      <Ex name="types/unwrap.warns" />
      <Note>
        Handle <C>none</C> with <C>??</C>, <C>?.</C>, <C>if let</C> or <C>match</C>, or say why it can&apos;t happen
        with <C>?? panic(&quot;…&quot;)</C>. And on a value the compiler already knows is <C>none</C>,{" "}
        <C>.unwrap()</C> won&apos;t even compile: see <Link href="/docs/errors/kinds-of-errors/">Kinds of Errors</Link>.
      </Note>
    </>
  ),
};

// ---------- Anonymous Records ----------

export const anonDoc: LessonDoc = {
  lead: "(times: 3, wait_ms: 200) is a record written on the spot, with no type declared for it.",
  simple: <>A quick record written on the spot, like a sticky note with a few labelled lines instead of a printed form. Handy when a function gives back several named values.</>,
  minutes: 4,
  before: ["types/records", "collections/tuples"],
  body: (
    <>
      <Ex name="records/sub-anon" />
      <P>
        A function can give back several named values: <C>r.lo</C> reads better than the position in a tuple. Two
        anonymous records with the same fields fit each other, in any order. Name the shape with an alias when you
        use it often:
      </P>
      <Ex name="records/alias" />
      <Note>
        Start with an anonymous record while a design settles; turn it into a <C>type</C> when it needs methods.
      </Note>
    </>
  ),
};

// ---------- Generics ----------

export const genericsDoc: LessonDoc = {
  lead: "A generic function or type works for any type: fn first<T>(items: [T]) -> T? works on a list of anything.",
  simple: <>A function that gives the first item of a list doesn&apos;t care what the list holds: numbers, words, anything. A generic lets you write it once, for every kind of thing.</>,
  minutes: 5,
  before: ["functions/functions", "types/records"],
  body: (
    <>
      <Ex name="types/generics-fn" />
      <P>
        <C>&lt;T&gt;</C> is a placeholder for a type. Each call fills it in: <C>first([3, 1, 2])</C> uses{" "}
        <C>Int</C>, <C>first([&quot;x&quot;, &quot;y&quot;])</C> uses <C>String</C>. Types like{" "}
        <C>Pair&lt;A, B&gt;</C> work the same way. A method can have its own placeholders too:
      </P>
      <Ex name="types/generics-methods" />
      <H2>Type arguments written where a value is made</H2>
      <P>
        A generic type&apos;s arguments are usually worked out from its fields and from where the value goes. When no
        field shows them, write them where the value is made, or give the variable its type. Inside the type&apos;s
        methods, <C>I</C> is then the type the value was made with:
      </P>
      <Ex name="types/generics-construction" />
      <P>The count must match the type&apos;s parameters:</P>
      <Ex name="types/generics-arity.fails" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/02-types/09-generics.md" label="spec 2.9 · generics" />
        <SrcRef file="tests/conformance/02-types/generic-type-methods-know-their-type-arguments.kav" label="tested" />
      </div>
    </>
  ),
  deeper: (
    <P>
      A generic function is compiled once for each type it is used with, so it runs as fast as if you had written a
      copy for that type yourself. To hold values of different types together in one list instead, use an{" "}
      <Link href="/docs/types/interface-values/">interface value</Link>.
    </P>
  ),
};

// ---------- Interfaces ----------

export const interfacesDoc: LessonDoc = {
  lead: "An interface names methods a type promises to have. A type keeps the promise only when it says so.",
  simple: <>An interface is like a job description: it lists the tasks someone must be able to do. A type gets the job only when it says it accepts it and can do every task on the list.</>,
  minutes: 5,
  before: ["types/methods", "types/generics"],
  body: (
    <>
      <Ex name="types/interfaces" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>type User: Describe</C> promises a <C>describe</C> method; leaving it out is an error.</li>
        <li><C>extend Rect: Describe</C> adds the promise to a type declared elsewhere.</li>
        <li><C>&lt;T: Describe&gt;</C> accepts any type that keeps it.</li>
      </ul>
      <P>A type that happens to have the method but never said so doesn&apos;t count, and the message says what to write:</P>
      <Ex name="types/interfaces-say-so.fails" />
      <P>
        A method that keeps the promise has the interface&apos;s types: its parameters, its value and its error, with{" "}
        <C>Self</C> read as the type and the interface&apos;s type arguments put in. <C>Getter&lt;String&gt;</C>{" "}
        promises a <C>get</C> that gives a <C>String</C>:
      </P>
      <Ex name="types/interfaces-same-types.fails" />
      <Note>Other languages call this a trait or a protocol.</Note>
    </>
  ),
};

export const interfaceValuesDoc: LessonDoc = {
  lead: "An interface is also a type for values: a [Describe] can hold users and rectangles together.",
  simple: <>Like a box labelled &quot;things that can describe themselves&quot;: users and rectangles can go in together. Ask any item to describe itself, and each answers in its own way.</>,
  minutes: 4,
  before: ["types/interfaces"],
  body: (
    <>
      <Ex name="types/interface-values" />
      <P>
        Each <C>d.describe()</C> calls the method of whatever type <C>d</C> holds. Only the interface&apos;s
        methods can be used on such a value. To tell a closed set of types apart, use a{" "}
        <Link href="/docs/types/unions/">union</Link> instead.
      </P>
    </>
  ),
};

// ---------- Writing Types ----------

export const writingTypesDoc: LessonDoc = {
  lead: "Types are written where people read them: parameters, results and record fields. Everywhere else they are worked out.",
  simple: <>Like price tags in a shop: they go where customers look, not on every box in the store room. You write the type of each input, result and field, and Kavach works out the rest.</>,
  minutes: 3,
  body: (
    <>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <tbody>
            {[
              ["Local variables", "worked out"],
              ["Lambda parameters and results", "worked out"],
              ["Function parameters", "written"],
              ["A function's result", "written"],
              ["Record fields", "written"],
            ].map(([place, rule]) => (
              <tr key={place} className="border-b border-line">
                <td className="py-2 pr-4">{place}</td>
                <td className="py-2 font-bold text-ink">{rule}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <Ex name="types/untyped-param.fails" />
      <P>
        A signature is what people, and AI tools, read to know a function. So it is always spelled out, even for a
        private function; inside the body, the compiler works everything out.
      </P>
    </>
  ),
};
