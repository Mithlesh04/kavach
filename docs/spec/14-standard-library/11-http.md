# 14.11 std.http

HTTP/1.1 and HTTP/2 clients and servers, and HTTP/3 clients, written in Kavach on top of `std.net` (`stdlib/http/`), behind `Net` ([decision 0022](../../decisions/0022-http2-and-http3.md)).

```kavach
pub type Request { method: String, path: String, headers: [Header], body: Bytes }
pub type Response { status: Int, headers: [Header], body: Bytes }
pub type HttpError = Network(error: NetError) | BadUrl(url: String) | BadMessage(reason: String) |
    Status(code: Int) | BadJson(error: JsonError)
```

Headers are a list of `Header(name:, value:)` in the order they are sent, names in lower case, and a name may come more than once: two `set-cookie` lines stay two headers, never one value. `r.header(name)` gives the first value of a name (`String?`, the name in any case), and `r.headers_named(name)` all of them, in order ([decision 0087](../../decisions/0087-http-headers-and-server-limits.md)).

```kavach
fn login(r: Request) -> Response {
    const client = r.header("User-Agent") ?? "unknown"
    Response(status: 200, headers: [
        Header(name: "set-cookie", value: "session=abc; HttpOnly"),
        Header(name: "set-cookie", value: "theme=dark"),
    ], body: "hello, {client}".bytes())
}
```

| Function | Gives |
|---|---|
| `http.get(url) -> Response ! HttpError` | `GET url` |
| `http.post(url, body) -> Response ! HttpError` | `POST url` with `body` (`Bytes`) |
| `http.get3(url) -> Response ! HttpError` | `GET url` over HTTP/3 |
| `http.open(url) -> Session ! HttpError` | A session: one HTTP/1.1 connection to the server of `url` (`http://` or `https://`; the path is not used), kept open for several requests |
| `s.get(path)`, `s.post(path, body) -> Response ! HttpError` | A request on session `s`'s connection, waiting for its response |
| `s.close()` | Closes the session's connection |
| `http.serve(listener, handler, limits: ServeLimits()) ! NetError` | Serves HTTP on `listener`: each connection in a task of its own, on any core (`spawn cores`, 9.8), its requests answered by `handler: fn(Request) -> Response`, each client held to `limits` (below). It runs until the function that called it returns, which cancels it (9.2) |
| `http.text(status, body) -> Response` | A response with text |
| `http.get_json<T>(url) -> T ! HttpError` | `GET url`, the response's JSON body decoded into a `T` (`json.decode<T>`, 2.12) |
| `http.post_json<T>(url, body) -> T ! HttpError` | `POST url` with the `Json` value `body`, the response's JSON decoded into a `T` |
| `http.json_response(status, value) -> Response` | A response with the `Json` value `value` (write `json.of(order)` for a typed value) |
| `http.read_json<T>(request) -> T ! JsonError` | A request's JSON body decoded into a `T` |

```kavach
use std.http
use std.net

fn hello(r: Request) -> Response {
    http.text(200, "hello from {r.path}")
}

fn main() ! Error {
    const listener = try net.listen("127.0.0.1:8080")
    const server = spawn http.serve(listener, hello)
    const page = try http.get("http://127.0.0.1:8080/home")
    print(page.body.text() ?? "")      // hello from /home
}
```

`post_json` sends its body with `content-type: application/json`, as `json_response` does. A status other than 2xx fails with `Status(code)`; a body that isn't the JSON of a `T` fails with `BadJson(error)`. With them, a service and its client share their types:

```kavach
use std.http
use std.json
use std.net

pub type Order {
    id: Int
    item: String
}

fn orders(r: Request) -> Response {
    const wanted = http.read_json<Order>(r) catch e => return http.text(400, "not an order")
    http.json_response(200, json.of(Order(id: wanted.id, item: "{wanted.item}, packed")))
}

fn main() ! Error {
    const listener = try net.listen("127.0.0.1:8080")
    const server = spawn http.serve(listener, orders)
    const order = Order(id: 7, item: "pen")
    const packed = try http.post_json<Order>("http://127.0.0.1:8080/orders", json.of(order))
    print(packed.item)      // pen, packed
}
```

A client's HTTP/1.1 connection carries one request and its response (`connection: close`). `http.serve` keeps a connection open for more, as HTTP/1.1 does by default: it answers the requests in the order they come, and closes the connection when the client asks (`connection: close`, or HTTP/1.0 without `connection: keep-alive`), or when the client sends nothing for 5 seconds.

`http.serve` spreads its connections over every core of the machine: each connection's task is spawned with `spawn cores` (9.8), so in a release build (`kavach build`) handlers run at once on all cores, and other runs answer them on one core, in turn, with the same responses. A handler is a function value, so it shares nothing with the others but what it reaches through a `Shared` value (6.2) or a channel. What handlers print keeps the order `spawn cores` tasks keep ([decision 0049](../../decisions/0049-tasks-on-all-cores-keep-the-programs-order.md)): each handler's own lines in the order it prints them, whole. One task accepts the connections and hands each to a core; measured on Windows (24 cores, 32 clients making 200 requests each on kept-open connections), a handler that computes answered in 2.5–3.0 s instead of 18–20 s on one core, and one that does nearly nothing in 131–221 ms instead of 461–633 ms.

**Limits.** A server faces clients that are slow, large or hostile, so `http.serve` holds each one to `ServeLimits`, whose every field has a safe default: `http.serve(listener, handler)` is already protected. A program changes the ones it needs.

```kavach
pub type ServeLimits {
    pub max_header_bytes: Int = 16_384        // 431 past it
    pub max_body_bytes: Int = 10_000_000      // 413 past it, refused by `content-length` before the body is read
    pub buffered_body: Int = 1_000_000        // larger bodies are read by the handler, `r.read_body()`
    pub header_timeout: Duration = 10.seconds // 408 when a request's line and headers take longer
    pub min_body_rate: Int = 1_000            // bytes a second, after a second's grace: 408 when slower
    pub idle_timeout: Duration = 5.seconds    // a kept-open connection waiting for its next request
    pub max_connections: Int = 10_000         // 503 past it
    pub max_in_flight: Int = 1_000            // requests being answered at once: 503 past it
    pub retry_after: Int = 1                  // what `retry-after` says on a 503, in seconds
    pub drain_timeout: Duration = 10.seconds  // on a stop signal, how long to finish the requests under way
}

const server = spawn http.serve(listener, upload, limits: ServeLimits(max_body_bytes: 100_000_000))
```

A request that breaks a limit is answered with its status and the connection closed; the handler never sees it. A client sending its headers a byte at a time (slowloris) holds a task for at most `header_timeout`, and one sending a body too slowly for at most a second past what `min_body_rate` allows. A request with `expect: 100-continue` gets `100 Continue` once its `content-length` is known to fit, before it sends the body. A 503 says `retry-after`, so well-behaved clients wait before trying again. The counts of connections and requests are kept across every core.

**A client that leaves.** While a request is answered, `http.serve` keeps reading its connection, as Go's server does. When the client closes it, the handler's task is cancelled (9.6), so a request nobody waits for stops using the server; no answer is sent. A handler that panics gets 500, and the server goes on. Anything the client sends meanwhile (the next request, sent ahead) is kept for after the answer.

**Large bodies.** A request's body up to `buffered_body` (a `ServeLimits` field, 1 MB) is read whole into `r.body`. A larger one, up to `max_body_bytes`, is left for the handler to read as it comes, a piece at a time, with `r.read_body() -> Bytes? ! NetError` (`none` once it has all come): an upload of any size then needs no more memory than a piece ([decision 0089](../../decisions/0089-streamed-http-bodies.md)). Such a request's `r.body` is empty. It isn't watched for the client leaving, as its handler reads the connection itself; a read fails when the client goes. A body the handler doesn't read to its end closes the connection after the answer.

A response can be sent as it is read: `Response(status:, headers:, body:, stream:)` takes a reader of `fs.chunks` (14.3) as `stream`, and `body` is then sent before it. The response says its length when its headers give `content-length`, and comes in chunks otherwise. `http.file(path) -> Response ! FsError` answers with a file that way, its `content-type` from the file's extension and its `content-length` from its size, so a file of any size is sent without being held in memory.

```kavach
fn upload(r: Request) -> Response uses Net {
    let total = 0
    while true {
        const piece = r.read_body() catch e => return http.text(400, "upload cut off")
        total += (piece ?? break).len()
    }
    http.text(200, "got {total} bytes")
}

fn download(r: Request) -> Response uses Fs.read {
    http.file("./files/report.pdf") catch e => http.text(404, "no such file")
}
```

**Forms with files.** A `multipart/form-data` body (an HTML form with files) is read a part at a time, each part's data as it comes, so a large upload is never all in memory ([decision 0090](../../decisions/0090-forms-read-part-by-part.md)). `http.form(r) -> Form ! HttpError` opens it, from `r.body` or from `r.read_body()` alike.

| Written | Gives |
|---|---|
| `form.next() -> FormPart? ! HttpError` | The next part, or `none` after the last; data of the part before not read yet is skipped |
| `part.name`, `part.filename`, `part.content_type` | The field's name, the file's name (`String?`, `none` for a plain field) and its `content-type` (`text/plain` when not given) |
| `part.read() -> Bytes? ! HttpError` | The part's next piece of data, or `none` at its end |
| `part.text() -> String ! HttpError` | The rest of the part as text: for small fields |

A form that isn't `multipart/form-data`, has no boundary, or is cut off fails with `BadMessage`.

```kavach
fn upload(r: Request) -> Response ! HttpError uses Net, Console {
    const form = try http.form(r)
    let received = 0
    while true {
        const part = (try form.next()) ?? break
        if part.filename == none {
            print("{part.name} = {try part.text()}")
            continue
        }
        while true {
            received += ((try part.read()) ?? break).len()
        }
    }
    http.text(200, "received {received} bytes")
}
```

**Stopping.** `http.serve` holds the program's stop (`cli.on_stop`, 5.8) for `drain_timeout`. On Ctrl+C or `SIGTERM` (how Docker and Kubernetes stop a container), it answers new connections with 503, closes those waiting for their next request, finishes the requests under way, and then lets go, so the program ends without cutting a response off. A second signal ends it at once.

A session sends its requests one after another on its connection, each waiting for its response, which must say its length (`content-length` or chunks). When the server has closed the connection, the request fails with `Network(error: Failed(address: host, reason: "the server closed the connection"))`; open a new session to go on. A session is for one task at a time: two tasks sending on one session at once could each read the other's response. Give each task its own session.

```kavach
const api = try http.open("http://127.0.0.1:8080")
const user = try api.get("/users/1")
const made = try api.post("/users", body)
api.close()
```

A body's size comes from `content-length`, or the body comes in chunks (`transfer-encoding: chunked`, RFC 9112 7.1): requests and responses sent that way are read whole, chunk sizes and trailers left out. A chunked response cut off before its last chunk fails with `BadMessage`. `https://` URLs go through `net.connect_tls`, offering `h2` and `http/1.1` (14.10): when the server chooses `h2`, the request goes over HTTP/2. Anything other than `http://` and `https://` gives `BadUrl`.

`http.serve` answers HTTP/2 too, from a client that starts with the HTTP/2 preface without TLS ("prior knowledge", as services calling each other do). It answers that connection's requests one at a time, each with `handler`. The program sees the same `Request` and `Response` either way.

`std.http.h2` has the parts, for programs that speak HTTP/2 themselves (gRPC, for one). A message is `H2Message(stream: Int, headers: [String: String], body: Bytes)`, with the pseudo-headers (`:method`, `:path`, `:status`, …) among its headers. Flow control is handled: a large body waits for the other side's window. Server push is never used, and stream priorities are ignored.

| Function | Gives |
|---|---|
| `h2.request(conn, headers, body) -> H2Message ! H2Error` | Sends one request on a new connection (`headers` start with `:method`, `:scheme`, `:authority`, `:path`) and reads the response |
| `h2.accept(conn, data) -> Link ! H2Error` | The server side of a connection that began with `data` |
| `h2.next(link) -> Next ! H2Error` | `Next(link:, message:)`: the next whole request, or `none` once the client is done |
| `h2.respond(link, stream, headers, body) -> Link ! H2Error` | Sends the response on `stream`; `headers` start with `:status` |
| `h2.begin(conn, headers) -> Link ! H2Error` | Opens a request on a new connection and leaves its stream open: `send_data` sends the body piece by piece while `pull` reads the answer as it arrives |
| `h2.send_data(link, stream, data, end) -> Link ! H2Error` | Sends `data` on `stream`; `end: Bool` ends this side of the stream |
| `h2.send_headers(link, stream, headers) -> Link ! H2Error` | Sends a response's headers and leaves the stream open |
| `h2.send_trailers(link, stream, trailers) -> Link ! H2Error` | Sends header fields after the body, ending the stream |
| `h2.watch(link) -> H2Watched ! H2Error` | The server side piece by piece: `H2Watched(link:, event:)`, where the event is `StreamOpened(stream, headers)` as soon as a request's headers arrive, `StreamData(stream, data)` as its body arrives, `StreamEnded(stream)`, or `ClientDone`. A server uses `watch` or `next`, not both |

`H2Error` is `Network(error: NetError) | Protocol(reason: String) | Reset(stream: Int, code: Int)`.

```kavach
const conn = try net.connect("127.0.0.1:8080")
const reply = try h2.request(conn, [
    Header(name: ":method", value: "GET"),
    Header(name: ":scheme", value: "http"),
    Header(name: ":authority", value: "localhost"),
    Header(name: ":path", value: "/"),
], Bytes([]))
print(reply.headers[":status"])       // 200
```

HTTP/3 (RFC 9114) runs over QUIC (14.10). `http.get3(url)` sends `GET url` over HTTP/3 (`https://` only) and gives the same `Response` and `HttpError` as `http.get`. `http.get` itself stays on TCP; choosing HTTP/3 on its own, when a server announces it (`alt-svc: h3=":443"`), comes later.

`std.http.h3` has the parts, for programs that speak HTTP/3 themselves. A message is `H3Message(headers: [String: String], body: Bytes)`, with the pseudo-headers among its headers. Headers are compressed with QPACK (RFC 9204) using its static table and literals only: Kavach announces a dynamic table of size 0, so a server never refers to one.

| Function | Gives |
|---|---|
| `h3.request(conn, headers, body) -> H3Message ! H3Error` | Sends one request on a new connection from `net.connect_quic(address, ["h3"])` (`headers` start with `:method`, `:scheme`, `:authority`, `:path`), and reads the response |
| `h3.encode(headers) -> Bytes` | A QPACK header block of `Header(name:, value:)`s |
| `h3.decode(block) -> [Header] ! H3Error` | The headers of a QPACK block that uses no dynamic table |

`H3Error` is `Network(error: NetError) | Protocol(reason: String)`.

```kavach
const conn = try net.connect_quic("cloudflare-quic.com:443", ["h3"])
const reply = try h3.request(conn, [
    Header(name: ":method", value: "GET"),
    Header(name: ":scheme", value: "https"),
    Header(name: ":authority", value: "cloudflare-quic.com"),
    Header(name: ":path", value: "/"),
], Bytes([]))
print(reply.headers[":status"])       // 200
```

`std.http.hpack` compresses HTTP/2 headers (RFC 7541), for `std.http` and for programs that speak HTTP/2 themselves ([decision 0022](../../decisions/0022-http2-and-http3.md)). Each side of a connection keeps a table of recent headers; decoding a block gives its headers and the table for the next block.

| Function | Gives |
|---|---|
| `hpack.new_table(limit) -> Table` | An empty table of at most `limit` bytes (HTTP/2 starts with 4096) |
| `hpack.decode(block, table) -> Decoded ! HpackError` | `Decoded(headers: [Header], table: Table)`; `Malformed(reason)` for a broken block |
| `hpack.encode(headers) -> Bytes` | A block with each `Header(name:, value:)` as a literal, names in lower case |
| `hpack.huffman(data) -> Bytes ! HpackError` | Decodes Huffman-coded text (RFC 7541 appendix B); QPACK uses the same code |

```kavach
const block = hpack.encode([Header(name: "content-type", value: "text/plain")])
const d = try hpack.decode(block, hpack.new_table(4096))
print(d.headers[0].value)              // text/plain
```
