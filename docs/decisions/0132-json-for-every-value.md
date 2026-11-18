# 0132. JSON for every value and key

- **Status:** Accepted
- **Date:** 2026-11-18

## Context

`json.of` and `json.stringify` refused many values at compile time: a dict whose keys aren't `String`, a function, and any record with a `secret` field. Programs that log or send such values had to copy them into another shape first. A maintainer asked on 2026-11-18 that every value convert, with keys as text, and chose among keys as text, dicts as `[key, value]` pairs, and keeping the refusals.

## Decision

```kavach
print(json.of({1: "x", 2: "y"}))              // {"1":"x","2":"y"}
print(json.of({[2, 3, 4]: "name"}))           // {"[2,3,4]":"name"}
print(json.of(Card(holder: "Asha", number: "4242")))   // {"holder":"Asha","number":"<secret>"}
print(json.of(big, big_as_text: true))        // "123456789012345678901234567890"
```

1. **Keys are always JSON text.** `String` keys are themselves; numbers, `Bool`, `Char` and `BigInt` are their text; a variant without fields is its name; any other key (a list, tuple, record, dict or set) is its own compact JSON, as text. `json.decode` reads each back as the key type says.
2. **`BigInt` and `Decimal` are exact numbers.** `big_as_text: true` writes them as text, for readers such as JavaScript that round numbers past 2^53. Decoding, `.bigint()` and `.decimal()` read both forms.
3. **Every value has a JSON form.** Lists, sets and tuples are arrays, records objects, unions as before, `T?` its value or `null`, `Bytes` base64, `Shared(x)` as `x`, `Weak` as its value or `null`.
4. **Values with no JSON meaning** (functions, `Task`, `Channel`, files, sockets) are written as their printed form, `"<function>"`. This goes one way: decoding a type that holds one is a compile error.
5. **A `secret` field is written `"<secret>"`**, as `print`, logs and the audit log show it (1.11, 8.7). This replaces the compile error: the secret still never reaches the JSON, and a record holding one can be logged or sent.

## Consequences

- `json.of` and `json.stringify` accept any value; only `json.decode` and `json.schema` refuse types.
- Code that relied on the compile error to keep a record with a secret out of JSON now gets `"<secret>"` in its place.
- A JSON Schema is unchanged: dicts with other keys, sets, tuples and functions still have none.

## Alternatives considered

- **Dicts as `[key, value]` pairs:** exact for any key, but not the object every other tool expects.
- **Keeping the refusals:** safest, but every program that logs a value with an `Int`-keyed dict needed a copy first.
