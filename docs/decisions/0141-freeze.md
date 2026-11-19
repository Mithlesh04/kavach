# 0141. Frozen values: `x.freeze()`, `Frozen<T>` and `f.unfreeze()`

- **Status:** Accepted
- **Date:** 2026-11-19

## Context

Values in Kavach are copies (spec 6.1), so a library's own data never changes when a caller changes what it was given. What a library couldn't do was stop a caller from changing the value in its own `let` copy, and so keep a value it hands out as it made it: a parsed configuration, a session, a list of rules. `pub(read)` fields (spec 10.1) solve this for one field of a type the library declares. Lists, dicts, and types the library didn't design needed something too (to-do item 19).

How other languages do it:

- **JavaScript:** `Object.freeze(o)` stops changes to `o`'s own properties, at run time, and only at the top level: nested objects stay changeable unless frozen too.
- **Rust:** a value without `mut` can't change; `&T` lends it for reading. The type system knows which functions only read.
- **Swift, Kotlin:** `let` and read-only collection types (`List` against `MutableList`).
- **Immutable.js, Python's `frozenset` and `tuple`:** separate types that have no changing methods.

## Decision

A maintainer chose (2026-11-16, 2026-11-19):

- `x.freeze()` gives a `Frozen<T>` whose fields and items never change, even in a `let`; `f.unfreeze()` gives a copy that can change. For every type.
- **Deep by default:** `freeze()` freezes every nested list, dict and record; `freeze(nested: false)` freezes only the top level; `unfreeze()` gives a copy changeable all the way down. The other option was shallow by default, as JavaScript.
- **Sealed with a key:** `x.freeze(key: k)` opens only with `f.unfreeze(key: k)` and an equal key. A library makes the key a value of a type private to its module. A wrong key is caught by its type while compiling and by its value while running.
- **Where a `T` is written,** a `Frozen<T>` goes only to a function that only reads that parameter, which the compiler knows. So a key seals, and reading functions stay plain. The other options were: always, as a copy, which would let any function hand back an open copy; or only where `Frozen<T>` is written, which would make every reading function say so.
- `f with { x: 1 }` gives a new value that isn't frozen.

What wasn't decided was chosen as the cheapest that fits (spec 2.20 lists the open points):

- A value frozen without a key is the plain value while running: freezing and unfreezing cost nothing. A sealed value is the value with its key beside it.
- A frozen value prints and encodes as its `T`. A part read out of it is a plain copy, as any value read out is (6.1).
- A wrong key while running is a panic (5.6), as it is a mistake in the program, not a condition to handle.
- "Only reads" is worked out from the function's body, by the uses of the parameter's name (spec 2.20 lists them). It looks into the functions and methods the parameter is passed on to, and assumes a call back into a function being worked out only reads.

## Consequences

- A library can return a list, a dict or a record and know it stays as made, without declaring a type for it.
- Reading code needs no change: `fn total(o: Order)` takes a frozen order.
- A function that copies its parameter into a `let` can't take a frozen value, even when it changes only its copy. Its caller passes `f.unfreeze()`.
- The checker's types gain `Frozen`, which reads see through, as they see through a collection's numbers (spec 2.9).

## Alternatives considered

- **Run-time freezing, as JavaScript.** A change would be found only when it runs, and every change would pay for a check. Kavach checks it while compiling.
- **Read-only types per collection** (`FrozenList`, `FrozenDict`). Each would need its own methods, and types a library didn't design would have none.
- **Only `pub(read)`.** It covers a library's own records, not the lists and dicts it hands out.
