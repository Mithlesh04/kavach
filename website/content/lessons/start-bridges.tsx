import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { findNode } from "@/lib/nav";
import { otherSource } from "@/lib/samples";
import { Ex, H2, Note, P } from "./parts";
import { Cell } from "./spec-table";

// ---------- Coming From Another Language ----------
//
// One program, bridge/orders, written in Kavach (checked on every build) and
// in five other languages (run by hand: each prints the same three lines).
// A row of a table is [their habit, the Kavach way, the course page that
// teaches it]; `code` in backticks is shown as code.

type Row = [theirs: string, ours: string, page: string];

/** The href of a course page; an unknown key fails the build, so no link leads nowhere. */
function href(key: string): string {
  const node = findNode(key);
  if (!node) throw new Error(`Coming From Another Language: no course page ${key}`);
  return node.href;
}

function Changes({ language, rows }: { language: string; rows: Row[] }) {
  return (
    <div className="overflow-x-auto">
      <table className="w-full border-collapse text-[15px]">
        <thead>
          <tr className="border-b border-line text-left text-soft">
            <th className="py-2 pr-4 font-bold">In {language}</th>
            <th className="py-2 pr-4 font-bold">In Kavach</th>
            <th className="py-2 font-bold">Learn it</th>
          </tr>
        </thead>
        <tbody>
          {rows.map(([theirs, ours, page]) => (
            <tr key={theirs} className="border-b border-line align-top">
              <td className="py-2 pr-4"><Cell text={theirs} /></td>
              <td className="py-2 pr-4 text-muted"><Cell text={ours} /></td>
              <td className="py-2 whitespace-nowrap"><Link href={href(page)}>{findNode(page)?.title}</Link></td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

const shared: Row[] = [
  ["a `class` that inherits", "no inheritance: a field, or an interface", "types/classes-and-inheritance"],
  ["`null`, `nil` or `None`", "`none`, only in a type that ends in `?`", "types/optionals"],
  ["errors that pass up unseen", "the error type in the signature, `! OrderError`, and `try` at each call that passes it on", "errors/try"],
  ["two names for one list", "values: `b = a` copies, so changing `b` leaves `a` as it was", "memory/values-not-references"],
  ["any function may touch files or the network", "a function says what it may touch: `uses Fs`", "agents/capabilities"],
];

/** One language's page: its version of the program beside Kavach's, then what changes. */
function bridge(o: { language: string; file: string; lang: string; lead: string; rows: Row[]; also?: React.ReactNode }): LessonDoc {
  return {
    lead: o.lead,
    minutes: 6,
    before: ["start/coming-from-another-language"],
    body: (
      <>
        <P>
          Here is one small program twice: in {o.language} as you would write it, then in Kavach. Orders with a
          quantity and a price, a total that refuses an order with no quantity, and discount codes that may be
          missing. Both print the same three lines.
        </P>
        <CodeBlock code={otherSource(`bridge/${o.file}`)} file={o.file} lang={o.lang} />
        <Ex name="bridge/orders" />
        <H2>What changes</H2>
        <Changes language={o.language} rows={o.rows} />
        {o.also}
        <Note>
          Habits from other languages get a clear error that says what to write instead: see{" "}
          <Link href="/docs/start/classic-mistakes-fixed/#habits-from-other-languages">Habits from other languages</Link>.
        </Note>
      </>
    ),
  };
}

export const bridgeDoc: LessonDoc = {
  lead: "Coming from Python, JavaScript, C++, Rust or Go: the same small program in each language and in Kavach, and what changes.",
  simple: <>If you already program in another language, most of Kavach will look familiar. These pages put a program you know next to the same program in Kavach, and point at what works differently.</>,
  minutes: 3,
  body: (
    <>
      <P>Pick the language you know best. Each page shows one program in that language and in Kavach, then a table of what changes, with a link to the page that teaches each part.</P>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        {[
          ["python", "Python", "classes, exceptions and None"],
          ["javascript", "JavaScript", "classes, null, === and async"],
          ["cpp", "C++", "pointers, exceptions and undefined behaviour"],
          ["rust", "Rust", "borrowing, Result and Option"],
          ["go", "Go", "if err != nil, nil and goroutines"],
        ].map(([slug, name, what]) => (
          <li key={slug}>
            <Link href={href(`start/coming-from-another-language/${slug}`)}>{name}</Link>: {what}.
          </li>
        ))}
      </ul>
      <H2>What changes whichever language you know</H2>
      <Changes language="most languages" rows={shared} />
      <P>The program each page uses, in Kavach:</P>
      <Ex name="bridge/orders" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>total</C> says in its signature that it can fail with <C>OrderError</C>, and the caller decides what happens then: <C>catch</C> gives a value, <C>match</C> looks at each case.</li>
        <li><C>codes[&quot;WINTER&quot;]</C> may be missing, so its type is <C>Int?</C>, and <C>?? 0</C> says what to use then.</li>
      </ul>
      <H2>What you give up</H2>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Kavach is young. It has far fewer packages than pip, npm, Cargo or Go modules, so you lean on its standard library and on code you write.</li>
        <li>It runs on Linux and Windows on x86-64 today. macOS and ARM64 are planned for a later phase of the roadmap.</li>
        <li>Some habits are refused on purpose: there is no inheritance, no exception that passes up unseen, and no <C>null</C>.</li>
      </ul>
      <SrcRef file="docs/vision/roadmap.md" label="the roadmap" />
    </>
  ),
};

export const pythonDoc = bridge({
  language: "Python",
  file: "orders.py",
  lang: "python",
  lead: "From Python: records instead of classes, errors in the signature instead of exceptions, and none only where a type allows it.",
  rows: [
    ["`class Order:` with `@dataclass`", "`type Order { … }`, its methods inside it", "types/records"],
    ["`raise BadQty(…)` and `except`", "`fail OrderError.BadQty(item: …)`, with `! OrderError` in the signature, and `catch` or `match`", "errors/errors"],
    ["an exception passes up by itself", "`try` passes it on, so every call that can fail is marked", "errors/try"],
    ["`None`", "`none`, only in a type that ends in `?`", "types/optionals"],
    ["`codes.get(\"SPRING\", 0)`", "`codes[\"SPRING\"] ?? 0`", "collections/maps/reading-a-value"],
    ["`f\"total {x}\"`", "`\"total {x}\"`: every string can hold values", "basics/strings/interpolation"],
    ["`sum(o.cost() for o in orders)`", "`orders.sum(o => o.cost())`", "collections/lists/map-filter-sum"],
    ["`-7 // 2` is `-4`, rounded down", "`-7 / 2` is `-3`, towards zero; `div_floor` rounds down", "basics/operators/arithmetic"],
    ["`b = a` shares a list", "`b = a` copies it", "memory/values-not-references"],
    ["types checked by a separate tool, if at all", "every type checked before the program runs", "basics/data-types"],
    ["threads and the GIL, or `asyncio`", "`spawn` starts a task, `spawn cores` runs it on any core; there is no `async`", "concurrency/how-tasks-run"],
    ["`pip` and `requirements.txt`", "`kavach.toml`, and `kavach add`", "tools/packages"],
    ["blocks by indentation", "blocks in braces, `{ … }`", "basics/syntax"],
  ],
});

export const javascriptDoc = bridge({
  language: "JavaScript",
  file: "orders.js",
  lang: "javascript",
  lead: "From JavaScript: types checked before the program runs, == that compares contents, no null or undefined, and tasks instead of async.",
  rows: [
    ["`class Order` with a `constructor`", "`type Order { … }`: `Order(item: \"pen\", qty: 3, cents: 150)` makes one", "types/records"],
    ["`extends`", "no inheritance: a field, or an interface", "types/classes-and-inheritance"],
    ["`throw` anything, then `catch`", "`fail` with an error of the type the signature names, `! OrderError`", "errors/errors"],
    ["`null` and `undefined`", "`none`, only in a type that ends in `?`", "types/optionals"],
    ["`a === b`", "`a == b`, which compares contents", "basics/operators/comparison"],
    ["`new Map([[\"SPRING\", 10]])`", "`[\"SPRING\": 10]`", "collections/maps/making-a-map"],
    ["template strings, `${x}`", "`\"total {x}\"`: every string can hold values", "basics/strings/interpolation"],
    ["`orders.reduce((s, o) => s + o.cost(), 0)`", "`orders.sum(o => o.cost())`", "collections/lists/map-filter-sum"],
    ["every number is a float, and `Math.trunc`", "`Int` for whole numbers, whose `/` gives a whole number; `Decimal` for exact amounts", "basics/data-types"],
    ["objects and arrays shared by reference", "values: `b = a` copies", "memory/values-not-references"],
    ["`async`, `await` and promises", "`spawn` and `.wait()`, with no `async` and no `await`", "concurrency/tasks"],
    ["`npm` and `package.json`", "`kavach.toml`, and `kavach add`", "tools/packages"],
    ["types only with TypeScript", "every type checked before the program runs", "basics/data-types"],
  ],
});

export const cppDoc = bridge({
  language: "C++",
  file: "orders.cpp",
  lang: "cpp",
  lead: "From C++: native speed with no undefined behaviour, no manual memory, errors in the signature, and Drop where you would use RAII.",
  rows: [
    ["a `struct` or `class` with methods", "`type Order { … }`, its methods inside it", "types/records"],
    ["inheritance and `virtual`", "interfaces: a type says which ones it implements", "types/interfaces"],
    ["exceptions, unseen in the signature", "`! OrderError` in the signature, and `try` or `catch` at each call", "errors/errors"],
    ["`nullptr`, or `find()` against `end()`", "`T?`, and `?? 0` for when it's missing", "types/optionals"],
    ["`new`, `delete` and smart pointers", "no allocating or freeing: a value is freed after its last use", "memory/automatic-memory"],
    ["RAII destructors", "`Drop`: `drop` runs when the block holding the value ends", "memory/automatic-memory/freed-immediately"],
    ["signed overflow is undefined behaviour", "overflow stops the program with a panic", "basics/data-types/int"],
    ["`v[5]` past the end is undefined behaviour", "`xs[5]` panics; `xs.get(5)` gives `none`", "collections/lists/reading-by-index"],
    ["templates", "generics: `fn first<T>(items: [T]) -> T?`", "types/generics"],
    ["`std::thread` and `std::mutex`", "`spawn`, and `Shared<T>` for what tasks change together", "memory/shared-state"],
    ["headers and a build system", "modules with `use`, one `kavach.toml`, `kavach build`", "tools/modules"],
    ["raw pointers anywhere", "raw memory only inside `unsafe { … }`", "memory/unsafe"],
  ],
  also: (
    <P>
      The C++ version shows a trap of its own: <C>{'std::cout << "total " << total(broken)'}</C> prints{" "}
      <C>total</C> before <C>total</C> throws, so it is split into two statements.
    </P>
  ),
});

export const rustDoc = bridge({
  language: "Rust",
  file: "orders.rs",
  lang: "rust",
  lead: "From Rust: Result and Option are written ! E and T?, there is no borrowing to plan, and tasks need no async runtime.",
  rows: [
    ["`struct` and `impl`", "`type Order { … }` with its methods inside, or in an `extend`", "types/records"],
    ["traits", "interfaces", "types/interfaces"],
    ["`Result<i64, OrderError>`", "`-> Int ! OrderError`", "errors/errors"],
    ["`?`", "`try` before the call", "errors/try"],
    ["`Option<T>` and `unwrap_or(0)`", "`T?` and `?? 0`", "types/optionals"],
    ["`&`, `&mut`, lifetimes and `.clone()`", "none of them: values are copied, and `inout` marks a parameter a function changes", "memory/values-not-references"],
    ["`impl Drop`", "`type Guard: Drop`", "memory/automatic-memory/freed-immediately"],
    ["`HashMap::from([(\"SPRING\", 10)])`", "`[\"SPRING\": 10]`", "collections/maps/making-a-map"],
    ["`iter().map(…).sum()`", "`orders.sum(o => o.cost())`", "collections/lists/map-filter-sum"],
    ["`async` and a runtime such as Tokio", "`spawn`, with no `async`", "concurrency/tasks"],
    ["Cargo and crates", "packages in `kavach.toml`, each granted only the capabilities its `can` lists", "tools/packages"],
    ["`unsafe`", "`unsafe { … }` too, and `kavach audit` lists every use", "memory/unsafe"],
  ],
});

export const goDoc = bridge({
  language: "Go",
  file: "orders.go",
  lang: "go",
  lead: "From Go: try instead of if err != nil, no nil, interfaces a type names, and no garbage collector.",
  rows: [
    ["a `struct` with methods on a receiver", "`type Order { … }`, its methods inside it", "types/records"],
    ["interfaces met without saying so", "a type names the interfaces it implements: `type Guard: Drop`", "types/interfaces"],
    ["`(int, error)` and `if err != nil`", "`-> Int ! OrderError`, and `try` to pass the error on", "errors/try"],
    ["`nil`", "`none`, only in a type that ends in `?`", "types/optionals"],
    ["`codes[\"WINTER\"]` gives `0` when it's missing", "`codes[\"WINTER\"]` is an `Int?`: you say `?? 0`", "collections/maps/reading-a-value"],
    ["goroutines", "`spawn`; a task never outlives the function that started it", "concurrency/tasks-never-leak"],
    ["channels", "channels too: `put` and `take`", "concurrency/channels"],
    ["a garbage collector", "no collector: a value is freed after its last use", "memory/automatic-memory"],
    ["a capital letter exports a name", "`pub` exports it", "tools/modules/pub"],
    ["`fmt.Printf(\"%d\", n)`", "`print(\"{n}\")`", "basics/strings/interpolation"],
    ["`go.mod`", "`kavach.toml`", "tools/packages"],
  ],
});
