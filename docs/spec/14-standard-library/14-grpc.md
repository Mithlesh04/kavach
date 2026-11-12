# 14.14 std.grpc

gRPC clients and servers from Kavach types, written in Kavach on top of `std.http.h2` (`stdlib/grpc/`), behind `Net` ([decision 0027](../../decisions/0027-grpc-as-a-module.md)).

```kavach
pub type GrpcError = Network(error: NetError) | Protocol(reason: String) |
    BadMessage(reason: String) | Status(code: Int, message: String)
pub type GrpcCall { method: String, message: Bytes, messages: [Bytes] }
pub type GrpcReply = Message(data: Bytes) | Stream(messages: [Bytes]) | Failed(code: Int, message: String)
```

| Function | Gives |
|---|---|
| `grpc.call<Req, Res>(target, method, request) -> Res ! GrpcError` | Calls `method` (`/shop.Orders/Place`) on the server at `target` with `request`, and decodes its answer into a `Res` |
| `grpc.call_stream<Req, Res>(target, method, requests) -> Res ! GrpcError` | Client streaming: sends each of `requests` as its own message on one call, and decodes the one answer into a `Res` |
| `grpc.each<Req, Res>(target, method, request, on) ! GrpcError` | Server streaming: runs `on: fn(Res)` for each message of the answer as it arrives |
| `grpc.each_stream<Req, Res>(target, method, requests, on) ! GrpcError` | Two-way streaming: sends each of `requests`, then runs `on` for each message of the answer as it arrives |
| `grpc.serve(listener, handler) ! NetError` | Serves gRPC on `listener`: each call is answered by `handler: fn(GrpcCall) -> GrpcReply`, each connection in a task of its own |
| `grpc.serve_talks(listener, handler, talks) ! NetError` | Serves like `serve`, but holds a conversation on each method in `talks: [String]`: `handler` answers each of its messages as soon as it arrives |
| `grpc.talk_with<Req, Res>(target, method, first, on) ! GrpcError` | A conversation: sends `first`, then runs `on: fn(Res) -> [Req]?` for each answer as it arrives, sending what it gives; `none` ends the client's side |
| `grpc.read<T>(call) -> T ! GrpcError` | A call's message decoded into a `T` (the first, when the client sent several) |
| `grpc.read_all<T>(call) -> [T] ! GrpcError` | Every message the client sent on the call, in order, each decoded into a `T` |
| `grpc.reply<T>(value) -> GrpcReply` | An answer with `value` |
| `grpc.reply_stream<T>(values) -> GrpcReply` | An answer that streams `values`, one message each |
| `grpc.failed(code, message) -> GrpcReply` | An answer with a gRPC status other than OK (`3` is `INVALID_ARGUMENT`, `5` `NOT_FOUND`, …) |
| `grpc.encode<T>(value) -> Bytes` | `value` as a Protocol Buffers message |
| `grpc.decode<T>(data) -> T ! GrpcError` | A Protocol Buffers message decoded into a `T` |

```kavach
use std.grpc
use std.net

pub type Order {
    id: Int
    item: String
    qty: Int = 1
}

pub type Placed {
    id: Int
    total: Float
}

fn orders(call: GrpcCall) -> GrpcReply {
    const order = grpc.read<Order>(call) catch e => Order(id: 0, item: "")
    if order.id == 0 {
        return grpc.failed(3, "not an order")
    }
    grpc.reply(Placed(id: order.id, total: 2.5 * Float(order.qty)))
}

fn main() ! Error {
    const listener = try net.listen("127.0.0.1:50051")
    const server = spawn grpc.serve(listener, orders)
    const order = Order(id: 7, item: "pen", qty: 2)
    const placed = try grpc.call<Order, Placed>("http://127.0.0.1:50051", "/shop.Orders/Place", order)
    print(placed.total)       // 5.0
}
```

- **Messages** are Protocol Buffers, worked out from the type (`json.schema<T>()`, 2.12): field `n` of a record is its `n`th declared field. `Int` and the sized integers are `int64` (or `uint64` for `UInt64`), `Float` is `double`, `Bool` is `bool`, `String` is `string`, a record is a nested message, and a list is `repeated` (packed for numbers and booleans). An optional field is left out when it is `none`, and a missing field decodes as `none`, its default, or the type's zero value. A union field is a **wrapper message holding a `oneof`**: counting the fieldless variants first (in declared order), then the variants with fields, variant `k` is field `k` of the wrapper — an empty message for a fieldless variant, the variant's fields as a message otherwise. `.proto` for it: `message Status { oneof value { Empty pending = 1; Paid paid = 2; } }`. A union field that never arrived is a problem in an `Invalid` error (2.12): there is no default variant. A map field with `String` keys travels as Protocol Buffers' own map encoding: a repeated message per entry, the key as field 1 and the value as field 2; `.proto` for it is `map<string, V>`.
- **Server streaming**: a handler answers `grpc.reply_stream(values)` and the messages travel on one stream, each behind its own 5-byte prefix, the status in the trailers as always. The client calls `grpc.each<Req, Res>(target, method, request, on)`: `on` runs for each message **as it arrives**, and `each` returns when the server finishes with status OK, or fails with `Status` as `call` does. A plain `call` to a streaming method gives the first message.
- **`kavach proto file.kav`** prints the `proto3` file for the program's declared record and union types, with these conventions, so `protoc` in any language generates matching code. Generic types are skipped with a comment.
- **For other languages**, write the `.proto` file with the same fields in the same order: `message Order { int64 id = 1; string item = 2; int64 qty = 3; }`. Adding fields at the end of a record keeps older clients working; reordering fields breaks them.
- **Calls** are HTTP/2 `POST` requests to `method` with `content-type: application/grpc`, each message behind gRPC's 5-byte prefix, and the status in the `grpc-status` trailer, so any gRPC library can call a Kavach server and be called by a Kavach client. A status other than 0 fails the call with `Status(code, message)`.
- **The target** says how to connect: `http://` speaks HTTP/2 without encryption, which must be written out, as on a machine's own network; `https://` uses TLS (14.10).
- **Client streaming**: `grpc.call_stream` sends every request on one stream, each behind its own prefix, then ends the stream. The handler gets them all in `call.messages` (`grpc.read_all<T>(call)`); `call.message` is the first, so a handler written for one message still works.
- **Two-way streaming**: `grpc.each_stream` sends every request, ends its side of the stream, and runs `on` for each answer as it arrives, like `each`. On the wire this is a gRPC bidirectional call, so any gRPC server can answer it. The client's messages all go before the answers are read, and a handler sees all of them at once; for a back-and-forth, use a conversation.
- **Conversations**: a method named in `serve_talks`'s `talks` is answered one message at a time. As each message arrives, the server calls `handler` with a `GrpcCall` holding just that message, and sends its reply at once: `reply` sends one message, `reply_stream` several, and `failed` ends the call with that status. When the client ends its side, the server ends the call with status OK. `talk_with` is the client's side: it sends `first`, then, as each answer arrives, runs `on`, which gives the messages to send next, or `none` to end the client's side; it returns when the server ends the call. So each side can answer the other's last message:

```kavach
fn plus_one(call: GrpcCall) -> GrpcReply {
    const c = grpc.read<Count>(call) catch e => Count(n: 0)
    grpc.reply(Count(n: c.n + 1))
}

const server = spawn grpc.serve_talks(listener, plus_one, ["/game.Count/Up"])
try grpc.talk_with<Count, Count>(target, "/game.Count/Up", [Count(n: 1)], c => {
    if c.n < 6 { [Count(n: c.n + 1)] } else { none }
})
```

  A conversation goes turn by turn: when neither side has anything to send, both wait. On the wire it is a gRPC bidirectional call, so a conversation works with any gRPC library on the other side.
- Not yet: compression, deadlines and metadata.
