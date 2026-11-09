# 0019. A built-in `Json` union for dynamic values

- **Status:** Accepted
- **Date:** 2026-10-31

## Context

Programs constantly handle data whose shape is only known when they run: JSON from web services, files and other programs. Python and JavaScript make this easy with dicts and objects, but a typo in a key or a missing field is found in production. Kavach is statically typed and has no `null` (spec 2.7), so it needs a way to hold such data that is just as easy to use and still can't crash or leak.

The data often comes from outside, so it can be hostile: deeply nested documents that overflow the stack, huge inputs, duplicate keys that two parsers read differently, and numbers that silently lose precision.

## Decision

- `Json` is a **built-in union** (spec 2.12): `Null`, `Boolean`, `Integer`, `Number`, `Text`, `Array`, `Object`. `match` sees every case, and the compiler checks it like any union.
- **Reading never panics:** `j["key"]` and `j[i]` give `Null` when there is nothing there, so reads chain; `text()`, `int()`, `float()` and the other methods give optionals.
- **Building uses literals:** where a `Json` is expected, ordinary literals (and encodable values inside them) become `Json`. `json.of(value)` converts any encodable value.
- **`std.json`** has `parse`, `decode<T>` (straight into a typed value, with no `Json` in between), `decode_value<T>`, `of` and `stringify`.
- **Safe by default:** limits on depth, size and fields per object; duplicate keys, integers that don't fit and numbers too large for `Float` are errors; `secret` fields can't be encoded.
- It is an ordinary value: reference-counted and copy-on-write like every other (decision 0003).

## Consequences

- Code that handles JSON stays short, and decoding into records keeps most code fully typed.
- The compiler gains one rule outside the usual "no implicit conversions" (2.1): inside a literal where a `Json` is expected, encodable values convert. It is limited to that place.
- `Json` needs support in the interpreter and both native backends, and a fast parser in the runtime.
- Objects compare like maps, so key order counts in `==`.

## Alternatives considered

- **`[String: Any]` with a dynamic `Any` type:** opens every program to runtime type errors, which is what static types are for.
- **Only typed decoding (`decode<T>`), no dynamic value:** fine for known shapes, but exploring a document, proxying it or handling a few optional fields then needs a record for everything.
- **Missing keys as optionals (`j["a"]?["b"]`):** safer-looking, but needs a new `?[` operator and makes the most common code longer. `Null` for missing gives the same safety, since every value still comes out through an optional.
- **One `Number` variant for all numbers (as JavaScript):** integers above 2^53 would silently lose digits; IDs often are that large.
- **Unlimited parsing (as most libraries):** a single crafted request could crash or exhaust a server.
