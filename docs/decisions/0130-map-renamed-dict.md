# 0130. `Map` renamed `Dict`

- **Status:** Accepted
- **Date:** 2026-11-18

## Context

Maps are written with braces since [decision 0129](0129-maps-written-with-braces.md): `{"asha": 31}`. Written so, a reader takes the value for Python's `dict`, and a maintainer said on 2026-11-18 "it's not a map, it's a dict". Asked to choose between renaming only the names, renaming the names and the type's shorthand, or keeping `Map`, the maintainer chose the names and the shorthand.

## Decision

```kavach
const ages: {String: Int} = {"asha": 31, "ravi": 27}
let cache: {Int: String} = Dict.capacity(100)
const empty: Dict<String, Int> = {:}
const names = ages.keys()
```

1. The type is `Dict<K, V>`, and its shorthand `{K: V}`: `{String: Int}`. `[String: Int]` is a compile error that says to write braces. Lists keep `[T]`.
2. `Map.capacity(n)` is `Dict.capacity(n)`, and every other place the type is named says `Dict`. Messages say "dict": "a `gpu fn` can't use a dict".
3. The list method `map(f)` and a GPU `map` keep their name: they map, they aren't the type.
4. Dict patterns keep their brackets, `const [host: h] = config`, as braces in a pattern are a record's.
5. Inside the compiler and runtime the old names may stay where no program sees them.

## Consequences

- Every `[K: V]` type, `Map.capacity` and "map" meaning the type changes, in the repository, the docs site and the packages.
- The docs site changes for decisions 0129 and 0130 in one round.

## Alternatives considered

- **Names only:** `Dict` but `[K: V]` for the type, as Swift's `[K: V]` beside the braces; two bracket styles for one type.
- **Keeping `Map`:** familiar from Java, Go and Rust, but not what braces suggest.
