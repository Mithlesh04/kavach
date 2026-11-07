# 0070. Variants are written with their type

- **Status:** Accepted
- **Date:** 2026-11-05

## Context

A union's variant could be written bare when the name was unambiguous: `Pending` for `Status.Pending`, `NotFound` for `Code.NotFound` (spec 2.6). A name two unions shared was an error unless the type settled it, and a program's own variant silently won over a standard library one, so `print(NotFound)` in a program using `std.fs` meant `Code.NotFound` without a word, a rule the spec never said.

The type checker caught most wrong picks, but not where any type fits (`print`, `Json`, interface values). And a reader can't tell a bare `Pending` from a constant or a variable: adding a union with a `Pending` variant, in the program or in a module it uses, changes what existing lines mean or breaks them.

Other languages: Rust, TypeScript, Kotlin, C# and Java (outside `switch`) write the type (`Status::Pending`, `Status.Pending`); Swift allows `.pending` where the type is known; Haskell, OCaml and Elm use bare constructors, which must then be unique in scope.

## Decision

**A variant is always written with its union's name: `Status.Pending`, `Code.NotFound`, `Shape.Circle(radius: 2.0)`, in expressions and in `match` cases alike.** Mithlesh chose this on 2026-11-05.

```kavach
type Status = Pending | Paid | Refunded

const chosen = Status.parse("Paid") ?? Status.Pending
match chosen {
    Status.Paid => print("paid"),
    _ => print("not paid"),
}
```

A bare variant name is a compile error that says what to write, and `kavach fix` rewrites it.

## Consequences

- What a name means never depends on which other unions exist: a new union or module can't change or break a line.
- No hidden rule picks between a program's variant and a standard library one.
- Code is longer, most of all in `match` cases. Every example, test, standard library module and spec sample is rewritten in the same change, and the docs site's samples with it.

## Alternatives considered

- **Bare only where the type is known** (Swift): shorter, but a reader still has to work out the type to know what a name is, and the rule has edges (which places count as known).
- **Keep the old rule and write down "own wins"**: no rewriting, but the confusion stays.
