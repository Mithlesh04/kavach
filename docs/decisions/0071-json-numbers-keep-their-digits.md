# 0071. JSON numbers keep their digits

- **Status:** Accepted
- **Date:** 2026-11-05

## Context

A `Json` value held a number as `Integer(value: Int)` or `Number(value: Float)` (spec 2.12). So an integer past `Int` was an error (`OutOfRange`), a `Float` lost digits (`0.1`, `12345678901234567890.5`), and `BigInt` and `Decimal` couldn't be read or written exactly: `json.of` wrote a `Decimal` as its nearest `Float`.

JSON itself (RFC 8259) lets a number have any number of digits; it is the reader that chooses what to keep. What others do:

| Language | By default | Exact when asked |
|---|---|---|
| Python `json` | integers exact at any size; fractions as `float` | `parse_float=Decimal` |
| JavaScript | every number a `Number` (exact to 2^53); `BigInt` can't be written | `JSON.rawJSON`, source text in `JSON.parse`'s reviver |
| Go `encoding/json` | `float64` | `Decoder.UseNumber()`: `json.Number`, the text |
| Rust `serde_json` | `i64`, `u64` or `f64` | the `arbitrary_precision` feature: the text |
| Java Jackson | `int`, `long`, `double` | `BigInteger`, `BigDecimal` |

Beyond RFC 8259, some readers take more: Python reads and writes `NaN` and `Infinity` by default; JSON5 adds comments, trailing commas, hex numbers, `Infinity`, `NaN`, single quotes and unquoted keys; JSONC (VS Code's settings files) adds comments and trailing commas. `23j` is Python's complex number, which no JSON has.

## Decision

Mithlesh chose (2026-11-05):

1. **A `Json` number keeps the digits it was written with**, as Go's `json.Number` and Rust's `arbitrary_precision` do. `Json` has one number case, `Number(digits: String)`, in place of `Integer` and `Number(value: Float)`. `int()`, `bigint()`, `float()` and `decimal()` read it, each `none` when the number doesn't fit. Typed decoding reads a `BigInt` or `Decimal` field exactly, and `json.of` writes them with all their digits.
2. **`json.parse` stays strict RFC 8259**, as in JavaScript, Go and Rust. `json.parse(text, comments: true)` also takes `//` and `/* */` comments and trailing commas (JSONC), for configuration files. `NaN`, `Infinity`, hex numbers and other JSON5 forms stay syntax errors; a `Float` that is `NaN` or infinite is still written as `null`.

## Consequences

- No number is changed or refused by reading it: an ID like `12345678901234567890` or a price like `0.1` survives a parse and a write exactly.
- Code that matched `Json.Integer` or `Json.Number(value)` changes to `Json.Number(digits)` or the methods. Kavach has no users yet; the repository's code is rewritten in the same change.
- To stay fast, parsing keeps a number whose text is exactly an `Int` or a `Float` as one while it reads, and typed decoding parses each field's number straight from the text. Measured on [benchmarks/json](../../benchmarks/json/README.md) (2026-11-05, against the commit before): `stringify` and `decode` unchanged, `parse` about 15% slower (each number holds its digits as text), reading numbers from a `Json` about 60% slower (`int()` and `float()` read the digits each time).

## Alternatives considered

- **Exact only in typed fields:** `Json` unchanged; only `json.decode<T>` and `json.of` exact for `BigInt` and `Decimal`. Less to change, but `json.parse` would still refuse big integers and lose digits.
- **`Integer(value: BigInt)` and `Number(value: Decimal)`:** exact too, but every number a big value, so parsing and reading slower.
- **JSON5 on request, or strict only:** JSON5's extra forms (hex, `Infinity`, single quotes) are rarely in data a program receives; comments and trailing commas are what configuration files need.
