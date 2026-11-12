import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { fileFigure, specBlockWith, specTables } from "@/lib/spec-tables";
import { Ex, H2, Note, P, Syntax } from "./parts";
import { ModuleTable, Table } from "./spec-table";

// ---------- HTTP ----------

// What spec 14.11 measured for http.serve on every core, read from its sentence.
const HTTP_FILE = "14-standard-library/11-http.md";
const HTTP_SPEC = `docs/spec/${HTTP_FILE}`;
const computes = () => fileFigure(HTTP_SPEC, /a handler that computes answered in ([\d.–]+ s instead of [\d.–]+ s on one core)/);
const idles = () => fileFigure(HTTP_SPEC, /one that does nearly nothing in ([\d–]+ ms instead of [\d–]+ ms)/);
/** Spec 14.11's table of what a form and its parts give. */
const formTable = () => {
  const t = specTables(HTTP_FILE, "14.11").find((x) => x.header[0] === "Written" && x.rows.some((r) => r[0].includes("form.next()")));
  if (!t) throw new Error("spec 14.11: no table of form.next()");
  return t;
};
const servedSetup = () => fileFigure(HTTP_SPEC, /measured on (Windows \(\d+ cores, \d+ clients making \d+ requests each on kept-open connections\))/);

export const httpDoc: LessonDoc = {
  lead: "std.http calls web APIs and serves them: HTTP/1.1 and HTTP/2, with HTTP/3 for calls, written in Kavach itself.",
  simple: <>HTTP is how apps ask a website for something and get an answer back, like a customer at a shop counter. With <C>std.http</C>, your program can be the customer, the shop, or both.</>,
  minutes: 6,
  before: ["standard-library/json", "concurrency/tasks"],
  body: (
    <>
      <Ex name="http/sub-server" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>hello</C> is the handler: it takes a <C>Request</C> and gives a <C>Response</C>.</li>
        <li><C>net.listen(&quot;127.0.0.1:0&quot;)</C> lets the system pick a free port; <C>listener.address()</C> says which.</li>
        <li><C>spawn http.serve(…)</C> serves in a task of its own, and stops when <C>main</C> returns: a server can&apos;t be left running by mistake.</li>
        <li><C>http.get</C> calls it, here from the same program; a real client is any other program, in any language.</li>
      </ul>
      <H2>Every core at work</H2>
      <P>
        <C>http.serve</C> runs each connection in a task of its own, spawned with <C>spawn cores</C>, so in a release
        build (<C>kavach build</C>) handlers run at once on every core. <C>kavach run</C> answers them on one core, in
        turn, with the same responses.
      </P>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>A handler that computes answered in {computes()}.</li>
        <li>A handler that does nearly nothing answered in {idles()} on one core.</li>
        <li>Measured on {servedSetup()}.</li>
        <li>Handlers share nothing but what they reach through a <C>Shared</C> value or a channel. Each handler&apos;s printed lines come out in its own order, whole.</li>
      </ul>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={HTTP_SPEC} label="spec 14.11 · serving" />
        <SrcRef file="tests/conformance/14-standard-library/http-serves-many-clients-on-every-core.kav" label="tested" />
      </div>
      <H2>Every function</H2>
      <ModuleTable file="11-http.md" number="14.11" first="Function" />
      <Note>
        Network access needs the <C>Net</C> capability, so a <C>pub</C> function that calls an API says{" "}
        <C>uses Net</C>, and an agent must list it in its <C>can</C>: see{" "}
        <Link href="/docs/agents/capabilities/">Capabilities</Link>.
      </Note>
    </>
  ),
  deeper: (
    <>
      <P>
        HTTP is written in Kavach, on top of <C>std.net</C>: HTTP/1.1, HTTP/2 with its header compression, and HTTP/3
        over QUIC. <C>https://</C> addresses go through TLS, offering HTTP/2 first. Programs that speak HTTP/2 or 3
        themselves, as gRPC does, use the parts in <C>std.http.h2</C> and <C>std.http.h3</C>.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/14-standard-library/11-http.md" label="spec 14.11 · std.http" />
        <SrcRef file="docs/decisions/0022-http2-and-http3.md" label="decision 0022" />
      </div>
    </>
  ),
  quiz: [
    {
      type: "choice",
      prompt: <>What happens to a server started with <C>spawn http.serve(…)</C> when the function that started it returns?</>,
      options: ["It is cancelled: tasks never outlive the function that started them", "It keeps running in the background", "The program waits for it forever"],
      answer: 0,
      explain: "A task belongs to the function that spawned it and is cancelled when that function returns (spec 9.2).",
    },
  ],
};

export const httpCallDoc: LessonDoc = {
  lead: "http.get and http.post call a URL and give its Response: the status, the headers and the body.",
  simple: <>Calling a URL is like posting a letter to an address and getting a reply. The reply says how it went (the status), carries notes on the envelope (the headers), and holds the letter itself (the body).</>,
  minutes: 3,
  body: (
    <>
      <Syntax code={'const page = try http.get("https://example.com/")\nprint(page.status)\nprint(page.body.text() ?? "")'} />
      <P>When a call can&apos;t work, the error says why. A refused connection and an address that isn&apos;t HTTP:</P>
      <Ex name="http/errors" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>Network(…)</C>: the connection failed, with the <C>NetError</C> inside.</li>
        <li><C>BadUrl</C>: not an <C>http://</C> or <C>https://</C> address.</li>
        <li><C>Status(code)</C>: the server answered, but not with success (from the JSON helpers).</li>
        <li><C>BadMessage</C>, <C>BadJson</C>: the answer was broken, or not the JSON you asked for.</li>
      </ul>
      <Note>
        Every network wait has a time limit, so a server that stops answering can&apos;t hang your program: see{" "}
        <Link href="/docs/networks/sockets/timeouts/">Timeouts</Link>.
      </Note>
    </>
  ),
};

export const httpServeDoc: LessonDoc = {
  lead: "A server is one function from Request to Response. Route by r.method and r.path; answer with http.text or http.json_response.",
  simple: <>A server is like a shop counter: each customer brings a request, and you hand back an answer. You look at what they ask for and where, then decide what to give them.</>,
  minutes: 4,
  body: (
    <>
      <Ex name="http/routes" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Each connection runs in a task of its own, so one slow client doesn&apos;t hold up the others.</li>
        <li>The status says what happened: <C>200</C> found, <C>201</C> made, <C>404</C> not found, <C>400</C> a bad request.</li>
      </ul>
      <H2>Headers</H2>
      <P>
        Headers are a list of <C>Header(name:, value:)</C>, in the order they are sent, names in lower case. A name
        may come more than once: two <C>set-cookie</C> lines stay two headers.
      </P>
      <Ex name="http/headers" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>r.header(name)</C> gives the first value of a name, or <C>none</C>. The name can be in any case.</li>
        <li><C>r.headers_named(name)</C> gives every value of a name, in order.</li>
      </ul>
      <H2>Limits</H2>
      <P>
        A server faces clients that are slow, large or hostile, so <C>http.serve</C> holds each one to{" "}
        <C>ServeLimits</C>. Every field has a safe default, so <C>http.serve(listener, handler)</C> is already
        protected. Change only the ones you need:
      </P>
      <Ex name="http/limits" />
      <Syntax code={specBlockWith(HTTP_FILE, "14.11", "pub type ServeLimits")} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>A request that breaks a limit gets its status and the connection is closed. The handler never sees it.</li>
        <li>A client that sends its headers a byte at a time holds a task for at most <C>header_timeout</C>.</li>
        <li>A request with <C>expect: 100-continue</C> gets <C>100 Continue</C> once its size is known to fit, before it sends the body.</li>
        <li>A 503 says <C>retry-after</C>, so well-behaved clients wait before trying again.</li>
      </ul>
      <H2>Stopping</H2>
      <P>
        On Ctrl+C or <C>SIGTERM</C> (how Docker and Kubernetes stop a container), <C>http.serve</C> answers new
        connections with 503, closes those waiting for their next request, and finishes the requests under way, for
        at most <C>drain_timeout</C>. Then the program ends, without cutting a response off. A second signal ends it
        at once. See <Link href="/docs/errors/panics/when-a-program-stops/">When a program stops</Link>.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={HTTP_SPEC} label="spec 14.11 · headers, limits, stopping" />
        <SrcRef file="tests/conformance/14-standard-library/http-headers-keep-repeats.kav" label="headers, tested" />
        <SrcRef file="tests/conformance/14-standard-library/http-serve-limits-refuse-what-is-too-much.kav" label="limits, tested" />
        <SrcRef file="docs/decisions/0087-http-headers-and-server-limits.md" label="decision 0087" />
      </div>
      <H2>A client that leaves</H2>
      <P>
        While a request is answered, <C>http.serve</C> keeps reading its connection. When the client closes it, the
        handler&apos;s task is cancelled, so a request nobody waits for stops using the server. A handler that
        panics gets 500, and the server goes on.
      </P>
      <SrcRef file="tests/conformance/14-standard-library/http-handlers-stop-when-the-client-leaves.kav" label="tested" />
      <H2>Large bodies</H2>
      <P>
        A body up to <C>buffered_body</C> (1 MB unless you say) is read whole into <C>r.body</C>. A larger one is
        left for the handler to read as it comes, a piece at a time, with <C>r.read_body()</C>, which gives{" "}
        <C>none</C> once it has all come. An upload of any size then needs no more memory than a piece:
      </P>
      <Ex name="http/upload" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>The 5-byte body fits in <C>buffered_body: 100</C>, so it is all in <C>r.body</C>, and <C>read_body</C> has nothing more.</li>
        <li>The 3000-byte body is not in <C>r.body</C>: the handler reads it with <C>read_body</C>.</li>
        <li>A read fails with a <C>NetError</C> when the client goes away during the upload.</li>
      </ul>
      <H2>Sending a file</H2>
      <P>
        <C>http.file(path)</C> answers with a file as it is read, so a file of any size is sent without being held in
        memory. Its <C>content-type</C> comes from the file&apos;s extension, and its <C>content-length</C> from its
        size:
      </P>
      <Ex name="http/file" />
      <P>
        To send something else as it is read, give a <C>Response</C> a reader from <C>fs.chunks</C> as its{" "}
        <C>stream</C>: <C>Response(status: 200, headers: [], body: Bytes([]), stream: reader)</C>.
      </P>
      <H2>Forms with files</H2>
      <P>
        An HTML form with files sends a <C>multipart/form-data</C> body. <C>http.form(r)</C> reads it a part at a
        time, and each part&apos;s data as it comes, so a large upload is never all in memory:
      </P>
      <Syntax code={specBlockWith(HTTP_FILE, "14.11", "const form = try http.form(r)")} />
      <Table header={formTable().header} rows={formTable().rows} />
      <P>A form that isn&apos;t <C>multipart/form-data</C>, has no boundary, or is cut off fails with <C>BadMessage</C>.</P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={HTTP_SPEC} label="spec 14.11 · large bodies, forms" />
        <SrcRef file="tests/conformance/14-standard-library/http-large-bodies-stream.kav" label="bodies, tested" />
        <SrcRef file="tests/conformance/14-standard-library/http-forms-read-part-by-part.kav" label="forms, tested" />
        <SrcRef file="docs/decisions/0089-streamed-http-bodies.md" label="decision 0089" />
        <SrcRef file="docs/decisions/0090-forms-read-part-by-part.md" label="decision 0090" />
      </div>
    </>
  ),
};

export const httpJsonDoc: LessonDoc = {
  lead: "post_json and get_json send and read your own types; read_json and json_response do it on the server. The type is the contract.",
  simple: <>The client and the server fill in the same form: your own type. Anything sent that doesn&apos;t fit the form is refused, on both sides, so neither side works with data it can&apos;t use.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="http/sub-json" />
      <P>
        The client and the server share <C>Order</C>. A body that isn&apos;t an <C>Order</C> is refused with status
        400 on the server, and with <C>BadJson</C> on the client. A status that isn&apos;t a success fails with{" "}
        <C>Status(code)</C>:
      </P>
      <Ex name="http/sub-status" />
    </>
  ),
};

export const httpSessionDoc: LessonDoc = {
  lead: "http.open keeps one connection open for many requests, instead of connecting again for each.",
  simple: <>Like staying on one phone call to ask several questions, instead of hanging up and dialling again for each one. Each question waits for its answer before the next.</>,
  minutes: 2,
  body: (
    <>
      <Ex name="http/session" />
      <P>
        Requests on a session go one after another, each waiting for its answer. A session is for one task at a time:
        give each task its own, so no task reads another&apos;s answer.
      </P>
    </>
  ),
};
