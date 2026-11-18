import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import { Terminal } from "@/components/terminal";
import type { LessonDoc } from "@/lib/chapter";
import { loadProject, loadSample } from "@/lib/samples";
import { specBlockWith } from "@/lib/spec-tables";
import { Ex, H2, Note, P, Syntax } from "./parts";

const croot = loadProject("interop/croot.native", ["kavach.toml", "src/main.kav"]);
const numbers = loadProject("interop/numbers.native", ["src/main.kav"]);
const text = loadProject("interop/text.native", ["src/main.kav"]);
const callback = loadProject("interop/callback.native", ["src/main.kav"]);
const records = loadProject("interop/records.native", ["src/main.kav"]);
const byValue = loadProject("interop/by-value.native", ["src/main.kav"]);
const money = loadProject("interop/sprintf.native", ["src/main.kav"]);
const loadedUnsafe = loadProject("interop/loaded-unsafe.fails", ["src/main.kav"]);
const mathlib = loadSample("interop/mathlib.lib");
const audit = loadProject("agents/audit-cmd", ["src/main.kav"]);

/** A native project's main file, with what it printed. */
function Native({ run }: { run: ReturnType<typeof loadProject> }) {
  return <CodeBlock code={run.files[0].code} file="src/main.kav" output={run.output} />;
}

// ---------- Calling C ----------

export const callingCDoc: LessonDoc = {
  lead: "extern c declares the functions of a C library; Kavach calls them directly. Every call is unsafe, so you wrap it once in a safe function.",
  simple: <>A C library is a box of ready tools from another language. Kavach can use them but can&apos;t check them, like borrowed tools nobody has inspected, so you wrap each one once in a safe Kavach function, and the rest of the program uses that.</>,
  minutes: 5,
  before: ["memory/unsafe"],
  body: (
    <>
      <Syntax code={'extern c "library" {\n    fn c_name(param: CType) -> CType\n}'} />
      <CodeBlock code={croot.files[0].code} file="kavach.toml" lang="toml" />
      <Native run={{ ...croot, files: [croot.files[1]] }} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>&quot;m&quot;</C> is C&apos;s math library and <C>&quot;c&quot;</C> the C library itself; <C>&quot;z&quot;</C> would be zlib (<C>libz</C>, or <C>z.lib</C> on Windows).</li>
        <li>A C call can do anything to memory, so it goes inside <C>unsafe {"{ … }"}</C>, and the package says <C>unsafe = true</C>.</li>
        <li><C>cube_root</C> wraps the call: the rest of the program calls a safe Kavach function.</li>
      </ul>
      <H2>Types across the border</H2>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <thead>
            <tr className="border-b border-line text-left text-soft">
              <th className="py-2 pr-4 font-bold">Kavach</th>
              <th className="py-2 font-bold">C</th>
            </tr>
          </thead>
          <tbody>
            {[
              ["Int, UInt64", "int64_t, uint64_t"],
              ["Int32, Int16, Int8, UInt32 …", "the C integer of the same size"],
              ["Float", "double"],
              ["Bool", "bool"],
              ["String (a parameter)", "const char*: a copy ending in a zero byte, valid during the call"],
              ["Bytes (a parameter)", "const uint8_t*: a copy of the bytes, valid during the call; another argument says how many"],
              ["Ptr<T>", "T*: a pointer, also to a struct"],
              ["a record of these types", "a struct, by value, as C passes one (x86-64 for now)"],
              ["fn(…) -> … (a parameter)", "a function pointer, to a named Kavach function"],
              ["no ->", "void"],
            ].map(([k, c]) => (
              <tr key={k} className="border-b border-line align-top">
                <td className="py-2 pr-4"><C>{k}</C></td>
                <td className="py-2 text-muted">{c}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <Note>
        C functions run in native code: <C>kavach build</C> and <C>kavach run --native</C>. The interpreter stops at
        the first call of one and says to run the program natively.
      </Note>
      <div><SrcRef file="docs/spec/10-modules-and-packages/07-interop.md" label="spec 10.7 · calling C" /></div>
    </>
  ),
  quiz: [
    {
      type: "choice",
      prompt: <>Why must a call to a C function be inside <C>unsafe</C>?</>,
      options: ["C can do anything to memory, and Kavach can't check it", "C functions are slow", "C has no types"],
      answer: 0,
      explain: "Kavach checks its own code; C's it can't. unsafe marks the place, and kavach audit lists every one.",
    },
  ],
};

export const cNumbersDoc: LessonDoc = {
  lead: "Numbers and Bools cross to C exactly as C passes them, with no conversion.",
  simple: <>Numbers and true-or-false values mean the same thing in Kavach and in C, like coins that work in both countries. So they pass across as they are, with nothing to convert.</>,
  minutes: 2,
  body: <Native run={numbers} />,
};

export const cTextDoc: LessonDoc = {
  lead: "A String goes to C as a copy ending in a zero byte. Text C gives back comes as a Ptr<UInt8>, and .text() copies it.",
  simple: <>C marks the end of text with a zero, like a full stop at the end of a sentence. Kavach hands C a copy with that mark added, and <C>.text()</C> copies C&apos;s text back into Kavach text.</>,
  minutes: 3,
  body: (
    <>
      <Native run={text} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>strlen</C> counts bytes, as C does: <C>नमस्ते</C> is 6 characters but 18 bytes in UTF-8.</li>
        <li>The copy C gets is valid only during the call: C must not keep it.</li>
        <li><C>.text()</C> gives <C>none</C> for a null pointer, as <C>getenv</C> returns for a variable that isn&apos;t set.</li>
        <li>If the library says its text must be freed, freeing it is up to your program.</li>
      </ul>
    </>
  ),
};

export const cCallbacksDoc: LessonDoc = {
  lead: "A C function can call back into Kavach: pass the name of a Kavach function whose type matches C's function pointer.",
  simple: <>Sometimes a C function needs to ask you something while it works, like a sorting machine asking &quot;which of these two comes first?&quot;. You give it the name of a Kavach function it can call for the answer.</>,
  minutes: 3,
  body: (
    <>
      <Native run={callback} />
      <P>
        C&apos;s <C>qsort</C> calls <C>by_value</C> to compare two items. The callback must be a named function that
        can&apos;t fail: a lambda can&apos;t be passed, because C has nowhere to keep what it captures.
      </P>
    </>
  ),
};

export const cRecordsDoc: LessonDoc = {
  lead: "A record of C types is laid out as a C struct, and reaches C through a Ptr: Ptr<Point> is C's struct point*.",
  simple: <>A record is like a form with boxes. Kavach lays the boxes out exactly where C expects them, so C can read the form, and you hand C the form&apos;s address instead of the form itself.</>,
  minutes: 3,
  body: (
    <>
      <Native run={records} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Each field sits where C would put it: at the next offset that is a multiple of its size.</li>
        <li><C>ps + 1</C> is the address of the second <C>Point</C>, and <C>*second</C> reads it, as in C.</li>
        <li>A record read back through a <C>Ptr</C> is checked against its <C>invariant</C>s, like a new one.</li>
      </ul>
      <H2>Records by value</H2>
      <P>
        A record of C types also goes to C and comes back by value, as a C <C>struct</C> does. C&apos;s{" "}
        <C>lldiv</C> gives back a <C>struct</C> of two numbers, so it is declared to give a <C>LongDiv</C>:
      </P>
      <Native run={byValue} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>It is passed as the system&apos;s C passes a <C>struct</C>: in registers, on the stack, or through a hidden pointer.</li>
        <li>This works on x86-64 (Windows, Linux, macOS on Intel) for now; elsewhere it is a compile error.</li>
      </ul>
      <SrcRef file="tools/cli/tests/c_libraries.rs" line={187} symbol="records_go_to_a_c_library_by_value" label="by value, tested" />
    </>
  ),
};

export const cDotsDoc: LessonDoc = {
  lead: "A C function whose parameters end in ..., as printf's do, takes more arguments after its own: numbers, Ptrs and Strings.",
  simple: <>Some C functions, like <C>printf</C>, take as many values as you give them, with a format that says what each one is. Kavach passes them as C expects, but you must make each number the size the format says.</>,
  minutes: 3,
  body: (
    <>
      <P>C&apos;s <C>sprintf</C> writes cents as money. This ran with <C>kavach run --native</C>:</P>
      <Native run={money} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Each argument after <C>...</C> goes as its type is, after C&apos;s promotions: <C>Int8</C>, <C>Int16</C>, <C>UInt8</C> and <C>UInt16</C> become a C <C>int</C>, and a <C>Float32</C> a <C>double</C>.</li>
        <li>A number literal there is an <C>Int</C> (<C>int64_t</C>) or a <C>Float</C> (<C>double</C>). So write <C>Int32(42)</C> where C reads an <C>int</C> (<C>%d</C>), and use <C>%lld</C> for an <C>Int</C>.</li>
        <li>Only numbers, <C>Ptr</C>s and <C>String</C>s go after <C>...</C>; a record there is a compile error.</li>
      </ul>
      <Note>
        Spec 10.7&apos;s example uses <C>snprintf</C>, which also takes the buffer&apos;s size. Windows&apos; C library
        has <C>snprintf</C> only in its headers, so there <C>kavach run --native</C> says it can&apos;t call it, before
        running. <C>kavach build</C> links it. <C>sprintf</C>, <C>printf</C> and the rest of the family are found in
        the system&apos;s <C>msvcrt.dll</C>.
      </Note>
      <P>A Kavach function has no <C>...</C>: it takes more values in a list.</P>
      <Ex name="interop/dots-only-c.fails" />
      <SrcRef file="tests/conformance/10-modules/c-dots-take-numbers-pointers-and-text/src/main.kav" label="what goes after ..., tested" />
    </>
  ),
};

export const cLoadedDoc: LessonDoc = {
  lead: "extern c library Name { … } declares a library the program loads itself while running, for plugins and libraries that may be missing. Name.open(path) loads it.",
  simple: <>Most C libraries are joined to the program when it is built. A plugin is different: the program finds it while running, and it may not be there at all. So the program opens it, and is told plainly if it can&apos;t.</>,
  minutes: 3,
  body: (
    <>
      <CodeBlock code={specBlockWith("10-modules-and-packages/07-interop.md", "10.7", "extern c library")} file="main.kav, from spec 10.7" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>The library isn&apos;t linked. <C>Adder</C> is a type, and <C>Adder.open(path)</C> loads the library at <C>path</C>; a file name alone is looked for as the system does.</li>
        <li>It finds each function of the block in the library, and each is then a method of the value: <C>lib.add(2, 3)</C>.</li>
        <li>It fails with a <C>LibraryError</C>, whose text says which library couldn&apos;t be loaded and why, or which function it lacks.</li>
        <li>A loaded library stays loaded until the program ends.</li>
      </ul>
      <P>Loading runs the library&apos;s own start-up code, so <C>open</C> and each call go inside <C>unsafe</C>:</P>
      <CodeBlock code={loadedUnsafe.files[0].code} file="src/main.kav" output={loadedUnsafe.output} outcome="refused" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="tools/cli/tests/c_libraries.rs" line={60} symbol="a_program_loads_a_library_while_running" label="tested" />
        <SrcRef file="tests/conformance/10-modules/loaded-library-calls-need-unsafe/src/main.kav" label="unsafe, tested" />
      </div>
    </>
  ),
};

// ---------- A Library for Any Language ----------

export const libraryDoc: LessonDoc = {
  lead: "kavach build --lib turns Kavach code into a shared library with a C header, which Python, Node.js, Java, Go and C# can load like any other.",
  simple: <>Like a spare part that fits many makes of car, <C>kavach build --lib</C> turns your Kavach code into a piece other languages can plug in and use. You can write just one part of a program in Kavach.</>,
  minutes: 4,
  before: ["tools/modules/pub"],
  body: (
    <>
      <CodeBlock code={mathlib.code} file={mathlib.file} />
      <P><C>kavach build --lib mathlib.kav</C> writes the library and this header, word for word:</P>
      <CodeBlock code={mathlib.output ?? ""} file="out/mathlib.h" lang="c" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Each <C>pub(out)</C> function, the ones a library offers, becomes a C function named after the library: <C>mathlib_add</C>. What isn&apos;t <C>pub(out)</C> stays inside.</li>
        <li>Any language that can call C can call it: Python&apos;s <C>ctypes</C>, Node-API, JNI, cgo, P/Invoke.</li>
        <li>Text the library gives back is freed with <C>mathlib_free</C>; an error&apos;s reason comes from <C>mathlib_last_error</C>.</li>
      </ul>
      <Note>
        You don&apos;t have to rewrite a system to use Kavach in it: write the fast or safety-critical part in Kavach,
        and call it from the code you have. The repository&apos;s own test does exactly this from Python.
      </Note>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="tools/cli/tests/library.rs" line={93} symbol="other_languages_call_a_kavach_library" />
        <SrcRef file="docs/decisions/0026-a-library-for-other-languages.md" label="decision 0026" />
      </div>
    </>
  ),
};

// ---------- kavach audit ----------

export const auditDoc: LessonDoc = {
  lead: "kavach audit lists the places a reviewer must look first: unsafe code, revealed secrets, and what each function can touch.",
  simple: <>Like a checklist for a building inspector, <C>kavach audit</C> points to the few places that need a person&apos;s eyes: where safety checks are off, where secrets are let out, and what each function can reach.</>,
  minutes: 3,
  before: ["agents/secrets/reveal", "memory/unsafe"],
  body: (
    <>
      <CodeBlock code={audit.files[0].code} file="src/main.kav" />
      <Terminal session={audit.output ?? ""} />
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <thead>
            <tr className="border-b border-line text-left text-soft">
              <th className="py-2 pr-4 font-bold">Section</th>
              <th className="py-2 font-bold">Lists</th>
            </tr>
          </thead>
          <tbody>
            {[
              ["unsafe code", "every unsafe block: raw memory and C calls, which Kavach can't check"],
              ["revealed secrets", "every reveal(…), where a secret value is let out on purpose"],
              ["audited calls", "every call of an audited fn, which makes a checked value such as a sql\"…\" Query from text known only when the program runs"],
              ["capability use", "each function and the capabilities it uses: files, network, terminal, AI…"],
            ].map(([s, l]) => (
              <tr key={s} className="border-b border-line align-top">
                <td className="py-2 pr-4 whitespace-nowrap">{s}</td>
                <td className="py-2 text-muted">{l}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <P>
        Everything else the compiler has already checked. So a reviewer, or a security team, starts with this short
        list instead of reading the whole program: see <Link href="/docs/agents/secrets/reveal/">reveal</Link>,{" "}
        <Link href="/docs/memory/unsafe/">Unsafe</Link> and{" "}
        <Link href="/docs/tools/tagged-literals/#text-known-only-when-the-program-runs">audited fn</Link>.
      </P>
    </>
  ),
};
