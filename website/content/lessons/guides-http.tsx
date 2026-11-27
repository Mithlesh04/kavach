import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import { Terminal } from "@/components/terminal";
import type { LessonDoc } from "@/lib/chapter";
import { guideProject } from "./guide-parts";
import { H2, Note, P } from "./parts";

// ---------- Guides › Write an HTTP Service ----------
//
// One task from start to finish: guides/stock-service.project, a JSON
// service that keeps a shop's stock. main serves it on a free local port
// and calls it, so the sample needs no network beyond this machine; the
// tests call the handler with no network at all.

const svc = guideProject("stock-service", ["src/main.kav", "src/stock.kav", "src/stock.test.kav", "kavach.toml"]);

export const guideHttpDoc: LessonDoc = {
  lead: "One task from start to finish: a small JSON service that keeps a shop's stock. It answers GET and POST, says clearly what is wrong, shares one value between requests, and is tested without a network.",
  simple: <>A web service is a program that waits for questions sent over the network and sends back answers. Here the question is &quot;how much tea is left?&quot;, and the answer is a little JSON.</>,
  minutes: 14,
  before: ["networks/http/serving", "networks/http/json-over-http", "memory/shared-state"],
  body: (
    <>
      <P>
        The service keeps a count for each item. <C>GET /stock/tea</C> gives tea&apos;s count; <C>POST /stock</C>{" "}
        with <C>{'{"item": "cake", "count": 5}'}</C> sets one. Every piece of code and every output on this page comes
        from one project the site runs.
      </P>
      <H2>1. The data, as a type</H2>
      <P>
        What goes over the wire is a record. The same type is read from a request&apos;s JSON and written into a
        response, so the two can&apos;t drift apart:
      </P>
      <CodeBlock code={svc.piece("src/stock.kav", "pub type Stock")} file="src/stock.kav" />
      <H2>2. One function answers every request</H2>
      <P>
        A handler is a plain function from a <C>Request</C> to a <C>Response</C>. Its routes are <C>if</C>s on the
        method and the path. Each mistake gets its own status and a sentence that says what to send instead:
      </P>
      <CodeBlock code={svc.piece("src/stock.kav", "pub fn handle")} file="src/stock.kav" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>An item it doesn&apos;t have is 404. A body that isn&apos;t a <C>Stock</C>, or a count below zero, is 400, and changes nothing.</li>
        <li><C>stock</C> is a <Link href="/docs/memory/shared-state/">Shared</Link> value: the one value every request sees. <C>update</C> changes it with no other request in the middle, so two requests at once never lose a change.</li>
        <li><C>handle</C> has no <C>uses</C>, so it can&apos;t read a file or open a connection; at most it could print, as every function may. The compiler holds it to that, so what it does is all on the page.</li>
      </ul>
      <H2>3. Serve it</H2>
      <P>
        <C>main</C> listens on a free port of this machine, serves <C>handle</C>, and then calls the service itself, as
        a client would:
      </P>
      <CodeBlock code={svc.piece("src/main.kav", "fn main")} file="src/main.kav" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>127.0.0.1:0</C> asks for any free port on this machine; <C>listener.address()</C> says which it got.</li>
        <li>The handler is a lambda that passes <C>stock</C> along with each request.</li>
        <li><C>http.serve</C> runs until the function that started it returns, which stops it: nothing is left running.</li>
        <li>In a <C>kavach build</C>, each connection is handled on any core of the machine, at once (see <Link href="/docs/networks/http/serving/">Serving</Link>).</li>
      </ul>
      <H2>4. Run it</H2>
      <Terminal session={svc.said("check")} />
      <Terminal session={svc.said("run src/main.kav")} />
      <P>
        <C>http.get_json&lt;Stock&gt;</C> reads the answer straight into a <C>Stock</C>, so the client is typed as well.
      </P>
      <H2>5. Test it without a network</H2>
      <P>
        A handler is a function, so a test calls it with a <C>Request</C> it builds, and no port is opened. A test file
        beside <C>stock.kav</C> is part of that module, so it sees <C>handle</C> and its imports:
      </P>
      <CodeBlock code={svc.file("src/stock.test.kav")} file="src/stock.test.kav" />
      <Terminal session={svc.said("test --seed 7")} />
      <Note>
        <C>handle</C> takes a <C>Request</C> and a <C>Shared</C>, which <C>kavach test</C> can&apos;t make up yet, so it
        says so instead of skipping it quietly: the two tests above are what check it.
      </Note>
      <H2>6. Before it faces the internet</H2>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Each client is already held to limits on time, size and connections, with safe defaults. Change them with <C>ServeLimits</C> (see <Link href="/docs/networks/http/serving/#limits">Limits</Link>).</li>
        <li><C>kavach build</C> makes one program to copy to a server; its runtime is inside it.</li>
        <li>Keep the stock somewhere that lasts, such as a database, when it must survive a restart: a <C>Shared</C> value lives as long as the program.</li>
      </ul>
      <H2>The whole project</H2>
      <CodeBlock code={svc.file("src/main.kav")} file="src/main.kav" />
      <CodeBlock code={svc.file("src/stock.kav")} file="src/stock.kav" />
      <CodeBlock code={svc.file("kavach.toml")} file="kavach.toml" lang="toml" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/14-standard-library/11-http.md" label="spec 14.11 · std.http" />
        <SrcRef file="docs/spec/06-memory.md" label="spec 6.2 · Shared" />
        <SrcRef file="website/content/samples/guides/stock-service.project/runs" label="the commands this page runs" />
      </div>
    </>
  ),
};
