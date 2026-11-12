# 0096. Decoding refuses fields the type doesn't have

- **Status:** Accepted
- **Date:** 2026-11-12

## Context

`json.decode<T>` skipped fields `T` doesn't have, while `json.schema<T>()` said `additionalProperties: false`. The two disagreed. Rath, a web framework written in Kavach, found this (2026-11-12): a request with a misspelled field (`quantiy`) was taken as if the field were missing, and a smuggled field went unnoticed.

A maintainer chose (2026-11-12) to refuse unknown fields. Asked again about clients, whose services may add fields to their answers, they chose to let a caller skip them.

## Decision

- A field the type doesn't have is a problem in `JsonError.Invalid`, at its path, expecting "no field of this name". Every one is listed, with the other problems (decision 0086).
- `json.decode` and `json.decode_value` take `unknown: JsonUnknown.Ignore` to skip such fields. `JsonUnknown.Refuse` is the default.
- `std.http`'s client skips them in the responses it decodes, so a service can add fields without breaking its clients. A server's `http.read_json` refuses them.
- Other readers of saved values, such as an agent's state, still skip them, so a field removed from a type doesn't stop old state from loading.

## Consequences

- Decoding and the type's schema now agree.
- A client of another service that decodes with `json.decode` directly writes `unknown: JsonUnknown.Ignore`.

## Alternatives considered

- **Strict everywhere:** typed clients would break whenever a service adds a field.
- **Ignore everywhere, and drop `additionalProperties: false` from schemas:** misspelled fields would stay silent.
