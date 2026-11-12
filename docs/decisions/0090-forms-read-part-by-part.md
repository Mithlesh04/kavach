# 0090. Forms read part by part

- **Status:** Accepted
- **Date:** 2026-11-12

## Context

HTML forms with files are sent as `multipart/form-data`: the body holds parts, each with its own headers, divided by a boundary. Rath, a web framework written in Kavach, asked (2026-11-12) for streamed multipart uploads, so a large file never sits in memory. Bodies larger than `buffered_body` are already read as they come ([decision 0089](0089-streamed-http-bodies.md)).

A maintainer chose (2026-11-12) the first of the options put to them.

## Decision

- **`http.form(r) -> Form`** (spec 14.11) reads a form a part at a time: `form.next()` gives the next `FormPart`, with its `name`, `filename` and `content_type`, and `part.read()` its data a piece at a time; `part.text()` reads a small field whole.
- It works the same on a body read whole and on one read with `r.read_body()`.
- It is written in Kavach, in `std.http.multipart`, on a new `Bytes` method, `b.index_of(part)` (spec 2.13), which finds the boundary without a loop over every byte.

## Consequences

- An upload of any size needs about one piece of memory, plus the boundary.
- A part not read to its end is skipped by `next()`, so a handler can look only at the fields it wants.

## Alternatives considered

- **Every part whole** (`http.form_parts(r) -> [FormPart]`): simplest, but a large upload would sit in memory.
- **Both:** the whole form can be built on the part reader later if it is wanted.
