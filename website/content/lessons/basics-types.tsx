import Link from "next/link";
import { C } from "@/components/inline-code";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, H2, Note, P } from "./parts";
import { Limits } from "./limits";

// ---------- Data Types ----------

// Every built-in type (spec 2.1), then the types built from others; each
// row links to the page that teaches it.
const single: [string, string, string, string][] = [
  ["Int", "a whole number, 64 bits", "31, -7, 1_000_000", "/docs/basics/data-types/int/"],
  ["Int8 Int16 Int32", "a smaller whole number", "-128", "/docs/basics/data-types/sized-integers/"],
  ["UInt8 … UInt64", "a whole number that is never negative", "200", "/docs/basics/data-types/sized-integers/"],
  ["Float", "a number with a fraction, 64 bits", "1.68", "/docs/basics/data-types/float-and-float32/"],
  ["Float32", "a 32-bit float, what GPUs compute fastest", "0.5", "/docs/basics/data-types/float-and-float32/"],
  ["Decimal", "an exact decimal of any size: Kavach's big decimal, for money", "499.99", "/docs/basics/data-types/decimal/"],
  ["BigInt", "a whole number of any size", "2 ** 100", "/docs/basics/data-types/bigint/"],
  ["Bool", "true or false", "true", "/docs/basics/data-types/bool/"],
  ["Char", "one character", "'A', 'क'", "/docs/basics/data-types/char/"],
  ["String", "text", "\"Asha\"", "/docs/basics/strings/"],
  ["Bytes", "raw bytes, as in files and on the network", "\"hi\".bytes()", "/docs/standard-library/bytes/"],
  ["Json", "JSON whose shape is known only while running", "json.parse(text)", "/docs/standard-library/json/json-values/"],
];

const built: [string, string, string][] = [
  ["[T]", "a list of T", "/docs/collections/lists/"],
  ["[K: V]", "a map from keys K to values V", "/docs/collections/maps/"],
  ["Set<T>", "each T once", "/docs/collections/sets/"],
  ["(A, B)", "a tuple: a few values together", "/docs/collections/tuples/"],
  ["T?", "a T, or none", "/docs/types/optionals/"],
  ["type Name { … }", "a record of your own", "/docs/types/records/"],
  ["type Name = A | B", "a union: one of several cases", "/docs/types/unions/"],
  ["(name: T, …)", "an anonymous record", "/docs/types/anonymous-records/"],
  ["fn(A) -> B", "a function", "/docs/functions/functions-as-values/"],
  ["interface Name", "any type that keeps a promise", "/docs/types/interfaces/"],
];

export const dataTypesDoc: LessonDoc = {
  lead: "Every value has a type: whole numbers are Int, text is String, true and false are Bool, and so on.",
  simple: <>A type is the kind of a value, like sorting things into boxes marked &quot;numbers&quot;, &quot;text&quot; and &quot;yes or no&quot;. Knowing the kind of every value lets Kavach stop you from mixing them up by mistake.</>,
  minutes: 4,
  before: ["basics/variables"],
  body: (
    <>
      <Ex name="datatypes/overview" />
      <P>You didn&apos;t write a single type: the compiler works each one out from the value.</P>
      <H2>Built-in types</H2>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <thead>
            <tr className="border-b border-line text-left text-soft">
              <th className="py-2 pr-4 font-bold">Type</th>
              <th className="py-2 pr-4 font-bold">Holds</th>
              <th className="py-2 font-bold">Example</th>
            </tr>
          </thead>
          <tbody>
            {single.map(([t, what, ex, href]) => (
              <tr key={t} className="border-b border-line">
                <td className="py-2 pr-4"><Link href={href}><C>{t}</C></Link></td>
                <td className="py-2 pr-4 text-muted">{what}</td>
                <td className="py-2"><C>{ex}</C></td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <H2>Types built from other types</H2>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <tbody>
            {built.map(([t, what, href]) => (
              <tr key={t} className="border-b border-line">
                <td className="py-2 pr-4"><Link href={href}><C>{t}</C></Link></td>
                <td className="py-2 text-muted">{what}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <H2>No null: none instead</H2>
      <P>
        Kavach has no <C>null</C> that any value could secretly be. A value that may be missing has a{" "}
        <C>?</C> in its type, as in <C>String?</C>, and is <C>none</C> when missing. The compiler then makes you
        handle that case, here with <C>??</C>, which gives a fallback: see{" "}
        <Link href="/docs/types/optionals/">Optionals</Link>.
      </P>
      <P>
        JSON does have <C>null</C>. Read as a <C>Json</C> value it is <C>Null</C>, and a key that isn&apos;t there
        reads as <C>Null</C> too. Decoded into your own type, it becomes <C>none</C>:
      </P>
      <Ex name="datatypes/null" />
      <P>Writing <C>null</C> (or <C>nil</C>, <C>None</C>, <C>undefined</C>) in Kavach code is an error that says what to write:</P>
      <Ex name="habits/null.fails" />
      <Note>
        Where a <C>Json</C> value is expected,{" "}
        <C>none</C> becomes JSON&apos;s <C>null</C>; more on <Link href="/docs/standard-library/json/json-values/">Json values</Link>.
      </Note>
      <Note>
        Kavach never turns one number type into another behind your back. An <C>Int</C> plus a <C>Float</C> is an
        error until you say which you want: see <Link href="/docs/basics/data-types/converting-types/">Converting types</Link>.
      </Note>
      <H2>Limits and sizes</H2>
      <Limits id="numbers" />
    </>
  ),
  quiz: [
    {
      type: "choice",
      prompt: <>Which type should hold a price like 499.99?</>,
      options: ["Decimal", "Float", "Int"],
      answer: 0,
      explain: "Decimal is exact. A Float rounds in binary, so cents can drift.",
    },
  ],
};

export const intDoc: LessonDoc = {
  lead: "Int is a whole number, positive or negative, up to about nine quintillion.",
  simple: <>An <C>Int</C> is a whole number, with no part after the point: the kind you use to count people or pages. If it ever grows past its limit, the program stops with a message instead of giving a wrong number.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="datatypes/int.warns" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>_</C> groups digits so big numbers stay readable; it changes nothing.</li>
        <li><C>0xff</C> is hexadecimal and <C>0b1010</C> binary.</li>
        <li>Dividing two <C>Int</C>s gives an <C>Int</C>: <C>7 / 2</C> is <C>3</C>, and the compiler warns, since that is rarely what was meant (see <Link href="/docs/basics/operators/arithmetic/">Arithmetic</Link>). <C>%</C> gives the remainder.</li>
      </ul>
      <P>An <C>Int</C> that goes past its limit stops the program with a message; it never wraps around to a wrong number.</P>
    </>
  ),
};

export const sizedDoc: LessonDoc = {
  lead: "Int8 to Int32 and UInt8 to UInt64 hold whole numbers in less space, or only positive ones.",
  simple: <>These are smaller boxes for whole numbers: a <C>UInt8</C> holds only 0 to 255. In some languages a full box rolls over to 0, like an old car&apos;s mileage counter; Kavach stops with a message instead.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="datatypes/sized" />
      <P>A <C>UInt8</C> holds 0 to 255, an <C>Int8</C> -128 to 127. A number that can&apos;t fit is refused before the program runs, with the type to use instead:</P>
      <Ex name="datatypes/sized-too-big.fails" />
      <P>
        Going past the limit while the program runs stops it, at the exact step. C and Go would wrap 255 + 1 round
        to 0 and carry on with a wrong number; Kavach never does:
      </P>
      <Ex name="types/sub-overflow.panics" />
      <P>The smallest value has no positive twin, so even dividing it by -1 doesn&apos;t fit, and is caught the same way:</P>
      <Ex name="datatypes/narrow-div.panics" />
    </>
  ),
};

export const floatDoc: LessonDoc = {
  lead: "Float is a number with a fraction, in 64 bits; Float32 is the 32-bit kind that GPUs compute fastest.",
  simple: <>A <C>Float</C> is a number with a part after the point, such as a height or a temperature. It is stored in a way that keeps most decimals a tiny bit off, so for money, use <C>Decimal</C> instead.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="datatypes/float" />
      <P>
        A <C>Float</C> always prints with a point or an exponent (<C>1000000000.0</C>), so you can tell it from an{" "}
        <C>Int</C>. Floats are stored in binary, so most decimals are kept a tiny bit off; for money, use{" "}
        <Link href="/docs/basics/data-types/decimal/">Decimal</Link>.
      </P>
    </>
  ),
};

export const decimalDoc: LessonDoc = {
  lead: "Decimal holds decimal numbers exactly, with as many digits as they need: it is Kavach's big decimal. Use it for money.",
  simple: <>A <C>Decimal</C> keeps a number exactly as you wrote it, to the last digit, like an accountant&apos;s ledger. That is why prices and bills belong in it: the sums never drift by a tiny amount.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="datatypes/money" />
      <P>
        Write the type, <C>: Decimal</C>, and the number is kept exactly as written. A <C>Float</C> would compute{" "}
        <C>0.1 + 0.2</C> as <C>0.30000000000000004</C> when worked out while the program runs; a <C>Decimal</C> gives{" "}
        <C>0.3</C>:
      </P>
      <Ex name="types/sub-decimal" />
      <H2>Rounding</H2>
      <Ex name="datatypes/rounding" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>round(2)</C> rounds to 2 places, a half away from zero: <C>2.345</C> becomes <C>2.35</C>, <C>-2.345</C> becomes <C>-2.35</C>. This is how most people round by hand.</li>
        <li><C>round_even(2)</C> rounds a half to the even digit, &quot;banker&apos;s rounding&quot;: <C>2.345</C> becomes <C>2.34</C>, <C>2.355</C> becomes <C>2.36</C>. Over many sums, halves then go up as often as down, so totals don&apos;t drift upward. Banks and statistics use it.</li>
      </ul>
      <H2>As big as it needs to be</H2>
      <P>
        There is no separate <C>BigDecimal</C> type to learn: a <C>Decimal</C> grows by itself. Up to about 38 digits
        it is kept in a fast 128-bit form, and beyond that it keeps every digit, with no change to your code. A
        division that never ends keeps 18 digits after the point.
      </P>
    </>
  ),
};

export const bigintDoc: LessonDoc = {
  lead: "BigInt is a whole number of any size, worked out exactly.",
  simple: <>A <C>BigInt</C> is a whole number with no size limit, like a strip of paper that grows as long as the number needs. It is slower than <C>Int</C>, so use it only for really big numbers.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="types/sub-bigint" />
      <P>
        A constant too big for an <C>Int</C>, like the first two above, is a <C>BigInt</C> by itself. Turn an{" "}
        <C>Int</C> into one with <C>BigInt(i)</C>. It is slower than <C>Int</C>, so use it only when numbers may
        really get that large.
      </P>
    </>
  ),
};

export const boolDoc: LessonDoc = {
  lead: "Bool has two values, true and false. Comparisons give a Bool.",
  simple: <>A <C>Bool</C> is the answer to a yes-or-no question, like a light switch that is either on or off. &quot;Is the shop open?&quot; has the answer <C>true</C> or <C>false</C>.</>,
  minutes: 2,
  body: (
    <>
      <Ex name="datatypes/bool" />
      <P>The <Link href="/docs/basics/booleans/">Booleans</Link> page shows how to combine them.</P>
    </>
  ),
};

export const charDoc: LessonDoc = {
  lead: "Char is one character, written in single quotes: 'k', 'क'.",
  simple: <>A <C>Char</C> is one letter, digit or symbol, like one tile in a word game. Text is a row of such tiles, and the letters can come from any writing system.</>,
  minutes: 2,
  body: (
    <>
      <Ex name="datatypes/char" />
      <P>
        A <C>Char</C> can be any Unicode character. <C>Int(&apos;a&apos;)</C> gives its code, <C>97</C>, and{" "}
        <C>chars()</C> splits text into its characters.
      </P>
    </>
  ),
};

export const convertDoc: LessonDoc = {
  lead: "Convert a value with the name of the type you want: Float(n), Int(text), String(x).",
  simple: <>Sometimes you have a value of one kind and need another, such as the text &quot;2&quot; when you want the number 2. You ask for it by the type&apos;s name, and Kavach never changes a value&apos;s kind without being asked.</>,
  minutes: 4,
  body: (
    <>
      <P>Kavach doesn&apos;t mix number types on its own. This is refused, with the fix in the message:</P>
      <Ex name="types/sub-no-mixing.fails" />
      <P>Say what you want with the type&apos;s name:</P>
      <Ex name="types/sub-convert" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>Float(n)</C> always works. <C>Int(2.7)</C> cuts the fraction off: <C>2</C>.</li>
        <li>
          Text may not be a number, so <C>Int(text)</C> can fail, and you must say what happens then:{" "}
          <C>catch _ =&gt; 0</C> uses 0. Errors have their own part of the course.
        </li>
        <li>When the text is written in the code, as in <C>Int(&quot;2&quot;)</C>, the compiler reads it itself.</li>
      </ul>
      <P>Numbers written straight in the code may mix, because the compiler knows their exact values. Typed values say which way to go:</P>
      <Ex name="types/sub-mixing" />
    </>
  ),
};
