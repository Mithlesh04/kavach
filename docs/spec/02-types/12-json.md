# 2.12 Json

`Json` holds data whose shape is only known when the program runs: a JSON document from a file or a web service, or a value you build to send. It is a built-in union, so `match` sees every case:

```kavach
type Json = Null |
    Boolean(value: Bool) |
    Number(digits: String) |
    Text(value: String) |
    Array(items: [Json]) |
    Object(fields: {String: Json})
```

A `Number` keeps the digits it was written with, exactly (`"12345678901234567890"`, `"0.1"`, `"-2.5e10"`), so no number is changed by reading it and writing it out again ([decision 0071](../../decisions/0071-json-numbers-keep-their-digits.md)). The methods below read it as the number type a program needs. Objects keep their keys in order, like dicts (2.2).

### Reading

Reading never panics. `j["key"]` and `j[i]` give the field or item, or `Null` when there is none (not an object or list, a missing key, an index out of range), so reads chain. A method then gives the value as an optional:

```kavach
use std.json

const doc = try json.parse(body)
const name = doc["user"]["name"].text() ?? "guest"
const first_tag = doc["tags"][0].text()            // String?
for item in doc["items"].list() ?? [] {
    print(item["price"].float() ?? 0.0)
}
```

| Method | Gives |
|---|---|
| `text()` | `String?`: the text of a `Text` |
| `int()` | `Int?`: a `Number` written without a fraction or exponent that fits `Int` |
| `bigint()` | `BigInt?`: a `Number` written without a fraction or exponent, of any size, or a `Text` holding one (as `big_as_text` writes it) |
| `float()` | `Float?`: a `Number`, as the nearest `Float`; `none` past `Float`'s range |
| `decimal()` | `Decimal?`: a `Number`, exactly, or a `Text` holding one |
| `bool()` | `Bool?`: the value of a `Boolean` |
| `list()` | `[Json]?`: the items of an `Array` |
| `object()` | `{String: Json}?`: the fields of an `Object` |
| `is_null()` | `Bool`: whether it is `Null` |
| `len()` | `Int`: the items of an `Array` or fields of an `Object`, otherwise 0 |

### Building

Where a `Json` is expected, literals become `Json` values: numbers, text, `true`, `false`, `none` (which becomes `Null`), lists and dicts with text keys, at any depth. Inside such a literal, any other expression is converted too, as `json.of` converts it (below):

```kavach
const order: Json = {
    "id": 1042,
    "customer": user.name,
    "items": ["pen", "ink"],
    "paid": true,
    "note": none,
}
```

`json.of(value)` converts any value (below). A `Json` is a value like any other (chapter 6): copies never share changes, and it is freed as soon as nothing holds it. It prints as compact JSON text. Two `Json` values are equal when they are the same variant with equal contents; objects compare like dicts, so the order of keys doesn't count (1.8).

### std.json

| Function | Gives |
|---|---|
| `json.parse(text) ! JsonError` | The `Json` value written in `text`, which must be standard JSON (RFC 8259) |
| `json.parse(text, comments: true) ! JsonError` | The same, also taking `//` and `/* */` comments and a trailing comma in arrays and objects (JSONC, as configuration files are written) |
| `json.decode<T>(text) ! JsonError` | A `T` read straight from `text`, with no `Json` in between |
| `json.decode_value<T>(j) ! JsonError` | A `T` read from the `Json` value `j` |
| `json.decode<T>(text, unknown: JsonUnknown.Ignore)` | The same, skipping fields `T` doesn't have (`decode_value` takes it too) |
| `json.of(value, big_as_text: false)` | `value` as a `Json`; `big_as_text: true` writes each `BigInt` and `Decimal` as text |
| `json.stringify(value, indent: 0, big_as_text: false)` | JSON text for any value; `indent` above 0 puts each field on its own line |
| `json.schema<T>()` | The [JSON Schema](https://json-schema.org) of `T` as a `Json`: what `ask<T>` sends a model (8.4), for modules that work from a type's shape (14.14). A `secret` field (2.11) is marked `"writeOnly": true`, so a library sees that it can't be written out |
| `json.describe(f) -> FnSchema` | The function `f` described: its name, doc comment, parameters with their schemas, the schemas of its result and error, the capabilities it `uses`, and its `example` clauses as text |
| `json.decode_texts<T>(names, texts) ! JsonError` | A record `T` read from the texts of its fields, such as a CSV row's cells: field `names[i]` from `texts[i]` (below) |
| `json.encode_texts(value) -> [String?]` | The fields of the record `value`, in the order written, each as `print` writes it: the reverse of `decode_texts` (below) |

**Encoding** works on every value ([decision 0132](../../decisions/0132-json-for-every-value.md)):

- Numbers (sized integers too), `Bool`, `String`, and `Char` (text of one character). `Decimal` and `BigInt` are numbers with all their digits; `big_as_text: true` writes them as text instead, for readers such as JavaScript that round numbers past 2^53. `NaN` and infinities are written as `null`, and a negative zero `Float` as `0.0`, as JavaScript does; JSON text read in and written back keeps its own `-0.0`.
- Lists, sets and tuples are arrays. Records are objects, with the fields in declared order. Dicts are objects, with their keys as text (below).
- An optional is its value, or `null` for `none`. A union variant without fields is its name as text; one with fields is `{"Variant": {fields}}`. A variant that carries a value (2.6) is that value instead: its text, or its number (below).
- `Bytes` is base64 text, and `std.time`'s `Date`, `Time` and `DateTime` are ISO 8601 text (both below). `Json` is itself.
- `Shared(x)` is `x`. A `Weak` is its value while it lives, and `null` once it was freed.
- A `secret` field (2.11) is written `"<secret>"`, as `print` shows it (1.11), so a secret never reaches JSON.
- Values with no JSON meaning, such as functions, `Task`, `Channel`, files and sockets, are written as their printed form, `"<function>"`. This goes one way: `json.decode` of a type that holds one is a compile error.

**Keys.** A JSON key is text, so each dict key is written as text: a `String` as itself; numbers, `Bool`, `Char` and `BigInt` as their text; a union variant without fields as its name, or as its value when it carries one; and any other key (a list, tuple, record, dict or set) as its own compact JSON, as text. Decoding reads each key back as the dict's key type says, and a key that doesn't read as one is a `Mismatch` at its path.

```kavach
print(json.of({1: "x", 2: "y"}))          // {"1":"x","2":"y"}
print(json.of({[2, 3, 4]: "name"}))       // {"[2,3,4]":"name"}
print(json.of({true: 1, false: 0}))       // {"true":1,"false":0}
const back = try json.decode<{Int: String}>(`\{"1": "x"}`)
print(back[1])                            // x
```

**Variants that carry values** (2.6) are written and read as their values, so a type can hold what another system sends. Writing gives the text, or the number; reading takes only a value, not a name, and anything else is a `Mismatch` expecting "a variant of `PaymentStatus`". A variant still prints as its name.

```kavach
use std.json

type PaymentStatus = Succeeded = "succeeded" | Processing = "processing" | RequiresAction = "requires_action"

type Payment {
    id: String
    status: PaymentStatus
}

const p = try json.decode<Payment>(`\{"id": "pi_1", "status": "requires_action"}`)
print(p.status)                // RequiresAction
print(json.stringify(p))       // {"id":"pi_1","status":"requires_action"}
print(json.of(Code.NotFound))  // 404, for `type Code = Ok = 200 | NotFound = 404`
```

**Decoding** is the reverse, and strict about values: an integer field takes only a number written without a fraction or exponent that fits its type, and a `BigInt` field one of any size, or one written as text; a `Float` field takes any number, as the nearest `Float`; a `Decimal` field takes any number exactly, or one written as text; text, booleans, arrays and objects must match. A number that doesn't fit its field is a `Mismatch`, as any other value that doesn't fit is. A missing field takes its default or `none` if it has one, otherwise decoding fails. A field the type doesn't have is a problem too, at its path, expecting "no field of this name", as the type's schema says (`additionalProperties: false`), so a misspelled or smuggled field is caught ([decision 0096](../../decisions/0096-unknown-fields-are-refused.md)). To read another service's answers, which may gain fields, write `unknown: JsonUnknown.Ignore`: such fields are then skipped. `std.http`'s client does this for the responses it decodes (14.11). In a generic function, `json.decode<T>` of a type parameter is checked where the parameter gets a real type: the call `load<Step>(text)` is the compile error when `Step` holds a function, and so is each call through other generic functions that pass the parameter on. Decoding makes a value only where the code could make it itself: outside its type's module, `json.decode<T>`, `decode_value` and `decode_texts` of a `T` with a private or `pub(read)` field, or of a type private to its module, is a compile error that names the field, wherever the type is inside `T` (10.1); so are `cli.parse<T>` (14.6) and `ask<T>` (8.4). Otherwise text from outside could make the key of a sealed value (2.20).

```kavach
// in module `main`, for `pub type SessionKey { n: Int }` of module `session`
const k = json.decode<session.SessionKey>(`\{"n": 1}`)   // compile error: its field `n` is private
```

**`Bytes` in JSON** is base64 text, as Protocol Buffers' JSON and OpenAPI write bytes. Encoding writes the standard alphabet with padding; decoding reads the standard alphabet or the URL-safe one (`-` and `_`), with or without padding, and other text is a `Mismatch` expecting "base64 text". It holds for a bare value as for a field, in lists, dicts and optionals too. Its schema is `{"type": "string", "contentEncoding": "base64"}`. `ask<T>` (8.4) still refuses `Bytes`: models write base64 badly.

```kavach
print(json.of(Bytes([104, 105])))                    // "aGk="
type Blob {
    data: Bytes
}
const b = try json.decode<Blob>(`\{"data": "aGk"}`)   // URL-safe or unpadded reads too
print(b.data)                                        // Bytes(6869)
```

**Dates and times** from `std.time` (14.4) are ISO 8601 text, as their `iso()` writes them: `"2024-01-02"`, `"03:04:05.120"`. A `DateTime` gets a `Z`, `"2024-01-02T03:04:05Z"`: JSON holds it in UTC, as `time.now()` and `time.utc` give it, so other programs read the right moment. Decoding reads the text as `time.parse_date` and `time.parse_time` do, so digits of a second past milliseconds are dropped (`03:04:05.123456` is 123 ms). A `DateTime` may end in `Z`, in an offset, which is turned to UTC (`2024-01-01T02:00:00+05:30` is `2023-12-31T20:30:00`), or in neither. Anything else, such as PostgreSQL's `infinity`, is a problem expecting ``a date such as `2024-01-02` ``, ``a time of day such as `03:04:05` `` or ``a date and time such as `2024-01-02T03:04:05Z` ``. In a schema, a `Date` is `{"type": "string", "format": "date"}`, a `DateTime` has `"format": "date-time"`, and a `Time` has a `"pattern"` (JSON Schema's `time` format needs an offset).

```kavach
use std.json
use std.time.{Date, DateTime}

type Booking {
    day: Date
    made: DateTime
}

const b = try json.decode<Booking>(`\{"day": "2024-01-02", "made": "2024-01-01T02:00:00+05:30"}`)
print(json.stringify(b))   // {"day":"2024-01-02","made":"2023-12-31T20:30:00Z"}
```

```kavach
const text = `\{"id": 1, "note": "x"}`
const a = json.decode<Order>(text)                               // Invalid: `note` is no field of `Order`
const b = json.decode<Order>(text, unknown: JsonUnknown.Ignore)  // `note` is skipped
```

**A type's schema** is a `Json` value a program can read: a record is `{"type": "object", "properties": {...}, "required": [...]}` with its fields in the order they are declared, and an anonymous record (2.14) the same, its fields in the order of their names, `Int` and the sized integers are `{"type": "integer"}`, with `"minimum": 0` for the unsigned ones (`UInt8` to `UInt64`), `Float` and `Decimal` `{"type": "number"}`, `Bool` `{"type": "boolean"}`, `String` `{"type": "string"}`, a list `{"type": "array", "items": ...}`, an optional `{"anyOf": [..., {"type": "null"}]}`, a union `{"anyOf": [...]}` of its variants as JSON writes them (for variants that carry values, `{"type": "string", "enum": [...]}` of the texts or `{"type": "integer", "enum": [...]}` of the numbers), and a dict with `String` keys `{"type": "object", "additionalProperties": ...}`. A type's doc comment (`///`) becomes its schema's `"description"`, and so does a field's. When both a field and its type have one, the field's comes first in one `"description"`, then the type's: `"How sure the model is. How sure."`. A type with no schema (dicts with other keys, sets, tuples, functions) is a compile error, and so is a record or union holding one, however deep. `ask<T>` (8.4) still refuses dicts: a model answers closed objects only.

```kavach
type Point {
    x: Int
    y: Float
}

print(json.schema<Point>().get("properties"))   // {"x":{"type":"integer"},"y":{"type":"number"}}
```

```kavach
/// A place on the map.
type Place {
    /// How far east, in metres.
    x: Int
}

print(json.schema<Place>().get("description"))   // "A place on the map."
```

```kavach
type Rates {
    base: String
    rates: {String: Float}
}

const r = try json.decode<Rates>(body)
```

**A function described.** `json.describe(f)` works from `f`'s signature when the program is compiled; when `f` names a function, it gives a constant `FnSchema`, and reading it costs nothing more than reading any value. A web framework uses it, with `json.schema<T>()`, to write an OpenAPI description of its handlers ([decision 0085](../../decisions/0085-function-types-with-errors-and-uses.md)).

```kavach
pub type FnSchema {
    pub name: String
    pub doc: String               // the `///` comment, its lines joined by spaces; "" when it has none
    pub params: [ParamSchema]
    pub result: Json              // the schema of what it returns; {"type": "null"} for nothing
    pub error: Json               // the schema of its error; null when it can't fail
    pub capabilities: [String]    // what it `uses`, written or worked out (8.1)
    pub examples: [String]        // each `example` clause as written (7.6)
}

pub type ParamSchema {
    pub name: String
    pub schema: Json
}
```

```kavach
/// Makes an order.
fn create(o: NewOrder) -> Order ! ApiError uses Db { … }

/// The price of `qty` items.
fn price(qty: Int) -> Int
    example price(3) == 30
{ … }

const d = json.describe(create)
print(d.doc)                     // Makes an order.
print(d.params[0].name)          // o
print(d.capabilities)            // ["Db"]
print(json.describe(price).examples)   // ["price(3) == 30"]
```

A parameter, result or error whose type has no schema (a function, a dict with other keys) is a compile error, as for `json.schema<T>()`.

**A function value described.** Given a function value instead of a name, such as a parameter or an item of a list of handlers, `json.describe(value)` gives `FnSchema?`: the description of the named function the value is, or `none` for a lambda (and for a generic function, which has no one description). It works inside a generic function too, with the types it was called with. A parameter, result or error with no schema (a function, a record holding one, the built-in `Error`) is `{}`, any value, there, so every named function can be described. A framework that is handed its routes as values can describe each one.

```kavach
/// Lists the orders.
fn list_orders() -> [Order] { … }

fn summary(handler: fn() -> [Order]) -> String {
    if let d = json.describe(handler) {
        return d.doc
    }
    "no description"
}

print(summary(list_orders))         // Lists the orders.
print(summary(() => []))            // no description
```

**Safe by default.** Text from outside can be hostile, so parsing and decoding have limits, which a call can raise: nesting deeper than `max_depth: 128` (at most 10_000, whatever the call says), text longer than `max_bytes: 64_000_000`, or an object with more than `max_fields: 100_000` fields is an error, not a stack overflow or a memory blow-up. A key that appears twice in one object is an error. Reading a number never rounds it silently: a number a field can't hold is an error, and a method gives `none`. A number whose exponent puts it beyond 10^10_000, or below 10^-10_000, is out of range for `decimal()` and `bigint()` too, so a short text like `1e999999999` can't make a huge value. Invalid escapes, control characters in text and unpaired surrogates are errors too. `NaN`, `Infinity`, hex numbers and the rest of JSON5 are syntax errors, even with `comments: true`.

```kavach
const big = try json.parse(body, max_depth: 1_000)

const doc = try json.parse(`{\`id": 12345678901234567890, "price": 0.1}`)
print(doc["id"].bigint())       // 12345678901234567890
print(doc["id"].int())          // none: too big for Int
print(doc["price"].decimal())   // 0.1, exactly}`
```

**Fields read from text.** `json.decode_texts<T>(names, texts)` makes a record `T` from one text for each field, as a CSV row, a form or a query string has them, with no `Json` in between: `names` and `texts` are two lists of the same length (a panic otherwise). Each text is read as its field's type reads text (2.1): `Int(s)`, `Float(s)` or `Decimal(s)` for a number, with no spaces around it; `true` or `false` for a `Bool`; and as a JSON string is read for a `String`, a `Char`, a date or a category (a union whose variants have no fields), by its variant's name. A float takes `NaN`, `inf` and `-inf`, as `print` writes them, and a `Float32` is read straight to single precision. A `none` text is `null`: `none` for an optional field. A field not named takes its default, as in `decode`, and the invariants are checked. A text that doesn't fit is a problem at the field's path, expecting `a whole number`, `a number` or `` `true` or `false` ``; anything else is as `decode` says it. A name `T` has no field for is a problem too. `std.table` reads each CSV row this way (14.34) (for a maintainer's review).

`json.encode_texts(value)` is the reverse: each field of the record `value`, in the order written, as `print` writes it (1.11), so `decode_texts` reads it back: `0.1` for a `Float32`, `NaN`, `inf`, `-0.0`, a date as ISO 8601, a category as its variant's name. A field that is `none` gives `none`, and a `secret` field (2.11) `"<secret>"`. A value that is not a record gives one text, its own. `std.table` writes each CSV row this way (for a maintainer's review).

```kavach
type Reading {
    sensor: String
    value: Float32
    ok: Bool = true
}

const r = try json.decode_texts<Reading>(["sensor", "value"], ["a7", "0.1"])
print(json.encode_texts(r))            // ["a7", "0.1", "true"]
```

A `JsonError` says what went wrong and where:

```kavach
type JsonError = Syntax(line: Int, column: Int, message: String) |
    TooDeep(limit: Int) |
    TooLarge(limit: Int) |
    DuplicateKey(key: String, line: Int, column: Int) |
    Invalid(problems: [JsonProblem])

pub type JsonProblem {
    pub path: String
    pub expected: String
}
```

`Invalid` comes from decoding, and lists **every** problem in the document, in its order, not only the first, so a service can answer with all of them at once ([decision 0086](../../decisions/0086-decoding-reports-every-problem.md)). Each problem's `path` is where in the document (`items[2].price`, `""` for the whole), and `expected` what the type wanted there, naming the field's type where a number doesn't fit it: ``a whole number that fits `UInt8` `` for `300` in a `UInt8` field.

A decoded value is checked against its types' `invariant`s (7.3), as any value is when it is made. Text from outside breaking one is not a mistake in the program, so it is a problem too, not a panic: its `path` is the record's, and `expected` is ``invariant `qty > 0` ``. The invariants are checked once every value has the right shape: until then, the problems are those of shape.

```kavach
type Item {
    name: String
    qty: Int

    invariant qty > 0
}

const text = `[\{"name": 5, "qty": 1}, \{"qty": 2}]`
const items = json.decode<[Item]>(text) catch e => match e {
    JsonError.Invalid(problems) => {
        for p in problems {
            print(`{p.path}: {p.expected}`)      // [0].name: text
        }                                        // [1]: a field `name`
        []
    }
    _ => [],
}
const none_left = json.decode<[Item]>(`[\{"name": "pen", "qty": 0}]`) catch e => {
    print(e)    // Invalid(problems: [JsonProblem(path: "[0]", expected: "invariant `qty > 0`")])
    []
}
```
