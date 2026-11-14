import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import { Terminal } from "@/components/terminal";
import type { LessonDoc } from "@/lib/chapter";
import { loadProject } from "@/lib/samples";
import { fileFigure, specBlockWith } from "@/lib/spec-tables";
import { Ex, Files, H2, Note, P, Syntax } from "./parts";
import { Limits } from "./limits";

const limits = loadProject("projects/limits.fails", ["kavach.toml", "src/main.kav"]);
const privateMethod = loadProject("modules/private-method", ["src/main.kav"]);

// ---------- Modules ----------

export const modulesDoc: LessonDoc = {
  lead: "One file is one module, a folder groups modules, and nothing leaves a module unless it is marked pub.",
  simple: <>A big program is split into files, like a big book into chapters, and folders group them like the parts of the book. Each file keeps what it holds to itself, unless it marks it <C>pub</C>.</>,
  minutes: 5,
  before: ["tools/project-layout"],
  body: (
    <>
      <Files name="modules/shop" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>src/billing/invoice.kav</C> is the module <C>billing.invoice</C>: the path is the name, so you always know which file a name comes from.</li>
        <li><C>use billing.invoice</C> brings it in; then <C>invoice.create(…)</C> calls its <C>pub</C> function.</li>
        <li>Everything is private until marked <C>pub</C>, fields included.</li>
      </ul>
      <Note>
        Private by default means you can change a module&apos;s inside freely: nothing outside can depend on what it
        doesn&apos;t make <C>pub</C>.
      </Note>
    </>
  ),
};

export const useDoc: LessonDoc = {
  lead: "use brings in a module, the standard library or a dependency; use a.b.{X, y} brings names in directly.",
  simple: <><C>use</C> is like borrowing from a neighbour&apos;s shed: once a tool is in your hands, you can use it. You can borrow the whole toolbox, or just the one tool you need.</>,
  minutes: 2,
  body: (
    <>
      <Syntax
        code={
          "use billing.invoice                    // then write invoice.create(…)\nuse billing.invoice.{Invoice, create}  // then write create(…)\nuse std.json                           // the standard library\nuse std.time.{Date, utc}               // a std type and function\nuse std.collections.Set as HashSet     // a new name"
        }
      />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Names are looked up in your own modules first, then <C>std</C>, then dependencies.</li>
        <li>Some names need no <C>use</C> at all: the built-in types, <C>print</C>, <C>debug</C>, <C>ask</C>, <C>panic</C>, <C>env</C>, <C>Shared</C> and <C>Channel</C>.</li>
        <li>A dependency is used by its name the same way: see <Link href="/docs/tools/packages/">Packages</Link>.</li>
        <li><C>{"use std.time.{Date, utc}"}</C> names a standard module&apos;s types and functions that are written in Kavach. A function the runtime provides can&apos;t be named alone, and the message says what to write:</li>
      </ul>
      <Ex name="modules/std-runtime-name.fails" />
      <SrcRef file="tests/conformance/14-standard-library/runtime-functions-are-not-named-alone.kav" label="tested" />
      <H2>A type through its module</H2>
      <P>
        Without bringing a type in by name, write it through its module, as you write its functions:{" "}
        <C>regex.Regex</C>, <C>regex.RegexError</C>, <C>fs.FsError</C>. That works for parameters, results and errors:
      </P>
      <Ex name="modules/std-type-path" />
      <SrcRef file="tests/conformance/10-modules/types-named-through-a-standard-module.kav" label="tested" />
      <H2>Re-exports</H2>
      <P>
        <C>pub use</C> makes the names it brings in part of the module too. So a package can declare each public type
        in a file of its own and show them all from its root:
      </P>
      <Files name="modules/re-export" first={["shop/src/lib.kav", "shop/src/orders.kav", "src/main.kav"]} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>A re-exported name is the same declaration, not a copy: <C>shop.Order</C> and <C>shop.orders.Order</C> are one type.</li>
        <li>Each name must be <C>pub</C> where it is declared, and a re-exported name can be re-exported again.</li>
        <li>Only names are re-exported, not a whole module:</li>
      </ul>
      <Ex name="modules/re-export-module.fails" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="tests/conformance/10-modules/re-exports-show-names-from-other-files/src/main.kav" label="tested" />
        <SrcRef file="tests/conformance/10-modules/re-exports-name-items.kav" label="names only, tested" />
        <SrcRef file="docs/decisions/0101-re-exports.md" label="decision 0101" />
      </div>
    </>
  ),
};

export const pubDoc: LessonDoc = {
  lead: "pub makes a type, a function or a field visible to other modules. Without it, only the module itself sees it.",
  simple: <><C>pub</C> is like a shop window: what is in it, everyone outside can see and use. What isn&apos;t stays in the back room, where only the shop itself can touch it, so the shop stays in charge of it.</>,
  minutes: 3,
  body: (
    <>
      <P>
        A record with a private field can be made only in its own module, so that module stays in charge of what the
        field holds. Another module that tries is refused:
      </P>
      <Files name="modules/private.fails" outcome="refused" />
      <P>The fix is the one the message gives: a <C>pub fn</C> in <C>billing.invoice</C> that makes an <C>Invoice</C>, as <C>create</C> does.</P>
      <H2>An extend in the same package</H2>
      <P>
        An <C>extend</C> of a type in another file of the <strong>same package</strong> counts as the type&apos;s own
        module. It may read, change and build the type&apos;s private fields, so a type&apos;s methods can be spread
        over files without making its fields <C>pub</C>:
      </P>
      <Files name="modules/extend-private" first={["src/lib.kav", "src/moves.kav"]} />
      <P>An <C>extend</C> in another package sees only <C>pub</C> fields.</P>
      <SrcRef file="tests/conformance/10-modules/an-extend-in-the-package-sees-private-fields/src/main.kav" label="tested" />
      <H2>Methods are private to their package</H2>
      <P>
        A method without <C>pub</C> can be called anywhere in its own <strong>package</strong>, so the package&apos;s
        modules can share helper methods without making them part of what it offers. Here <C>pricing</C> calls{" "}
        <C>tax</C>, and the program that uses <C>shop</C> can&apos;t:
      </P>
      <Files name="modules/private-method" first={["shop/src/lib.kav", "shop/src/pricing.kav", "src/main.kav"]} terminal={false} />
      <Terminal session={privateMethod.output ?? ""} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>The same holds on the type: <C>Order.blank()</C> is refused outside <C>shop</C> when <C>blank</C> isn&apos;t <C>pub</C>.</li>
        <li>A method an interface asks for can be called wherever the interface can, <C>pub</C> or not.</li>
      </ul>
      <SrcRef file="tests/conformance/10-modules/private-methods-stay-in-their-package/src/main.kav" label="tested" />
      <H2>Agents too</H2>
      <P>
        An agent is a declaration like any other. Mark it <C>pub</C>, and other modules can start it and call its
        handlers:
      </P>
      <Files name="modules/pub-agent" first={["src/bots.kav", "src/main.kav"]} />
      <P>Without <C>pub</C>, the agent stays inside its module:</P>
      <Files name="modules/private-agent.fails" first={["src/bots.kav", "src/main.kav"]} outcome="refused" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="tests/conformance/10-modules/pub-agents-start-from-other-modules/src/main.kav" label="pub agents, tested" />
        <SrcRef file="tests/conformance/10-modules/private-agents-stay-private/src/main.kav" label="private agents, tested" />
      </div>
    </>
  ),
};

// ---------- Packages ----------

const dep = loadProject("projects/dep", ["kavach.toml", "geometry/src/lib.kav", "src/main.kav"]);
const refused = loadProject("projects/grant.fails", ["kavach.toml", "greeter/src/lib.kav", "src/main.kav"]);
const granted = loadProject("projects/grant-add", ["kavach.toml"]);
const partGrant = loadProject("projects/grant-part.fails", ["kavach.toml", "notes/src/lib.kav", "src/main.kav"]);

// The spec's example of sources that are and aren't fetched, kept word for word.
const careToml = () =>
  fileFigure("docs/spec/10-modules-and-packages/12-dependencies.md", /\n {2}```toml\r?\n([\s\S]*?)\r?\n {2}```/).replace(/^ {2}/gm, "");

const workspace = loadProject("projects/workspace", ["kavach.toml", "packages/prices/src/lib.kav", "packages/shop/kavach.toml", "packages/shop/src/main.kav"]);
const WORKSPACES = "10-modules-and-packages/14-workspaces.md";

export const workspacesDoc: LessonDoc = {
  lead: "A workspace is several packages in one repository, checked, tested and built together, with one lock for all of them.",
  simple: <>A workspace is like one workshop with several benches: each package has its own bench and tools, but they share one store room, so everyone works with the same parts.</>,
  minutes: 3,
  before: ["tools/packages"],
  body: (
    <>
      <P>
        The <C>kavach.toml</C> at the repository&apos;s root lists the members. <C>packages/*</C> is every folder in{" "}
        <C>packages/</C> that has a <C>kavach.toml</C>:
      </P>
      <CodeBlock code={workspace.files[0].code} file="kavach.toml" lang="toml" />
      <CodeBlock code={workspace.files[1].code} file={workspace.files[1].path} />
      <CodeBlock code={workspace.files[2].code} file={workspace.files[2].path} lang="toml" />
      <CodeBlock code={workspace.files[3].code} file={workspace.files[3].path} />
      <P>At the root, with no file, <C>kavach check</C>, <C>kavach test</C> and <C>kavach build</C> run on every member, and say which member each line is about. <C>pens</C> is tested from its <C>example</C> and from 200 generated inputs that keep its <C>require</C>:</P>
      <Terminal session={workspace.output ?? ""} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Members run in the order <C>members</C> lists them, and the command fails when any member fails. In a member&apos;s folder, a command runs on that member alone.</li>
        <li>Members use each other by <C>path</C>, as any package does: <C>{"prices = { path = \"../prices\" }"}</C>.</li>
        <li><C>kavach test</C> runs a package&apos;s own tests only: <C>shop</C> uses <C>prices</C>, but doesn&apos;t test <C>pens</C> again.</li>
        <li>Every member&apos;s git dependencies are fetched once, into the root&apos;s <C>kavach_modules/</C>, and recorded in the root&apos;s <C>kavach.lock</C>, so all members use the same commit of each.</li>
        <li>A member&apos;s settings (<C>[build]</C>, <C>[test]</C>, <C>[env]</C>) are its own. The root may be a package too, with a <C>[package]</C> of its own.</li>
        <li>A package in a folder of a git repository is named with <C>dir</C>: <C>{"router = { git = \"https://…\", dir = \"packages/router\" }"}</C>.</li>
      </ul>
      <CodeBlock code={specBlockWith(WORKSPACES, "10.14", "# one for the whole workspace")} file="my-repo/" lang="text" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={`docs/spec/${WORKSPACES}`} label="spec 10.14 · workspaces" />
        <SrcRef file="tools/cli/tests/workspaces.rs" label="tested" />
        <SrcRef file="docs/decisions/0091-workspaces.md" label="decision 0091" />
      </div>
    </>
  ),
};

export const packagesDoc: LessonDoc = {
  lead: "A package is a library another project uses: name it in [dependencies], from a folder or a git repository, and use it like any module.",
  simple: <>A package is like a toolbox a friend built: you say where it is, once, in your project&apos;s settings, and from then on you use its tools as if they were your own — without copying them into your box.</>,
  minutes: 4,
  before: ["tools/modules"],
  body: (
    <>
      <CodeBlock code={dep.files[0].code} file="kavach.toml" lang="toml" />
      <CodeBlock code={dep.files[2].code} file={dep.files[2].path} also={[dep.files[1]]} output={dep.output} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>The name in <C>[dependencies]</C> is the module name: <C>use geometry</C> is the package&apos;s <C>src/lib.kav</C>, and <C>use geometry.shapes</C> would be its <C>src/shapes.kav</C>.</li>
        <li>Inside the package&apos;s own project, its <C>[package] name</C> names its modules the same way, so <C>shapes.kav</C> reaches the root&apos;s types with <C>{"use geometry.{Shape}"}</C> whichever project is built. <C>lib</C> names no module.</li>
        <li>Only its <C>pub</C> items can be used, as with <Link href="/docs/tools/modules/">any module</Link>.</li>
        <li>From a git repository: <C>{"money = { git = \"https://…\", tag = \"v1.0.0\" }"}</C>, or <C>branch</C> or <C>rev</C> instead of <C>tag</C>. <C>kavach add geometry --path ../geometry</C> and <C>kavach add money --git &lt;url&gt;</C> write these lines for you.</li>
        <li>A fetched repository lands in <C>kavach_modules/money/</C> in your project, as npm keeps <C>node_modules/</C>. Never commit that folder — <C>kavach new</C> puts it in <C>.gitignore</C>. A path dependency is read where it is, not copied.</li>
        <li><C>kavach.lock</C> records the exact commit of each git dependency. Commit it: every build then uses the same code, and the lock is enough to fetch <C>kavach_modules/</C> again. <C>kavach update</C> fetches again and rewrites it.</li>
        <li>A package whose code uses <C>unsafe</C> is refused until your entry allows it: <C>{"unsafe = true"}</C> in its line.</li>
        <li>Warnings in a dependency are its authors&apos; to fix, so they are not shown. Errors are.</li>
      </ul>
      <Note>
        There is no package registry yet, so a version number such as <C>{"stripe = \"1.2\""}</C> is an error that
        says so. The registry, with versions and signing, is an open question in the spec.
      </Note>
      <H2>Code from outside is handled with care</H2>
      <P>
        Fetching a dependency only copies its files: none of its code runs, because Kavach packages have no install
        scripts — where npm runs a package&apos;s install scripts on your machine. And what a line may name is checked
        before git is asked:
      </P>
      <CodeBlock code={careToml()} file="kavach.toml" lang="toml" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>A git source must be <C>https://</C>, <C>ssh://</C>, <C>user@host:path</C> or a folder. <C>http://</C>, <C>git://</C> and other transports are refused: they are unencrypted or run commands.</li>
        <li>A URL holding a password or token is refused, because <C>kavach.toml</C> and <C>kavach.lock</C> are committed. git finds the credentials for a private repository itself, from a credential helper or an SSH key.</li>
        <li>A fetched package can&apos;t name a <C>path</C> outside its own folder, and a name Windows keeps for a device (<C>con</C>, <C>nul</C>, <C>com1</C>…) is refused, so every project builds on every system.</li>
      </ul>
      <H2>Each package gets only the capabilities you grant</H2>
      <P>
        A dependency may use only the <Link href="/docs/agents/capabilities/">capabilities</Link> its entry lists in{" "}
        <C>can</C>. An entry without <C>can</C> grants none, so the package must be pure: it can&apos;t read files,
        reach the network or print. Here <C>greeter</C> prints, and its entry grants nothing:
      </P>
      <CodeBlock code={refused.files[1].code} file="greeter/src/lib.kav" />
      <CodeBlock code={refused.files[0].code} file="kavach.toml" lang="toml" output={refused.output} outcome="refused" />
      <P>
        <C>kavach add</C> reads what a package uses, says so, and writes it into <C>can</C>, so the person adding it
        sees what it may do:
      </P>
      <Terminal session={granted.output ?? ""} />
      <CodeBlock code={specBlockWith("10-modules-and-packages/12-dependencies.md", "10.12", "can = [")} file="kavach.toml" lang="toml" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>The error names the function and the capability, so you can decide whether to trust the package with it.</li>
        <li>A dependency&apos;s own dependencies get at most what it was granted: a package granted only <C>Console</C> can&apos;t pass <C>Net</C> on to another.</li>
        <li>An update that starts using a new capability fails to build until you grant it, so it can&apos;t quietly start reaching the network.</li>
      </ul>
      <P>
        <C>can</C> may grant part of a capability, as <C>uses</C> names it (<Link href="/docs/agents/capabilities/narrower-file-access/">narrower file access</Link>). With <C>{'can = ["Fs.read"]'}</C>,{" "}
        <C>notes</C> may read files but not write them:
      </P>
      <CodeBlock code={partGrant.files[1].code} file="notes/src/lib.kav" />
      <CodeBlock code={partGrant.files[0].code} file="kavach.toml" lang="toml" output={partGrant.output} outcome="refused" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="tests/conformance/10-modules/dependencies-use-only-granted-capabilities/src/main.kav" label="refused, tested" />
        <SrcRef file="tests/conformance/10-modules/granted-capabilities-let-a-dependency-act/src/main.kav" label="granted, tested" />
        <SrcRef file="tests/conformance/10-modules/dependencies-of-dependencies-get-no-more/src/main.kav" label="no more for theirs, tested" />
        <SrcRef file="tests/conformance/10-modules/grants-can-be-part-of-a-capability/src/main.kav" label="part of a capability, tested" />
        <SrcRef file="docs/decisions/0084-environment-variables-and-granted-capabilities.md" label="decision 0084" />
      </div>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/10-modules-and-packages/12-dependencies.md" label="spec 10.12 · dependencies" />
        <SrcRef file="tests/conformance/10-modules/dependencies-come-from-a-folder/src/main.kav" label="tested" />
        <SrcRef file="tools/cli/tests/packages.rs" label="every refusal tested" />
      </div>
    </>
  ),
};

// ---------- Small Files ----------

export const smallFilesDoc: LessonDoc = {
  lead: "A file may have at most 500 lines, and a function about 80: small files are easier to read, review, test and give to an AI.",
  simple: <>A long file is hard to read, like a book with no chapters. Kavach limits how long a file or a function may be, so code stays easy to read, review and test, for people and for AI.</>,
  minutes: 3,
  before: ["tools/modules"],
  body: (
    <>
      <P>The limits are set in <C>kavach.toml</C>. This project sets a tiny one, to show what happens:</P>
      <CodeBlock code={limits.files[0].code} file="kavach.toml" lang="toml" />
      <CodeBlock code={limits.files[1].code} file="src/main.kav" output={limits.output} outcome="refused" />
      <Limits id="files" />
      <Note>
        Every line counts, comments and blank lines too: a rule that skipped comments would push people to delete
        useful ones. Generated files are never counted. See <Link href="/docs/tools/settings/">Settings</Link> for the{" "}
        <C>[limits]</C> keys.
      </Note>
      <div><SrcRef file="docs/spec/12-maintainability.md" label="spec 12.1 · file and function length" /></div>
    </>
  ),
};

// ---------- Tools for AI ----------

const outline = loadProject("tools/ai-outline", ["src/main.kav"]);
const query = loadProject("tools/ai-query", ["src/main.kav"]);
const context = loadProject("tools/ai-context", ["src/main.kav"]);

export const aiToolsDoc: LessonDoc = {
  lead: "AI assistants pay for every token they read. Kavach hands them the part of the code they need, instead of whole files.",
  simple: <>An AI assistant pays for every piece of code it reads. Instead of handing it the whole book, Kavach hands it the table of contents and the one page it needs, which costs far less.</>,
  minutes: 4,
  before: ["functions/signatures"],
  body: (
    <>
      <P>
        Most of an assistant&apos;s tokens go on finding and reading code, not on writing the change. A Kavach signature
        already says what a function takes, gives, can fail with, can touch and promises, so the body is rarely needed.
      </P>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <thead>
            <tr className="border-b border-line text-left text-soft">
              <th className="py-2 pr-4 font-bold">Command</th>
              <th className="py-2 font-bold">Gives</th>
            </tr>
          </thead>
          <tbody>
            {[
              ["kavach outline file.kav", "a file's declarations, without the bodies"],
              ["kavach query callers | uses | fails | type | find", "one exact answer per question, instead of a text search and many reads"],
              ["kavach context name --budget N", "a function, plus the types and signatures it uses, within a token budget"],
              ["kavach mcp", "all of these, served to AI assistants over MCP"],
            ].map(([cmd, gives]) => (
              <tr key={cmd} className="border-b border-line align-top">
                <td className="py-2 pr-4"><C>{cmd}</C></td>
                <td className="py-2 text-muted">{gives}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <P>The program they answer about:</P>
      <CodeBlock code={outline.files[0].code} file="src/main.kav" />
    </>
  ),
};

export const outlineDoc: LessonDoc = {
  lead: "kavach outline shows a file's declarations, contracts and doc comments, with every body left out.",
  simple: <><C>kavach outline</C> is like a book&apos;s table of contents: it shows what a file offers, the names and what each part promises, and leaves out the long details inside.</>,
  minutes: 2,
  body: (
    <>
      <Terminal session={outline.output ?? ""} outputLang="kavach" />
      <P>An assistant learns what the file offers for a fraction of the tokens the whole file would cost.</P>
    </>
  ),
};

export const queryDoc: LessonDoc = {
  lead: "kavach query answers one question exactly: who calls a function, what a type holds, which names match.",
  simple: <>Searching for a word finds it everywhere, even in notes and comments. <C>kavach query</C> is like asking a librarian instead of turning every page: you get the exact answer, such as who uses a function.</>,
  minutes: 2,
  body: (
    <>
      <Terminal session={query.output ?? ""} />
      <P>
        A text search for <C>total</C> would also find it in comments and strings, and an assistant would read files
        to sort them out. A query finds only real declarations and uses. <C>uses Console</C> lists the functions that
        can touch a capability, here only <C>main</C>, which prints; <C>fails BankError</C> lists those that can fail
        with an error.
      </P>
    </>
  ),
};

export const contextDoc: LessonDoc = {
  lead: "kavach context gives a function, then what it uses, within a budget of tokens.",
  simple: <>Like packing a suitcase with a weight limit: <C>kavach context</C> packs a function first, then the things it uses, and stops at the limit you set. An AI gets what it needs to change that function, and nothing else.</>,
  minutes: 2,
  body: (
    <>
      <Terminal session={context.output ?? ""} outputLang="kavach" />
      <P>Everything needed to change <C>report</C>, and nothing else.</P>
    </>
  ),
};

export const mcpDoc: LessonDoc = {
  lead: "kavach mcp serves these tools to AI assistants such as Claude Code or Cursor, over the Model Context Protocol.",
  simple: <>MCP is a common plug that AI assistants know how to use. <C>kavach mcp</C> offers Kavach&apos;s code tools through that plug, on your own machine, so your code never leaves it.</>,
  minutes: 2,
  body: (
    <>
      <Terminal commands session="kavach mcp" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>It runs on your machine and talks over standard input and output: your code never leaves the machine.</li>
        <li>Add it to an assistant as an MCP server whose command is <C>kavach mcp</C>.</li>
        <li>Inside a project, answers come from an index kept in <C>.kavach/index</C>, so they are quick.</li>
      </ul>
      <Note>The site also serves <Link href="/llms.txt">llms.txt</Link>: a short guide to the whole language that an assistant reads once.</Note>
    </>
  ),
};
