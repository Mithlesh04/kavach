# 0027. gRPC as a Kavach module, driven by a type's JSON Schema

- **Status:** Accepted
- **Date:** 2026-11-01

## Context

Programs in other languages talk to services over gRPC: Protocol Buffers messages in HTTP/2 requests. Kavach should serve and call gRPC from its own types (to-do item 3). `std.http.h2` (decision 0022) already speaks HTTP/2 in Kavach.

The standard library modules must stay apart from the core: changing a module must not need a change in the compiler or the runtime (Mithlesh, 2026-11-01). gRPC has to be a module. What a module can't do today is learn a type's shape. Protocol Buffers numbers a message's fields and needs each field's kind, and Kavach code has no way to ask "what are the fields of `T`?". The compiler already knows: `ask<T>` sends a JSON Schema of `T` to the model (spec 8.4).

## Decision

**`std.grpc` is written in Kavach. The core gains two general primitives, nothing about gRPC.**

1. `json.schema<T>() -> Json` (spec 2.12): the JSON Schema of `T`, the one `ask<T>` uses, with a record's fields in the order they are declared. Any module can use it: gRPC, OpenAPI, forms.
2. `x.bits() -> UInt64` and `Float.from_bits(n)` (spec 2.1): a `Float`'s IEEE 754 bits, as binary formats need.
3. `std.grpc` encodes a value as Protocol Buffers from `json.of(value)` and `json.schema<T>()`: field `n` of the message is the `n`th field the type declares; `Int` is `int64`, `Float` is `double`, `Bool` is `bool`, `String` is `string`, a record is a nested message, a list is `repeated` (packed for numbers), and an optional field is left out when `none`. Decoding goes the other way, into `Json`, then `json.decode_value<T>`.
4. Calls and answers travel as gRPC does: `POST /<service>/<method>` over HTTP/2 with `content-type: application/grpc`, each message behind its 5-byte prefix, and the status in the `grpc-status` trailer.

## Consequences

- The core stays small: two primitives, both useful beyond gRPC. Everything gRPC is Kavach code, the same in the interpreter and native code.
- Field numbers follow declaration order, so adding a field at the end of a record keeps old clients working, and reordering fields breaks them, as renumbering a `.proto` does. The type is the contract.
- Other languages need a `.proto` file; one can be written from the Kavach types by hand now, and by a tool later (`kavach proto`).
- Not covered yet: unions (Protocol Buffers' `oneof`), maps, streaming calls, compression, deadlines. They come as the module grows, without core changes.

## Alternatives considered

- **Protocol Buffers in the compiler and runtime** (generated encoders per type): faster, but gRPC would live in the core, against the rule above.
- **`.proto` files compiled to Kavach types** (as other languages do): a code generator to build and run first; Kavach types are already a schema.
- **Field numbers written on each field** (`id: Int @1`): needs new syntax in the core language; declaration order is enough to start.
