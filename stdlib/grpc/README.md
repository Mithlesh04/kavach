# stdlib/grpc

`std.grpc` (spec 14.14, decision 0027): gRPC clients and servers from Kavach types, written in Kavach on `std.http.h2`.

| File | Module | Contents |
|---|---|---|
| `grpc.kav` | `std.grpc` | Calls (`grpc.call<Req, Res>`; streaming: `each`, `call_stream`, `each_stream`; conversations: `talk_with`) and servers (`grpc.serve`, `serve_talks`) |
| `frames.kav` | `std.grpc.frames` | The message prefix, the status trailers and `GrpcError`, shared by the client and the server |
| `protobuf.kav` | `std.grpc.protobuf` | Protocol Buffers messages from `Json` values and a type's JSON Schema (`json.schema<T>()`): field `n` is the type's `n`th declared field |
| `tests/found/` | | Inputs `kavach test` found that once failed (generated: commit, never edit) |

## Checked against another gRPC library

2026-11-02, with Python's `grpcio` 1.85.1 and a `.proto` written by `kavach proto`: a Kavach client against a `grpcio` server, and a `grpcio` client against a Kavach server, each making all four kinds of call (one message each way, server streaming, client streaming, two-way), and a conversation in which each side's next message depends on the other's last answer (`talk_with` against a `grpcio` server that answers per message, and a `grpcio` client against `serve_talks`). Every call gave the expected messages in both directions. The check is not yet a repeatable test: it needs `grpcio`, an outside dependency for the test suite.
