# 2.12 Json

`Json` holds data whose shape is only known when the program runs: a JSON document from a file or a web service, or a value you build to send. It is a built-in union, so `match` sees every case:

```kavach
type Json = Null |
    Boolean(value: Bool) |
    Number(digits: String) |
    Text(value: String) |
    Array(items: [Json]) |
    Object(fields: [String: Json])
```

A `Number` keeps the digits it was written with, exactly (`"12345678901234567890"`, `"0.1"`, `"-2.5e10"`), so no number is changed by reading it and writing it out again ([decision 0071](../../decisions/0071-json-numbers-keep-their-digits.md)). The methods below read it as the number type a program needs. Objects keep their keys in order, like maps (2.2).

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
| `bigint()` | `BigInt?`: a `Number` written without a fraction or exponent, of any size |
| `float()` | `Float?`: a `Number`, as the nearest `Float`; `none` past `Float`'s range |
| `decimal()` | `Decimal?`: a `Number`, exactly |
| `bool()` | `Bool?`: the value of a `Boolean` |
| `list()` | `[Json]?`: the items of an `Array` |
| `object()` | `[String: Json]?`: the fields of an `Object` |
| `is_null()` | `Bool`: whether it is `Null` |
| `len()` | `Int`: the items of an `Array` or fields of an `Object`, otherwise 0 |

### Building

Where a `Json` is expected, literals become `Json` values: numbers, text, `true`, `false`, `none` (which becomes `Null`), lists and maps with text keys, at any depth. Inside such a literal, any other expression whose type can be encoded (below) is converted too:

```kavach
const order: Json = [
    "id": 1042,
    "customer": user.name,
    "items": ["pen", "ink"],
    "paid": true,
    "note": none,
]
```

`json.of(value)` converts any value that can be encoded. A `Json` is a value like any other (chapter 6): copies never share changes, and it is freed as soon as nothing holds it. It prints as compact JSON text. Two `Json` values are equal when they are the same variant with equal contents; objects compare like maps, so the order of keys doesn't count (1.8).

### std.json

| Function | Gives |
|---|---|
| `json.parse(text) ! JsonError` | The `Json` value written in `text`, which must be standard JSON (RFC 8259) |
| `json.parse(text, comments: true) ! JsonError` | The same, also taking `//` and `/* */` comments and a trailing comma in arrays and objects (JSONC, as configuration files are written) |
| `json.decode<T>(text) ! JsonError` | A `T` read straight from `text`, with no `Json` in between |
| `json.decode_value<T>(j) ! JsonError` | A `T` read from the `Json` value `j` |
| `json.decode<T>(text, unknown: JsonUnknown.Ignore)` | The same, skipping fields `T` doesn't have (`decode_value` takes it too) |
| `json.of(value)` | `value` as a `Json` |
| `json.stringify(value, indent: 0)` | JSON text for a `Json` or any value that can be encoded; `indent` above 0 puts each field on its own line |
| `json.schema<T>()` | The [JSON Schema](https://json-schema.org) of `T` as a `Json`: what `ask<T>` sends a model (8.4), for modules that work from a type's shape (14.14) |
| `json.describe(f) -> FnSchema` | The function `f` described: its name, doc comment, parameters with their schemas, the schemas of its result and error, the capabilities it `uses`, and its `example` clauses as text |

**Encoding** works on types the compiler can see through: numbers (sized integers too), `Bool`, `String`, `Char` (text of one character), `Decimal` and `BigInt` (as numbers with all their digits), lists, sets and tuples (arrays), maps with `String` keys (objects), records (objects with the fields in declared order), optionals (`none` is `null`), unions (a variant without fields is its name as text; one with fields is `{"Variant": {fields}}`) and `Json` itself. Anything else is a compile error: functions, maps with other keys, and records with `secret` fields (2.11), so a secret can't leak into JSON by accident. `NaN` and infinities are written as `null`, as JavaScript does.

**Decoding** is the reverse, and strict about values: an integer field takes only a number written without a fraction or exponent that fits its type, and a `BigInt` field one of any size; a `Float` field takes any number, as the nearest `Float`; a `Decimal` field takes any number exactly; text, booleans, arrays and objects must match. A number that doesn't fit its field is a `Mismatch`, as any other value that doesn't fit is. A missing field takes its default or `none` if it has one, otherwise decoding fails. A field the type doesn't have is a problem too, at its path, expecting "no field of this name", as the type's schema says (`additionalProperties: false`), so a misspelled or smuggled field is caught ([decision 0096](../../decisions/0096-unknown-fields-are-refused.md)). To read another service's answers, which may gain fields, write `unknown: JsonUnknown.Ignore`: such fields are then skipped. `std.http`'s client does this for the responses it decodes (14.11).

```kavach
const text = "\{\"id\": 1, \"note\": \"x\"}"
const a = json.decode<Order>(text)                               // Invalid: `note` is no field of `Order`
const b = json.decode<Order>(text, unknown: JsonUnknown.Ignore)  // `note` is skipped
```

**A type's schema** is a `Json` value a program can read: a record is `{"type": "object", "properties": {...}, "required": [...]}` with its fields in the order they are declared, `Int` and the sized integers are `{"type": "integer"}`, `Float` and `Decimal` `{"type": "number"}`, `Bool` `{"type": "boolean"}`, `String` `{"type": "string"}`, a list `{"type": "array", "items": ...}`, an optional `{"anyOf": [..., {"type": "null"}]}`, a union `{"anyOf": [...]}` of its variants as JSON writes them, and a map with `String` keys `{"type": "object", "additionalProperties": ...}`. A type's doc comment (`///`) becomes its schema's `"description"`, and so does a field's. A type with no schema (maps with other keys, tuples, functions) is a compile error. `ask<T>` (8.4) still refuses maps: a model answers closed objects only.

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
    rates: [String: Float]
}

const r = try json.decode<Rates>(body)
```

**A function described.** `json.describe(f)` works from `f`'s signature when the program is compiled, so `f` must name a function, not hold a function value; it gives a constant, and reading it costs nothing more than reading any value. A web framework uses it, with `json.schema<T>()`, to write an OpenAPI description of its handlers ([decision 0085](../../decisions/0085-function-types-with-errors-and-uses.md)).

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

A parameter, result or error whose type has no schema (a function, a map with other keys) is a compile error, as for `json.schema<T>()`.

**Safe by default.** Text from outside can be hostile, so parsing and decoding have limits, which a call can raise: nesting deeper than `max_depth: 128` (at most 10_000, whatever the call says), text longer than `max_bytes: 64_000_000`, or an object with more than `max_fields: 100_000` fields is an error, not a stack overflow or a memory blow-up. A key that appears twice in one object is an error. Reading a number never rounds it silently: a number a field can't hold is an error, and a method gives `none`. A number whose exponent puts it beyond 10^10_000, or below 10^-10_000, is out of range for `decimal()` and `bigint()` too, so a short text like `1e999999999` can't make a huge value. Invalid escapes, control characters in text and unpaired surrogates are errors too. `NaN`, `Infinity`, hex numbers and the rest of JSON5 are syntax errors, even with `comments: true`.

```kavach
const big = try json.parse(body, max_depth: 1_000)

const doc = try json.parse("{\"id\": 12345678901234567890, \"price\": 0.1}")
print(doc["id"].bigint())       // 12345678901234567890
print(doc["id"].int())          // none: too big for Int
print(doc["price"].decimal())   // 0.1, exactly
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

const text = "[\{\"name\": 5, \"qty\": 1}, \{\"qty\": 2}]"
const items = json.decode<[Item]>(text) catch e => match e {
    JsonError.Invalid(problems) => {
        for p in problems {
            print("{p.path}: {p.expected}")      // [0].name: text
        }                                        // [1]: a field `name`
        []
    }
    _ => [],
}
const none_left = json.decode<[Item]>("[\{\"name\": \"pen\", \"qty\": 0}]") catch e => {
    print(e)    // Invalid(problems: [JsonProblem(path: "[0]", expected: "invariant `qty > 0`")])
    []
}
```
