import type { HeapObject, LaneState, Scene, SceneFrame } from "./types";

// One HTTPS request through a session: explain/https-get.shown.kav. It
// reaches a real server, so it is shown, not run (the build checks that it
// compiles). Checked in the source (2026-11-26):
// - std.http is Kavach code (stdlib/http/*.kav, spec 14.11). The Rust
//   runtime only resolves, connects, encrypts, reads and writes
//   (net.connect_tls, conn.read, conn.write, conn.close).
// - http.open splits the URL, adding :443 and the path / (stdlib/http/
//   client.kav:25-41), and connects at once with ALPN ["http/1.1"]
//   (stdlib/http/http.kav:362-371). A session is HTTP/1.1.
// - The name is looked up with Rust's to_socket_addrs on a new OS thread
//   made for this connect; only the first address is tried. The TCP
//   connect blocks that thread, 30 s at most (runtime/net/src/net/
//   connect.rs:49-54, 77; net.rs:34-36). The task waits in the scheduler,
//   and other tasks run (runtime/native/src/rt_waits.rs:45-60).
// - TLS is rustls with ring, TLS 1.3 and 1.2, the server's certificate
//   checked against Mozilla's roots built into the program (webpki-roots)
//   plus any roots the client's settings add, never the OS's store (runtime/net/src/net/tls.rs:70-107). The handshake
//   runs on the same helper thread; then the socket turns non-blocking and
//   joins the worker's readiness queue (tls.rs:161-179; connect.rs:77-100).
// - session.get builds the head as text, "GET / HTTP/1.1", host, and
//   content-length: 0; no user-agent or accept-encoding is added
//   (http.kav:324-332; client.kav:65-68), and writes it in one conn.write
//   (http.kav:333), which rustls encrypts (tls.rs:312-335).
// - With no task ready, the scheduler's thread waits on the OS's readiness
//   queue (epoll, kqueue or IOCP through mio); there is no network thread
//   (runtime/net/src/net/poller.rs:5-14, 159-166).
// - Each read is at most 64 KB (net.rs:38-57), decrypted by rustls. The
//   Kavach code gathers the pieces until the head's end and content-length
//   bytes are there, and gives Response(status, headers, body: Bytes)
//   (http.kav:336-356; stdlib/http/message.kav:44-119).
// - session.close sends TLS close_notify, then the socket is closed
//   (http.kav:319-321; net.rs:406-416; tls.rs:353-364).
// PLATFORM, not Kavach code: the resolver, TCP's handshake, the kernel's
// socket buffers and the network card.

const core = "cpu.core0";
const mem = "ram.chip2";
const kernel = "kernel.network";
const nic = "net.nic";
const cloud = "net.cloud";
const server = "net.server";

const lanes = ["main", "connect thread"];
type Who = Partial<Record<"main" | "connect thread", LaneState>>;

const request: HeapObject = {
  title: "the request, as text",
  cells: [{ text: "GET / HTTP/1.1" }, { text: "host: example.com" }, { text: "content-length: 0" }, { text: "(empty line)" }],
};
const response = (whole: boolean): HeapObject => ({
  title: "the response, read in pieces",
  cells: whole
    ? [{ text: "HTTP/1.1 200 OK" }, { text: "content-length: …" }, { text: "(empty line)" }, { text: "body: Bytes", note: "all of it" }]
    : [{ text: "HTTP/1.1 200 OK" }, { text: "…", dim: true, note: "more to come" }],
});

function step(line: number, at: string, who: Who, caption: string, extra: Partial<SceneFrame> = {}): SceneFrame {
  return { line, at, caption, lanes: who, ...extra };
}

export const httpsScene: Scene = {
  id: "https-get",
  title: "One HTTPS request: from your code to a server and back",
  caveman: "Ask OS: where server? Shake hands. Make secret talk. Send few lines of text. Wait, not hold core. Answer come in pieces. Put together. Done? Say bye.",
  sample: "explain/https-get.shown",
  code: "file",
  board: [
    { kind: "cpu", col: 0, row: 0, cores: 4, label: "CPU: the task, then the helper" },
    { kind: "ram", col: 1, row: 0, label: "RAM: the request and the reply" },
    { kind: "net", col: 1, row: 1, label: "Network card, the internet, the server" },
    { kind: "kernel", col: 0, row: 2 },
  ],
  lanes,
  lanesAre: "threads and tasks",
  panels: ["heap", "timeline"],
  heapEmpty: "Nothing yet: the URL is split into a host, a port and a path.",
  spec: "14.11",
  frames: [
    step(3, "http.open(", { main: "run" },
      "std.http, which is Kavach code, splits the URL: host example.com, port 443 added for https, and the path /. A session connects at once, not at its first request.",
      { lit: [core], labels: { [core]: "split URL" } }),
    step(3, "http.open(", { main: "wait", "connect thread": "run" },
      "The runtime, while running, starts one OS thread just for this connect, and parks the task: other tasks keep running. The thread asks the OS to turn example.com into an address, through the system's resolver.",
      { lit: [core, kernel, cloud], labels: { [core]: "connect thread", [kernel]: "resolve", [cloud]: "DNS" }, packets: [{ from: kernel, to: cloud, label: "example.com?" }, { from: cloud, to: kernel, label: "an address" }] }),
    step(3, "http.open(", { main: "wait", "connect thread": "run" },
      "The thread connects to the first address the resolver gives, and today only that one, on port 443. The OS's TCP does the three-way handshake through the network card. The thread waits for it, up to 30 seconds.",
      { lit: [kernel, nic, server], labels: { [kernel]: "TCP", [nic]: "SYN", [server]: "SYN-ACK" }, packets: [{ from: nic, to: server, label: "SYN" }, { from: server, to: nic, label: "SYN-ACK, ACK" }] }),
    step(3, "http.open(", { main: "wait", "connect thread": "run" },
      "Then TLS, by rustls inside the program, not the OS: the thread offers HTTP/1.1, the server sends its certificate, and rustls checks it against Mozilla's list of roots built into the program, plus any roots the program's own settings add, never the OS's store. Both sides agree on keys, so from now on every byte is encrypted.",
      { lit: [core, nic, server], labels: { [core]: "TLS", [server]: "certificate" }, packets: [{ from: core, to: server, label: "hello, http/1.1" }, { from: server, to: core, label: "certificate, keys" }] }),
    step(3, "http.open(", { main: "run", "connect thread": "done" },
      "The socket is ready. The runtime makes it non-blocking and adds it to the OS's readiness queue, the thread ends, and the task is woken with its session: one TLS connection, kept for the requests that follow.",
      { lit: [core], labels: { [core]: "session ready" } }),
    step(4, 'session.get("/")', { main: "run" },
      "session.get builds the request as text in memory: the request line, the host, and content-length: 0. Nothing else is added: no user-agent, no compression asked for.",
      { lit: [core, mem], labels: { [core]: "build request", [mem]: "request" }, heap: [request], packets: [{ from: core, to: mem, label: "GET / HTTP/1.1" }] }),
    step(4, 'session.get("/")', { main: "run" },
      "It goes out in one write: rustls encrypts it, the OS's network stack sends it through the network card, and the internet carries it to the server.",
      { lit: [core, kernel, nic, server], labels: { [kernel]: "send", [server]: "reads it" }, heap: [request], packets: [{ from: mem, to: kernel, label: "encrypted" }, { from: nic, to: server, label: "request" }] }),
    step(4, 'session.get("/")', { main: "wait" },
      "Now the task waits for the reply and gives up its turn. When no task is ready, the scheduler's thread sleeps in the OS's readiness queue. There is no network thread: the OS wakes the thread when bytes arrive.",
      { lit: ["kernel.scheduler"], labels: { "kernel.scheduler": "waiting" }, heap: [request] }),
    step(4, 'session.get("/")', { main: "run" },
      "The server's answer reaches the network card, the OS keeps it in the socket's buffer and wakes the thread. Each read takes up to 64 KB, and rustls decrypts it. std.http adds each piece until the head is whole.",
      { lit: [nic, kernel, core, mem], labels: { [kernel]: "receive", [mem]: "reply" }, heap: [response(false)], packets: [{ from: server, to: nic, label: "reply" }, { from: kernel, to: mem, label: "≤ 64 KB" }] }),
    step(4, 'session.get("/")', { main: "run" },
      "The head says the status, 200, and how many bytes the body has (content-length). Reading goes on until they are all there; then the task gets a Response: status, headers, and the body as Bytes.",
      { lit: [core, mem], labels: { [core]: "parse", [mem]: "reply" }, heap: [response(true)] }),
    step(5, "print(page.status)", { main: "run" },
      "page.status is a plain Int, and print writes it: 200 for this page.",
      { lit: [core], labels: { [core]: "print" }, heap: [response(true)] }),
    step(6, "session.close()", { main: "done" },
      "close tells the server the encrypted talk is over (TLS close_notify), then the OS closes the connection. A session never closed is closed when its last holder lets go, without that goodbye.",
      { lit: [core, kernel, nic, server], labels: { [kernel]: "close", [server]: "bye" }, packets: [{ from: nic, to: server, label: "close_notify, FIN" }] }),
  ],
  note: "The sample reaches a real server, so it is shown, not run: the compiler checks it. http.get(url), the one-call form, also offers HTTP/2 and uses a new connection each time. Name lookup, TCP, the socket buffers and the network card are the OS's; the rest is Kavach's runtime and std.http, which is written in Kavach.",
};
