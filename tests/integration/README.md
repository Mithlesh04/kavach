# Integration tests

Larger Kavach programs that exercise many features together. Each file's `test` blocks run with `kavach test` (the Rust test `tools/cli/tests/integration.rs` runs them all).

| File | What it checks |
|---|---|
| [fuzz-decoders.kav](fuzz-decoders.kav) | Hostile and corrupted bytes never crash the wire decoders: Protocol Buffers (`grpc.decode<T>`), HPACK, QPACK (to-do item 4) |
| [fuzz-wire.kav](fuzz-wire.kav) | A server answering junk never crashes the HTTP/1 and HTTP/2 clients, and hostile text never crashes `json.decode<T>` (to-do item 4) |
