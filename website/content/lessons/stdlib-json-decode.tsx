import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, Files, H2, P } from "./parts";

// ---------- JSON › Decoding into a type ----------
//
// Spec 2.12: json.decode<T> reads JSON straight into a type, and
// json.decode_texts<T> reads a record from one text per field, as a CSV
// row gives them.

const JSON_SPEC = "02-types/12-json.md";

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
      <H2>Generic types</H2>
      <P>
        A generic record or union decodes with its type arguments put in for its parameters:{" "}
        <C>Box&lt;Yes&gt;</C> reads its <C>value</C> as a <C>Yes</C>, and <C>Box&lt;Int&gt;</C> as an <C>Int</C>:
      </P>
      <Ex name="json/decode-generic-type" />
      <SrcRef file="tests/conformance/02-types/json-decode-fills-in-a-generic-type.kav" label="tested" />
      <H2>Fields read from text</H2>
      <P>
        A CSV row, a form or a query string gives each field as text. <C>json.decode_texts&lt;T&gt;(names, texts)</C>{" "}
        makes a record straight from them, with no <C>Json</C> in between: each text is read as its field&apos;s type
        reads text. <C>json.encode_texts(value)</C> goes back, each field as <C>print</C> writes it:
      </P>
      <Ex name="json/decode-texts" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>A field not named takes its default, as <C>ok</C> does here, and the invariants are checked.</li>
        <li>A number is read strictly, so <C>0,5</C> is a problem at the field&apos;s path, as in <C>decode</C>.</li>
        <li>An empty text is <C>none</C> for an optional field. A float takes <C>NaN</C>, <C>inf</C> and <C>-inf</C>, and a <C>Float32</C> keeps its own digits.</li>
        <li><C>std.table</C> reads and writes every CSV row this way.</li>
      </ul>
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
      <H2>Types only their module may make</H2>
      <P>
        A record with a private or <C>pub(read)</C> field is{" "}
        <Link href="/docs/modules/modules/">made only in its own module</Link>, so that module keeps control of what
        the field holds. Decoding makes a value, so it follows the same rule: outside the type&apos;s module,{" "}
        <C>json.decode</C>, <C>decode_value</C> and <C>decode_texts</C> of such a type are refused when compiling,
        wherever the type sits inside <C>T</C>. Otherwise text from outside could set <C>balance</C> to anything:
      </P>
      <Files name="json/decode-private.fails" first={["src/main.kav", "src/bank.kav"]} outcome="refused" />
      <P>The type&apos;s own module may decode it, check it, and hand the value out:</P>
      <Files name="json/decode-own-module" first={["src/bank.kav", "src/main.kav"]} />
      <P>
        The same holds for the other ways text becomes a record:{" "}
        <Link href="/docs/standard-library/command-line-programs/">cli.parse&lt;T&gt;</Link> and{" "}
        <Link href="/docs/agents/ask/">ask&lt;T&gt;</Link>.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/10-modules-and-packages/01-modules.md" label="spec 10.1 · private fields" />
        <SrcRef file="tests/conformance/10-modules/decoding-makes-no-private-records/src/main.kav" label="refused, tested" />
        <SrcRef file="tests/conformance/10-modules/decoding-sets-no-pub-read-fields/src/main.kav" label="pub(read), tested" />
        <SrcRef file="tests/conformance/10-modules/decoding-in-the-types-own-module/src/main.kav" label="own module, tested" />
        <SrcRef file="tests/conformance/10-modules/parsing-makes-no-private-records/src/main.kav" label="cli.parse, tested" />
      </div>
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
