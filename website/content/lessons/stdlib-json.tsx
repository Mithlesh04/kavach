import fs from "node:fs";
import path from "node:path";
import Link from "next/link";
import { C } from "@/components/inline-code";
import { RaceChart } from "@/components/race-chart";
import { SrcRef } from "@/components/src-ref";
import { VerifyBlock } from "@/components/verify-block";
import type { LessonDoc } from "@/lib/chapter";
import { specBlockWith } from "@/lib/spec-tables";
import { Ex, H2, Note, P, Syntax } from "./parts";
import { Limits } from "./limits";
import { ModuleTable } from "./spec-table";

const JSON_SPEC = "02-types/12-json.md";

// How the JSON benchmark was measured, and why `read` is slower, as its README says.
function jsonReadme(re: RegExp, what: string): string {
  const text = fs.readFileSync(path.join(process.cwd(), "..", "benchmarks/json/README.md"), "utf8");
  const said = text.match(re);
  if (!said) throw new Error(`benchmarks/json/README.md no longer says ${what}`);
  return said[1];
}
const digitsShare = () =>
  jsonReadme(/read the digits each time, but that is (only about \d+% of `read`) \(profiled/, "how little the digits cost");
const onTape = () =>
  jsonReadme(/Since \d{4}-\d{2}-\d{2} (a parsed document is one list of words) \(decision 0082\)/, "how a parsed document is held");
const readLoses = () =>
  jsonReadme(/Most of what `read` still loses to Python is (the small object .+ a loop visits)\./, "why read is slower");

// ---------- JSON ----------

// JSON's datatypes and how Kavach holds each (spec 2.12): the Json case,
// the method that reads it, and the type json.decode fills.
const kinds: [json: string, example: string, jsonCase: string, read: string, decoded: string][] = [
  ["null", "null", "Null", "is_null()", "none, in an optional (T?)"],
  ["true / false", "true", "Boolean(value)", "bool()", "Bool"],
  ["a number", "42, 2.5, 1e3", "Number(digits)", "int(), float(), decimal(), bigint()", "Int, Float, Decimal, BigInt, or a sized integer"],
  ["a string", "\"Asha\"", "Text(value)", "text()", "String (Char for one character)"],
  ["an array", "[1, 2]", "Array(items)", "list()", "a list, set or tuple"],
  ["an object", "{\"id\": 7}", "Object(fields)", "object()", "a record, or a dict with String keys"],
];

export const jsonDoc: LessonDoc = {
  lead: "JSON is text that carries data between programs. Kavach reads it straight into your own types, or into a Json value when its shape is known only while the program runs.",
  simple: <>JSON is like a shared form that many programs know how to fill in and read. Kavach can copy its answers straight into your own forms, checking that every box holds the right kind of thing.</>,
  minutes: 6,
  before: ["types/records", "types/unions"],
  body: (
    <>
      <Ex name="json/sub-decode" />
      <P>
        <C>json.decode&lt;[Order]&gt;</C> reads the text straight into a list of <C>Order</C> records, checking
        every field&apos;s type as it goes.
      </P>
      <H2>Two ways to read JSON</H2>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <thead>
            <tr className="border-b border-line text-left text-soft">
              <th className="py-2 pr-4 font-bold">When</th>
              <th className="py-2 pr-4 font-bold">Use</th>
              <th className="py-2 font-bold">You get</th>
            </tr>
          </thead>
          <tbody>
            <tr className="border-b border-line align-top">
              <td className="py-2 pr-4 text-muted">You know its shape</td>
              <td className="py-2 pr-4"><C>json.decode&lt;T&gt;(text)</C></td>
              <td className="py-2 text-muted">a <C>T</C>, every field checked by the compiler where you use it</td>
            </tr>
            <tr className="border-b border-line align-top">
              <td className="py-2 pr-4 text-muted">You don&apos;t, or only partly</td>
              <td className="py-2 pr-4"><C>json.parse(text)</C></td>
              <td className="py-2 text-muted">a <C>Json</C> value, read field by field: see <Link href="/docs/standard-library/json/json-values/">Json values</Link></td>
            </tr>
          </tbody>
        </table>
      </div>
      <H2>JSON&apos;s datatypes in Kavach</H2>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <thead>
            <tr className="border-b border-line text-left text-soft">
              <th className="py-2 pr-4 font-bold">JSON</th>
              <th className="py-2 pr-4 font-bold">Example</th>
              <th className="py-2 pr-4 font-bold">Json case</th>
              <th className="py-2 pr-4 font-bold">Read with</th>
              <th className="py-2 font-bold">Decodes into</th>
            </tr>
          </thead>
          <tbody>
            {kinds.map(([j, ex, kase, read, decoded]) => (
              <tr key={j} className="border-b border-line align-top">
                <td className="py-2 pr-4 whitespace-nowrap">{j}</td>
                <td className="py-2 pr-4"><C>{ex}</C></td>
                <td className="py-2 pr-4"><C>{kase}</C></td>
                <td className="py-2 pr-4"><C>{read}</C></td>
                <td className="py-2 text-muted">{decoded}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <H2>Every function</H2>
      <ModuleTable file={JSON_SPEC} number="2.12" first="Function" heading="std.json" />
      <H2>A type&apos;s schema</H2>
      <P>
        <C>json.schema&lt;T&gt;()</C> gives the <a href="https://json-schema.org">JSON Schema</a> of a type, as a{" "}
        <C>Json</C> value. A type&apos;s doc comment (<C>{"///"}</C>) becomes its schema&apos;s{" "}
        <C>&quot;description&quot;</C>, and so does a field&apos;s:
      </P>
      <Ex name="json/schema-docs" />
      <P>When both a field and its type have one, the field&apos;s comes first in one description, then the type&apos;s:</P>
      <Ex name="json/schema-docs-both" />
      <P>
        A <C>secret</C> field is marked <C>&quot;writeOnly&quot;: true</C>, so a library that works from the schema sees
        that it can&apos;t be written out:
      </P>
      <Ex name="json/schema-secret" />
      <P>An unsigned whole number (<C>UInt8</C> to <C>UInt64</C>) can&apos;t be negative, so its schema says <C>&quot;minimum&quot;: 0</C>:</P>
      <Ex name="json/schema-unsigned" />
      <H2>A function described</H2>
      <P>
        <C>json.describe(f)</C> describes a function from its signature, when the program is compiled: its name,
        doc comment, parameters with their schemas, the schemas of its result and error, what it <C>uses</C>, and
        its <C>example</C> clauses as text. A web framework uses it, with <C>json.schema&lt;T&gt;()</C>, to write an
        OpenAPI description of its handlers.
      </P>
      <Ex name="json/describe" />
      <Syntax code={specBlockWith(JSON_SPEC, "2.12", "pub type FnSchema")} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>When <C>f</C> names a function, the result is a constant <C>FnSchema</C>, so reading it costs nothing more than reading any value.</li>
        <li>A parameter, result or error whose type has no schema, such as a function, is a compile error.</li>
      </ul>
      <P>
        Given a function value instead, such as a parameter or an item of a list of handlers,{" "}
        <C>json.describe(value)</C> gives <C>FnSchema?</C>: the description of the named function the value is, or{" "}
        <C>none</C> for a lambda (and for a generic function, which has no one description). It works inside a generic
        function too, with the types it was called with. There, a parameter, result or error with no schema (a
        function, a record holding one, the built-in <C>Error</C>) is <C>{"{}"}</C>, any value, so every named
        function can be described. So a framework handed its routes as values can describe each one:
      </P>
      <Ex name="json/describe-value" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={`docs/spec/${JSON_SPEC}`} label="spec 2.12 · JSON" />
        <SrcRef file="tests/conformance/02-types/json-describe-describes-a-function.kav" label="describe, tested" />
        <SrcRef file="tests/conformance/02-types/json-describe-takes-function-values.kav" label="function values, tested" />
        <SrcRef file="tests/conformance/02-types/json-schema-carries-doc-comments.kav" label="descriptions, tested" />
        <SrcRef file="docs/decisions/0085-function-types-with-errors-and-uses.md" label="decision 0085" />
      </div>
      <H2>Safe with hostile input</H2>
      <P>Text from outside can be built to do harm. Parsing has limits, and breaking one is an error, not a crash:</P>
      <Ex name="json/deep" />
      <P>A key written twice could mean two different things to two programs, so it is refused:</P>
      <Ex name="json/duplicate" />
      <P>
        A number too big for its field is an error too, never rounded quietly. The limits on depth, size and fields can be raised for one call, as
        in <C>json.parse(body, max_depth: 1_000)</C>:
      </P>
      <Limits id="json" />
      <H2>Comments, for configuration files</H2>
      <P>
        <C>json.parse</C> reads standard JSON (RFC 8259). Configuration files are often written with comments and a
        trailing comma (JSONC): <C>comments: true</C> takes them, and only when asked.
      </P>
      <Ex name="json/comments" />
      <P>
        <C>NaN</C>, <C>Infinity</C>, hex numbers and the rest of JSON5 are syntax errors, even with{" "}
        <C>comments: true</C>.
      </P>
    </>
  ),
  deeper: (
    <>
      <P>How fast a whole JSON program runs, measured with the steps to repeat it below:</P>
      <RaceChart suite="json" id="whole" />
      <P>And each step on its own:</P>
      {["decode", "stringify", "parse", "read"].map((id) => (
        <RaceChart key={id} suite="json" id={id} />
      ))}
      <Note>
        Since{" "}
        <SrcRef file="docs/decisions/0071-json-numbers-keep-their-digits.md" label="decision 0071" />, a <C>Json</C>{" "}
        number keeps its exact digits, so <C>int()</C> and <C>float()</C> read the digits each time — but a profile
        showed that is {digitsShare().replace("`read`", "reading")}. And since{" "}
        <SrcRef file="docs/decisions/0082-the-json-tape-as-built.md" label="decision 0082" />, {onTape()}:{" "}
        <C>item[&quot;qty&quot;].int()</C> finds the field on the tape, where a constant key remembers its place, and
        reads the digits there, making nothing. Most of what reading still loses to Python is {readLoses()}.
      </Note>
      <VerifyBlock suite="json" id="whole" />
      <div><SrcRef file={`docs/spec/${JSON_SPEC}`} label="spec 2.12 · Json" /></div>
    </>
  ),
};

export const jsonValuesDoc: LessonDoc = {
  lead: "A Json value holds JSON whose shape is known only while the program runs. It is a union of six cases, one for each kind of JSON value.",
  simple: <>Sometimes data arrives and you don&apos;t know what it looks like, like a parcel with no label. A <C>Json</C> value can hold any of it, and tells you which of six kinds each piece is: a number, some text, a list and so on.</>,
  minutes: 4,
  body: (
    <>
      <Syntax
        code={
          "type Json = Null |\n    Boolean(value: Bool) |\n    Number(digits: String) |\n    Text(value: String) |\n    Array(items: [Json]) |\n    Object(fields: {String: Json})"
        }
      />
      <P>Being a union, <C>match</C> sees every case:</P>
      <Ex name="types/unknown-json" />
      <H2>Every method</H2>
      <ModuleTable file={JSON_SPEC} number="2.12" first="Method" />
      <H2>Numbers keep their digits</H2>
      <P>
        A <C>Number</C> keeps the digits it was written with, exactly, so no number is changed by reading it and
        writing it out again. Each method reads it as the type you need, and gives <C>none</C> when it doesn&apos;t fit:
      </P>
      <Ex name="json/exact-numbers" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>The id is too big for <C>Int</C>, so <C>.int()</C> gives <C>none</C>; <C>.bigint()</C> takes any size.</li>
        <li><C>.decimal()</C> reads <C>0.1</C> exactly, where a <C>Float</C> holds only the nearest number to it.</li>
        <li>Written out again, both are the same digits that came in.</li>
      </ul>
      <P><C>.float()</C> reads any number, whole or not; <C>&quot;free&quot;</C> isn&apos;t a number, so it adds <C>0.0</C>:</P>
      <Ex name="types/unknown-numbers" />
      <P>
        Parsing takes a number of any size. A number whose exponent puts it beyond 10^10_000, or below 10^-10_000, is
        out of range for <C>.decimal()</C> and <C>.bigint()</C> too, so a short text can&apos;t make a huge value:
      </P>
      <Ex name="json/huge" />
      <Note>
        JSON&apos;s <C>null</C> is the case <C>Null</C>. Kavach itself has no <C>null</C>: see{" "}
        <Link href="/docs/basics/data-types/">Data Types</Link> for how it meets <C>none</C>.
      </Note>
    </>
  ),
};

export const jsonReadingDoc: LessonDoc = {
  lead: "json.parse gives a Json value; doc[\"key\"] and doc[i] read into it, and .text(), .int() and the others give the value as an optional.",
  simple: <>Reading JSON is like following directions: take the box called <C>name</C>, then the first item inside it. If a box is missing, you get &quot;nothing here&quot; instead of a crash, and you can give a fallback.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="json/sub-dynamic" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Reading never panics: a missing key, an index out of range, or reading into something that isn&apos;t an object gives <C>Null</C>, so reads chain safely.</li>
        <li>The method then gives <C>none</C>, and <C>??</C> gives a fallback.</li>
      </ul>
      <Ex name="types/unknown-fields" />
    </>
  ),
};

export const jsonNestedDoc: LessonDoc = {
  lead: "Objects inside objects and lists inside them: chain reads to go down, or decode the whole shape into records inside records.",
  simple: <>JSON is often boxes inside boxes. You open them one at a time to reach the thing you want, or describe all the boxes once and read everything in one step.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="json/nested" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>order[&quot;items&quot;][1][&quot;price&quot;]</C> goes down one level at a time: a key for an object, a position for an array.</li>
        <li><C>.list()</C> gives an array&apos;s items, to loop over.</li>
        <li>A path that isn&apos;t there, however deep, ends in <C>Null</C>: <C>no phone</C>.</li>
      </ul>
      <P>When you know the shape, write it as types, and one call reads it all, checked at every level:</P>
      <Ex name="json/nested-decode" />
    </>
  ),
};

export const jsonBuildingDoc: LessonDoc = {
  lead: "Where a Json is expected, ordinary literals become JSON; json.stringify writes any value as JSON text.",
  simple: <>Building JSON is filling in the shared form from your own values, ready to send to another program. Kavach writes any value for you, and blacks out each secret field as <C>&lt;secret&gt;</C>.</>,
  minutes: 4,
  body: (
    <>
      <Ex name="json/sub-build" />
      <P><C>none</C> becomes <C>null</C>. A record is written with its fields in the order they are declared:</P>
      <Ex name="json/pretty" />
      <H2>How each type is written</H2>
      <Ex name="json/types" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>A <C>Decimal</C> and a <C>BigInt</C> are written as numbers with all their digits: <C>5.00</C> stays <C>5.00</C>.</li>
        <li>A <C>Char</C> is text of one character; an optional that is <C>none</C> is <C>null</C>.</li>
        <li>A union case without fields is its name; one with fields is an object holding them, under its name.</li>
        <li>A variant that carries a value (<Link href="/docs/types/unions/#values-on-variants">values on variants</Link>) is that value instead: its text, or its number.</li>
        <li>A <C>Date</C>, <C>Time</C> or <C>DateTime</C> is ISO 8601 text, a <C>DateTime</C> in UTC: see <Link href="/docs/standard-library/json/decoding-into-a-type/#dates-and-times">Dates and times</Link>.</li>
        <li><C>Shared(x)</C> is written as <C>x</C>. A <C>Weak</C> is its value while it lives, and <C>null</C> once it was freed.</li>
      </ul>
      <P>Every value can be written; none is a compile error.</P>
      <P>
        Values on variants let a type hold what another system sends. Reading takes only a value, not a name; anything
        else is a problem that expects &quot;a variant of <C>PaymentStatus</C>&quot;. A variant still prints as its name:
      </P>
      <Ex name="json/variant-values" />
      <P>
        A type&apos;s schema lists the values: <C>{'{"type": "string", "enum": [...]}'}</C> of the texts, or{" "}
        <C>{'{"type": "integer", "enum": [...]}'}</C> of the numbers.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="tests/conformance/02-types/json-writes-and-reads-variant-text.kav" label="text, tested" />
        <SrcRef file="tests/conformance/02-types/json-writes-variant-numbers.kav" label="numbers, tested" />
        <SrcRef file="tests/conformance/02-types/json-schema-of-variants-with-values.kav" label="schema, tested" />
      </div>
      <H2>Dict keys</H2>
      <P>
        A JSON key is always text, so each dict key is written as text. Numbers, <C>Bool</C>, <C>Char</C> and{" "}
        <C>BigInt</C> are their text, and a union case without fields is its name, or its value when it carries one. Any other key, such as a list, is its
        own compact JSON, as text:
      </P>
      <Ex name="json/keys" />
      <P>Decoding reads each key back as the dict&apos;s key type says. A key that doesn&apos;t read as one is a problem at its path.</P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="tests/conformance/02-types/json-keys-are-written-as-text.kav" label="keys, tested" />
        <SrcRef file="tests/conformance/02-types/json-keys-of-variants-with-values.kav" label="keys of variants with values, tested" />
      </div>
      <H2>Big numbers as text</H2>
      <P>
        A <C>BigInt</C> or <C>Decimal</C> is a number with all its digits. Some readers, such as JavaScript, round
        numbers past 2^53. For them, write <C>big_as_text: true</C>, to <C>json.stringify</C> or <C>json.of</C>:
      </P>
      <Ex name="json/big-as-text" />
      <P>Decoding, <C>.bigint()</C> and <C>.decimal()</C> read both forms, so the program reads back what it wrote either way.</P>
      <SrcRef file="tests/conformance/02-types/json-big-as-text-writes-bigint-and-decimal-as-text.kav" label="big as text, tested" />
      <H2>Secrets</H2>
      <P>
        A <C>secret</C> field is written <C>&quot;&lt;secret&gt;&quot;</C>, as <C>print</C> shows it, so a secret
        can&apos;t leave in an API response by accident. <C>grpc.encode</C> writes it the same way:
      </P>
      <Ex name="json/sub-secret" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="tests/conformance/02-types/json-secret-fields-are-written-hidden.kav" label="secret fields, tested" />
        <SrcRef file="tests/conformance/02-types/json-stringify-hides-secret-fields-in-lists.kav" label="in lists, tested" />
        <SrcRef file="tests/conformance/02-types/json-literals-hide-secret-fields.kav" label="in Json literals, tested" />
        <SrcRef file="tests/conformance/14-standard-library/grpc-hides-secret-fields.kav" label="gRPC, tested" />
      </div>
      <H2>Functions, one way only</H2>
      <P>
        A value with no JSON meaning, such as a function, a <C>Task</C>, a <C>Channel</C>, a file or a socket, is
        written as its printed form:
      </P>
      <Ex name="json/function" />
      <P>This goes one way. Text can&apos;t become a function, so decoding a type that holds one is a compile error:</P>
      <Ex name="json/decode-function.fails" />
      <P>
        Through a generic function too: <C>json.decode&lt;T&gt;</C> of a type parameter is checked at each call that
        gives it a real type. The call that would read a function is the error, even when it passes through other
        generic functions:
      </P>
      <Ex name="json/decode-generic.fails" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="tests/conformance/02-types/json-of-a-function-is-its-printed-form.kav" label="printed form, tested" />
        <SrcRef file="tests/conformance/02-types/json-decode-of-a-function-is-a-compile-error.kav" label="decoding, tested" />
        <SrcRef file="tests/conformance/02-types/json-decode-of-a-generic-is-checked-where-it-is-used.kav" label="generic calls, tested" />
        <SrcRef file="docs/decisions/0132-json-for-every-value.md" label="decision 0132" />
      </div>
    </>
  ),
};

export const jsonDecodingDoc: LessonDoc = {
  lead: "json.decode<T> reads JSON text straight into your type T, and says exactly where it doesn't fit.",
  simple: <>Decoding is like a clerk checking a filled-in form against the right template. If a box holds the wrong kind of answer, the clerk tells you exactly which box, and what was expected.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="json/decode-fields" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>A missing field takes its default (<C>city</C>) or <C>none</C> if it is optional (<C>nickname</C>).</li>
        <li>A field your type doesn&apos;t have is refused too, so a misspelled or smuggled field is caught. See below for reading a service that may add fields.</li>
        <li>Decoding is strict about values: an integer field takes only a number written without a fraction or exponent that fits its type.</li>
        <li>A <C>BigInt</C> field takes a whole number of any size, a <C>Decimal</C> field any number exactly, and a <C>Float</C> field any number, as the nearest <C>Float</C>.</li>
      </ul>
      <P>A number that doesn&apos;t fit its field is refused like any other value that doesn&apos;t fit, never rounded:</P>
      <Ex name="json/too-big" />
      <P>
        Decoding goes on after a problem, so <C>JsonError.Invalid</C> lists every place the text doesn&apos;t fit, in
        order: where it is (<C>path</C>), and what the type wanted there (<C>expected</C>). A service can answer
        with all of them at once:
      </P>
      <Ex name="json/sub-mismatch" />
      <H2>Invariants are checked too</H2>
      <P>
        A decoded value is checked against its type&apos;s <C>invariant</C>s, as any value is when it is made. Text
        from outside that breaks one is not a mistake in the program, so it is a problem in the list, not a panic:
      </P>
      <Ex name="json/decode-invariant" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>The problem&apos;s <C>path</C> is the record&apos;s: <C>[1]</C> is the second item.</li>
        <li>The invariants are checked once every value has the right shape. Until then, the problems are those of shape.</li>
      </ul>
      <H2>Fields your type doesn&apos;t have</H2>
      <P>
        A field the type doesn&apos;t have is a problem at its path, as the type&apos;s schema says. Another
        service&apos;s answers may gain fields over time, so to read them write{" "}
        <C>unknown: JsonUnknown.Ignore</C>, and such fields are skipped:
      </P>
      <Ex name="json/decode-unknown" />
      <P>
        <C>decode_value</C> takes <C>unknown</C> too. The <Link href="/docs/networks/http/">std.http</Link> client
        skips unknown fields in the responses it decodes, so a service can add fields without breaking its clients.
      </P>
      <H2>Dates and times</H2>
      <P>
        <C>std.time</C>&apos;s <C>Date</C>, <C>Time</C> and <C>DateTime</C> are ISO 8601 text in JSON, as their{" "}
        <C>iso()</C> writes them. A <C>DateTime</C> gets a <C>Z</C>: JSON holds it in UTC, so other programs read the
        right moment. An offset in the text is turned to UTC when it is read:
      </P>
      <Ex name="json/dates" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Decoding reads the text as <Link href="/docs/standard-library/time/#times-as-text">time.parse_date</Link> and <C>time.parse_time</C> do, so digits of a second past milliseconds are dropped.</li>
        <li>A <C>DateTime</C> may end in <C>Z</C>, in an offset, or in neither. Anything else, such as PostgreSQL&apos;s <C>infinity</C>, is a problem that says what was expected.</li>
        <li>In a schema, a <C>DateTime</C> has <C>&quot;format&quot;: &quot;date-time&quot;</C>, and a <C>Time</C> a <C>&quot;pattern&quot;</C>, since JSON Schema&apos;s <C>time</C> format needs an offset.</li>
      </ul>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="tests/conformance/02-types/json-dates-are-iso-text.kav" label="dates, tested" />
        <SrcRef file={`docs/spec/${JSON_SPEC}`} label="spec 2.12 · decoding" />
        <SrcRef file="tests/conformance/02-types/json-decode-refuses-unknown-fields.kav" label="unknown fields, tested" />
        <SrcRef file="docs/decisions/0096-unknown-fields-are-refused.md" label="decision 0096" />
        <SrcRef file="tests/conformance/02-types/json-decode-lists-every-problem.kav" label="every problem, tested" />
        <SrcRef file="tests/conformance/02-types/json-decode-checks-invariants.kav" label="invariants, tested" />
        <SrcRef file="docs/decisions/0086-decoding-reports-every-problem.md" label="decision 0086" />
      </div>
    </>
  ),
};
