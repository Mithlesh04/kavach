# 14.10 std.net

TCP connections and UDP datagrams, behind the `Net` capability (8.1). Addresses are text: `"127.0.0.1:8080"`, `"example.com:443"`; port `0` lets the system pick a free one.

| Function or method | Gives |
|---|---|
| `net.listen(address) -> Listener ! NetError` | Listens for connections at `address` |
| `listener.address() -> String` | The address it listens at, with the port the system picked |
| `listener.accept() -> Connection ! NetError` | Waits for the next connection |
| `net.connect(address) -> Connection ! NetError` | A connection to `address` |
| `net.connect_tls(address, protocols: [String] = [], roots: String = "", cert: String = "", key: String = "") -> Connection ! NetError` | A connection encrypted with TLS (1.3, or 1.2 for servers without 1.3). The server's certificate must be valid for the host name in `address`, checked against root certificates built into Kavach ([decision 0021](../../decisions/0021-tls-through-rustls.md)) and the PEM certificates in `roots`, if any (a company's own certificate authority); there is no way to skip the check. `protocols` are offered to the server (ALPN), such as `["h2", "http/1.1"]`. With `cert` and `key` (PEM), the client proves who it is to a server that asks (mutual TLS) |
| `net.listen_tls(address, cert, key, protocols: [String] = [], clients: String = "") -> Listener ! NetError` | Listens for TLS connections, proving the server's identity with the certificate chain `cert` and private key `key` (PEM text, read with `fs.read_text` where they are files). `key` takes a secret (8.7), here and in `connect_tls`, so a key read with `env` or kept in a `secret` field needs no `reveal`. It chooses the first of `protocols` a client offers. With `clients` (PEM certificates), every client must present a certificate signed by one of them (mutual TLS); a client without one is refused while connecting |
| `conn.read() -> Bytes? ! NetError` | Waits for data: the bytes that came (at most 64 KB at a time), or `none` once the other side has closed |
| `conn.write(data) ! NetError` | Sends all of `data` (`Bytes`) |
| `conn.protocol() -> String?` | The protocol the two sides agreed on, from those `connect_tls` offered and `listen_tls` takes, or `none` |
| `conn.peer() -> String` | The other side's address: for a connection a listener accepted, whoever opened it (`203.0.113.7:51234`) |
| `conn.start_tls(host, protocols: [String] = [], roots: String = "", cert: String = "", key: String = "") -> Connection ! NetError` | TLS begun as a client on a connection that has spoken plain TCP so far, the way PostgreSQL's `SSLRequest` and SMTP's `STARTTLS` do it. The server's certificate is checked against `host`, and the other settings are those of `connect_tls`. It gives the encrypted connection; `conn` itself is closed from then on, so nothing is sent in the clear by mistake |
| `conn.accept_tls(cert, key, protocols: [String] = [], clients: String = "") -> Connection ! NetError` | The server's side of the same: TLS begun on a connection the server accepted, with the settings of `listen_tls` |
| `conn.peer_certificate() -> Bytes?` | The other side's TLS certificate as DER: the server's on a client connection, a client's (mutual TLS) on a server one; `none` without TLS. For channel binding, such as SCRAM's `tls-server-end-point`, which hashes it |
| `conn.client_cert() -> ClientCert?` | With mutual TLS (`clients:` on `listen_tls`), the names on the certificate the client showed: `ClientCert(common_name:, dns:, uris:)`, from its subject and its subject-alternative names (a SPIFFE id such as `spiffe://shop/orders` is a URI); `none` otherwise ([decision 0099](../../decisions/0099-client-certificate-names.md)) |
| `conn.close()` | Closes the connection (or stops listening) |
| `net.bind(address) -> UdpSocket ! NetError` | A UDP socket at `address` |
| `sock.send(data, to) ! NetError` | Sends `data` (`Bytes`) as one datagram to the address `to` |
| `sock.receive() -> Datagram ! NetError` | Waits for the next datagram: `Datagram(data: Bytes, from: String)`, with the sender's address |
| `sock.address()`, `sock.close()` | As for a listener |

Waiting is waiting in the sense of 9.7: while a task waits for the network, other tasks run, and a cancelled task stops there (9.6). Every wait except `accept` and `receive` has a deadline, 30 seconds, so a server that stops answering can't hang the program: then the call fails with `TimedOut`. Copies of a `Listener`, `Connection` or `UdpSocket` share it, like `Shared` (6.2). Using one needs `Net`, like `net.connect`.

A TLS listener's `accept` also runs the handshake, within the same deadline: a client that doesn't finish it is dropped, and `accept` waits for the next one. Handshakes go on side by side, so a slow or silent client holds up neither the other clients nor the program's other tasks.

`.timeout(d)` (9.6) sets a wait's deadline to `d` instead: on `accept`, `read`, `write`, `send`, `receive`, `read_stream`, `write_stream`, `start_tls`, `accept_tls`, `net.connect` and `net.connect_tls`. When `d` passes, the call fails with `TimedOut`; a `write` may have sent part of its data by then.

```kavach
const data = try conn.read().timeout(5.seconds)
const reply = try sock.receive().timeout(200.ms)
```

```kavach
use std.net

fn echo_once(listener: Listener) ! NetError uses Net {
    const conn = try listener.accept()
    while true {
        const data = (try conn.read()) ?? break   // none: the client closed
        try conn.write(data)
    }
}
```

UDP carries each datagram on its own: it may get lost, come twice, or come out of order, and a large one may be cut down (at most 64 KB is read).

QUIC (RFC 9000) carries many streams in one encrypted connection over UDP, each delivered in order, with no stream waiting for another's lost packets. It is what HTTP/3 runs on (14.11). TLS 1.3 is part of it, checked like `connect_tls` ([decision 0022](../../decisions/0022-http2-and-http3.md)).

| Function or method | Gives |
|---|---|
| `net.connect_quic(address, protocols: [String]) -> Connection ! NetError` | A QUIC connection to `address`, offering `protocols` (ALPN, such as `["h3"]`); `conn.protocol()` tells which one the server chose |
| `conn.open_stream(two_way: Bool) -> Int ! NetError` | Opens a stream, two-way or only for sending, and gives its number; waits while the server allows no more |
| `conn.write_stream(stream, data, done: Bool) ! NetError` | Sends all of `data` on the stream; `done` ends the stream's sending side |
| `conn.read_stream(stream) -> Bytes? ! NetError` | Waits for data on a two-way stream: the bytes that came, or `none` once the server has ended its side |

`read` and `write` fail on a QUIC connection, and the stream methods on a TCP one. Streams the server opens are not read.

```kavach
const conn = try net.connect_quic("cloudflare-quic.com:443", ["h3"])
const s = try conn.open_stream(true)
try conn.write_stream(s, request, true)
const reply = try conn.read_stream(s)
```

A failed network operation returns a `NetError`, from `std.net`:

```kavach
pub type NetError = Refused(address: String) | TimedOut(address: String) | Failed(address: String, reason: String)
```
