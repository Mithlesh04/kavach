import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import { Terminal } from "@/components/terminal";
import type { LessonDoc } from "@/lib/chapter";
import { loadProject } from "@/lib/samples";
import { Ex, H2, Note, P, Syntax } from "./parts";
import { Limits } from "./limits";
import { ModuleTable } from "./spec-table";

// ---------- Sockets ----------

export const socketsDoc: LessonDoc = {
  lead: "std.net speaks the protocols under HTTP: TCP connections, UDP datagrams, TLS encryption and QUIC.",
  simple: <>These are the basic ways programs talk across a network. TCP is like a phone call that keeps everything in order, UDP like postcards that may get lost, and TLS locks the conversation so nobody else can read it.</>,
  minutes: 5,
  before: ["networks/http"],
  body: (
    <>
      <Ex name="net/echo" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>conn.read()</C> gives the bytes that came, or <C>none</C> once the other side has closed: <C>?? break</C> ends the loop then.</li>
        <li>While a task waits for the network, the program&apos;s other tasks run.</li>
        <li>Addresses are text, <C>&quot;host:port&quot;</C>; port <C>0</C> asks the system for a free one.</li>
      </ul>
      <H2>Every function</H2>
      <ModuleTable file="10-net.md" number="14.10" first="Function or method" />
      <P>A failed operation gives a <C>NetError</C>: <C>Refused(address)</C>, <C>TimedOut(address)</C> or <C>Failed(address, reason)</C>.</P>
      <H2>Limits and sizes</H2>
      <Limits id="network" />
    </>
  ),
};

export const tcpDoc: LessonDoc = {
  lead: "TCP is a connection: bytes arrive complete and in order. net.listen and accept on one side, net.connect on the other.",
  simple: <>TCP is like a phone call: one side waits for calls, the other dials, and then words arrive in the order they were said. It is one long stream, so the two sides must agree how to tell where each message ends.</>,
  minutes: 2,
  body: (
    <>
      <Ex name="net/echo" />
      <P>TCP carries a stream of bytes, not messages: a protocol on top of it, as HTTP is, says where one message ends.</P>
    </>
  ),
};

export const udpDoc: LessonDoc = {
  lead: "UDP sends datagrams: single messages with no connection, fast, but each may be lost, doubled or come out of order.",
  simple: <>UDP is like sending postcards: no call to set up, so it is quick, but a card can get lost or arrive late. It suits games and live audio, where a late answer is useless anyway.</>,
  minutes: 2,
  body: (
    <>
      <Ex name="net/udp" />
      <P>
        Each datagram says who sent it, in <C>from</C>, so a server can answer. Use UDP where a late answer is
        worthless anyway, as in games and live audio; use TCP or QUIC when every byte must arrive.
      </P>
    </>
  ),
};

export const tlsDoc: LessonDoc = {
  lead: "net.connect_tls encrypts a connection and checks the server's certificate; there is no way to skip the check.",
  simple: <>TLS is like a sealed envelope plus an ID check: nobody on the way can read the messages, and the certificate proves the server is who it says it is.</>,
  minutes: 3,
  body: (
    <>
      <Syntax
        code={
          'const conn = try net.connect_tls("example.com:443", protocols: ["h2", "http/1.1"])\n\n// a company\'s own certificate authority, and a client certificate (mutual TLS)\nconst inside = try net.connect_tls("billing.internal:443", roots: ca, cert: mine, key: my_key)\n\n// a server\nconst listener = try net.listen_tls("0.0.0.0:443", cert, key, clients: ca)'
        }
      />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>TLS 1.3, or 1.2 for servers without 1.3. The server&apos;s certificate must be valid for the host name, checked against the root certificates built into Kavach and any you add in <C>roots</C>.</li>
        <li><C>protocols</C> offers protocols to the server (ALPN); <C>conn.protocol()</C> says which one it chose.</li>
        <li>With <C>clients</C>, a server accepts only clients whose certificate one of those authorities signed. <C>conn.client_cert()</C> then gives the names on it, <C>ClientCert(common_name:, dns:, uris:)</C>, to allow callers by name; a SPIFFE id such as <C>spiffe://shop/orders</C> is one of the <C>uris</C>.</li>
        <li><C>key</C> takes a <Link href="/docs/agents/secrets/">secret</Link>, so a key read with <C>env</C> or kept in a <C>secret</C> field needs no <C>reveal</C>.</li>
        <li>A slow or silent client can&apos;t hold up the others: handshakes go on side by side.</li>
      </ul>
      <Note>
        A TLS example needs certificates, which expire, so this one isn&apos;t run on this page. The test below makes a
        fresh certificate authority and runs exactly this, mutual TLS included, every time Kavach&apos;s tests run.
      </Note>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="tools/cli/tests/tls.rs" label="the TLS test" />
        <SrcRef file="tests/conformance/08-capabilities/tls-keys-take-secrets.kav" label="secret keys, tested" />
        <SrcRef file="docs/decisions/0099-client-certificate-names.md" label="decision 0099" />
        <SrcRef file="docs/decisions/0021-tls-through-rustls.md" label="decision 0021" />
      </div>
    </>
  ),
};

export const timeoutsDoc: LessonDoc = {
  lead: "Every network wait gives up after 30 seconds. .timeout(d) sets a shorter or longer deadline for one call.",
  simple: <>A timeout is like deciding how long you will wait at a bus stop before you give up and walk. Without one, a program could wait forever for a server that never answers.</>,
  minutes: 2,
  body: (
    <>
      <Ex name="net/timeout" />
      <P>
        The server here never answers. Without a deadline a program could wait forever for it; with one, the call
        fails with <C>TimedOut</C> and the program decides what to do.
      </P>
    </>
  ),
};

// ---------- gRPC ----------

const proto = loadProject("grpc/proto", ["src/main.kav"]);

export const grpcDoc: LessonDoc = {
  lead: "gRPC services from your own Kavach types: no .proto file to write first, no code generator, and any gRPC library can call them.",
  simple: <>gRPC is a common way for programs to ask each other to do work, like sending a request form and getting a reply form back. In Kavach, your own types are the forms, so programs in other languages can call yours.</>,
  minutes: 5,
  before: ["networks/http", "types/records"],
  body: (
    <>
      <Ex name="grpc/sub-call" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>The handler reads the request into an <C>Order</C> and answers with a <C>Placed</C>. The types are the messages.</li>
        <li><C>grpc.failed(3, …)</C> answers with a gRPC status; 3 is <C>INVALID_ARGUMENT</C>. The client gets it as <C>Status(code, message)</C>.</li>
        <li><C>http://</C> means HTTP/2 without encryption, as inside a company&apos;s network; <C>https://</C> uses TLS.</li>
      </ul>
      <H2>Every function</H2>
      <ModuleTable file="14-grpc.md" number="14.14" first="Function" />
    </>
  ),
  deeper: (
    <>
      <P>
        Messages are Protocol Buffers worked out from each type: field <C>n</C> of a record is its <C>n</C>th
        declared field. Calls are HTTP/2 <C>POST</C>s with the status in a trailer, as the gRPC standard says, so a
        Go, Java or Python service can call a Kavach one, and the other way round.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/14-standard-library/14-grpc.md" label="spec 14.14 · std.grpc" />
        <SrcRef file="docs/decisions/0027-grpc-as-a-module.md" label="decision 0027" />
      </div>
    </>
  ),
};

export const grpcServerStreamDoc: LessonDoc = {
  lead: "The server answers with many messages, and the client handles each as it arrives: order tracking, live prices, logs.",
  simple: <>Like a delivery app that keeps sending updates: one question, then many answers over time. The client deals with each answer as soon as it comes, without waiting for the last one.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="grpc/sub-server-stream" />
      <P><C>grpc.reply_stream</C> sends a list as one message each; <C>grpc.each</C> runs its function for every message as it comes, not after the last one.</P>
    </>
  ),
};

export const grpcClientStreamDoc: LessonDoc = {
  lead: "The client sends many messages on one call, and the server answers once: uploads, batches, totals.",
  simple: <>Like handing a cashier your items one by one and getting one receipt at the end. The client sends many messages, and the server answers once.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="grpc/sub-client-stream" />
      <P><C>grpc.read_all</C> gives every message the client sent, in order. A handler written for one message still works: <C>grpc.read</C> gives the first.</P>
    </>
  ),
};

export const grpcTwoWayDoc: LessonDoc = {
  lead: "Two-way streams send many messages each way; a conversation answers each message as it arrives.",
  simple: <>Like a conversation on the phone: both sides send messages, and each side answers what the other just said.</>,
  minutes: 4,
  body: (
    <>
      <Ex name="grpc/sub-two-way" />
      <P>With <C>each_stream</C>, the client sends everything, then reads the answers. For a real back-and-forth, hold a conversation: each side answers the other&apos;s last message.</P>
      <Ex name="grpc/sub-talk" />
      <P>The client&apos;s function gives the next messages to send, or <C>none</C> to end its side.</P>
    </>
  ),
};

export const grpcProtoDoc: LessonDoc = {
  lead: "kavach proto writes the .proto file for your types, so a service in any other language can generate matching code.",
  simple: <>Programs in other languages need a description of your messages in a format they all understand: a <C>.proto</C> file. Kavach writes it from your types, so a team using another language can make code that matches yours.</>,
  minutes: 2,
  body: (
    <>
      <CodeBlock code={proto.files[0].code} file="src/main.kav" />
      <Terminal session={proto.output ?? ""} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>A union becomes a message with a <C>oneof</C>; a list is <C>repeated</C>; a map with text keys is a <C>map</C>.</li>
        <li>Add new fields at the end of a record: older clients keep working. Reordering fields breaks them.</li>
      </ul>
    </>
  ),
};
