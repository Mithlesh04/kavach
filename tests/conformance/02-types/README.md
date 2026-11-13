# 2. Types

Conformance tests for spec chapter [2. Types](../../../docs/spec/02-types/README.md). Format: [../README.md](../README.md).

| Spec | Test | Rule | Expects |
|---|---|---|---|
| 2.1 | [explicit-number-conversion.kav](explicit-number-conversion.kav) | Number types convert explicitly with `T(value)`; reading text can fail. | runs |
| 2.1 | [text-literal-conversions-are-checked.kav](text-literal-conversions-are-checked.kav) | Literal text that isn't a valid value of the type is a compile error. | compile error |
| 2.1 | [conversions-of-text-and-characters.kav](conversions-of-text-and-characters.kav) | Text converts to numbers, `Bool` and `Char`; characters by code point. | runs |
| 2.1 | [int-parse-is-gone.kav](int-parse-is-gone.kav) | `Int.parse(text)` is gone: every conversion is `T(value)`. | compile error |
| 2.1 | [no-implicit-number-conversion.kav](no-implicit-number-conversion.kav) | There are no implicit conversions between number types. | compile error |
| 2.1 | [big-constants-are-bigint.kav](big-constants-are-bigint.kav) | A constant that doesn't fit `Int`, with no type written, is an exact `BigInt`. | runs |
| 2.1 | [bigint-arithmetic.kav](bigint-arithmetic.kav) | `BigInt` takes arithmetic, comparisons and `==`, is a map key, and converts. | runs |
| 2.1 | [constants-must-fit-their-type.kav](constants-must-fit-their-type.kav) | A constant that doesn't fit its written type is a compile error. | compile error |
| 2.1, 5.6 | [bigint-division-by-zero-panics.kav](bigint-division-by-zero-panics.kav) | Dividing a `BigInt` by zero panics. | panic |
| 2.1 | [decimal-has-no-size-limit.kav](decimal-has-no-size-limit.kav) | A `Decimal` keeps every digit, however many. | runs |
| 2.1 | [constants-are-exact.kav](constants-are-exact.kav) | A constant of number literals is worked out exactly, then rounded once. | runs |
| 2.1, 5.6 | [int-of-nan-panics.kav](int-of-nan-panics.kav) | A float that isn't a number doesn't fit any whole-number type. | panic |
| 2.1 | [float32-is-single-precision.kav](float32-is-single-precision.kav) | `Float32` rounds to single precision at every step and prints its shortest form. | runs |
| 2.2 | [collections-change-in-place.kav](collections-change-in-place.kav) | `items[i] = v`, `map[key] = v`, `push` and `remove` change a `let` collection, also inside a field. | runs |
| 2.2 | [list-and-map-access.kav](list-and-map-access.kav) | list.get(i) and map[key] return optionals. | runs |
| 2.2 | [a-list-mapped-into-itself-keeps-its-copies.kav](a-list-mapped-into-itself-keeps-its-copies.kav) | `xs = xs.map(f)` gives `xs` the new items and leaves every copy of the old list as it was. | runs |
| 2.2 | [maps-keep-insertion-order.kav](maps-keep-insertion-order.kav) | Maps keep insertion order when iterated. | runs |
| 2.2 | [push-needs-a-var.kav](push-needs-a-var.kav) | Methods that change a collection (`push`, `remove`) need a `let`. | compile error |
| 2.2 | [lists-hold-any-text.kav](lists-hold-any-text.kav) | A list of text gives back every item as it was put in, short or long, ASCII or not, and copies of the list are independent. | runs |
| 2.2 | [maps-and-sets-with-room.kav](maps-and-sets-with-room.kav) | `Map.with_capacity(n)` and `Set.with_capacity(n)` make empty collections with room for `n` entries; a hint, not a limit. | runs |
| 2.2 | [small-signed-lists-keep-their-sign.kav](small-signed-lists-keep-their-sign.kav) | A list of `Int8` holds negative numbers, which print, compare and sort as numbers. | runs |
| 2.2 | [small-number-lists-keep-every-value.kav](small-number-lists-keep-every-value.kav) | A list of 16- or 32-bit numbers holds every value of its item type, which it gives back, sorts and copies unchanged. | runs |
| 2.3 | [tuples.kav](tuples.kav) | Tuples group values and can be destructured. | runs |
| 2.4 | [record-defaults.kav](record-defaults.kav) | Records are created with named arguments; fields with defaults may be left out. | runs |
| 2.4 | [record-needs-every-field.kav](record-needs-every-field.kav) | A record field without a default must be given. | compile error |
| 2.4 | [record-needs-named-arguments.kav](record-needs-named-arguments.kav) | Records are created with named arguments, not positional ones. | compile error |
| 2.4 | [records-are-values.kav](records-are-values.kav) | Changing a field through a var changes only that variable's value. | runs |
| 2.4 | [lists-of-records-are-values.kav](lists-of-records-are-values.kav) | A list of records holds values: changing a copy, or an item taken out, never changes the original. | runs |
| 2.4 | [with-makes-a-copy.kav](with-makes-a-copy.kav) | `with` creates a copy with some fields changed. | runs |
| 2.4 | [record-fields-that-may-be-left-out.kav](record-fields-that-may-be-left-out.kav) | `summary?: String` is short for `summary: String? = none`: a record made or decoded without it holds `none`, and `ask` doesn't require it. | runs |
| 2.4 | [optional-field-takes-one-question-mark.kav](optional-field-takes-one-question-mark.kav) | A field that may be left out takes its `?` once: `summary?: String?` is an error. | compile error |
| 2.4 | [optional-field-has-no-other-default.kav](optional-field-has-no-other-default.kav) | A field that may be left out is `none` when left out, so it takes no other default: `count?: Int = 3` is an error. | compile error |
| 2.4, 2.9 | [generic-fields-take-defaults.kav](generic-fields-take-defaults.kav) | A field whose type uses the record's type parameter can have a default. | runs |
| 2.5 | [extend-adds-methods.kav](extend-adds-methods.kav) | `extend` adds methods to an existing type. | runs |
| 2.5 | [generic-types-are-extended-elsewhere/](generic-types-are-extended-elsewhere/src/main.kav) | `extend<S> Box<S>` adds methods to a generic type from another module, `S` standing for the type's own parameter. | runs |
| 2.5 | [methods.kav](methods.kav) | Methods take self; functions without self are called on the type. | runs |
| 2.5, 14.2 | [methods-may-share-collection-names.kav](methods-may-share-collection-names.kav) | A type's own `add`, `push` or `remove` is its method, not the built-in one of lists, maps and sets. | runs |
| 2.6 | [variants-are-written-with-their-type.kav](variants-are-written-with-their-type.kav) | A variant is always written with its union's name; a bare name is a compile error. | compile error |
| 2.6 | [variants-named-like-built-in-types.kav](variants-named-like-built-in-types.kav) | A variant may have a built-in type's name: as a value or pattern it means the variant, in a type the built-in type. | runs |
| 2.6 | [variant-fields-have-defaults.kav](variant-fields-have-defaults.kav) | A variant's fields may have defaults, which may be left out. | runs |
| 2.6 | [variant-field-defaults-have-the-field-type.kav](variant-field-defaults-have-the-field-type.kav) | A variant field's default must have the field's type. | compile error |
| 2.6 | [enums-list-name-and-parse.kav](enums-list-name-and-parse.kav) | An enum's `all()`, `name()` and `parse(text)`. | runs |
| 2.6 | [enum-variants-have-numbers.kav](enum-variants-have-numbers.kav) | An enum's variants may have numbers: `value()` and `from(n)`. | runs |
| 2.6 | [enum-numbers-are-all-or-none.kav](enum-numbers-are-all-or-none.kav) | Every variant has a number, or none does. | compile error |
| 2.6 | [enum-helpers-need-an-enum.kav](enum-helpers-need-an-enum.kav) | `all`, `parse` and `from` are for unions whose variants carry no fields. | compile error |
| 2.7 | [no-null.kav](no-null.kav) | There is no null; only an optional type can hold none. | compile error |
| 2.7 | [optional-chaining-and-default.kav](optional-chaining-and-default.kav) | ?. reads through an optional, ?? gives a default, if let unwraps. | runs |
| 2.7 | [a-value-fills-an-optional-inside-a-type.kav](a-value-fills-an-optional-inside-a-type.kav) | A `T` can be used wherever a `T?` is expected, also as a lambda's result and as the value of a `Shared<T?>`. | runs |
| 2.7 | [unwrap-none-panics.kav](unwrap-none-panics.kav) | .unwrap() on none panics. | panic |
| 2.7 | [unwrap-gives-the-value-of-every-kind.kav](unwrap-gives-the-value-of-every-kind.kav) | `.unwrap()`, `is_some()` and `is_none()` work on an optional of every kind of value. | runs |
| 2.7 | [unwrap-outside-tests-warns.kav](unwrap-outside-tests-warns.kav) | `.unwrap()` outside `test` blocks and `example` clauses gets a warning; `?? panic("…")` says why and gets none. | warning |
| 2.7 | [unwrap-in-a-test-does-not-warn.kav](unwrap-in-a-test-does-not-warn.kav) | `.unwrap()` in a `test` block or an `example` clause gets no warning. | test pass |
| 2.8 | [interfaces-are-explicit.kav](interfaces-are-explicit.kav) | A type implements an interface only when it says so. | compile error |
| 2.8 | [interfaces-need-every-method.kav](interfaces-need-every-method.kav) | A type that says it implements an interface has each of its methods. | compile error |
| 2.8, 3.9 | [interface-methods-keep-inout.kav](interface-methods-keep-inout.kav) | An implementing method marks the same parameters `inout`. | compile error |
| 2.8 | [interface-methods-keep-types.kav](interface-methods-keep-types.kav) | An implementing method has the interface's types, with its type arguments put in. | compile error |
| 2.8 | [interfaces.kav](interfaces.kav) | A type implements an interface with `type T: I` or `extend T: I`. | runs |
| 2.8 | [built-in-types-implement-with-extend.kav](built-in-types-implement-with-extend.kav) | `extend` makes a built-in type implement an interface, so it meets that bound. | runs |
| 2.8, 2.9 | [interface-bounds-check-built-in-types.kav](interface-bounds-check-built-in-types.kav) | A built-in type meets an interface bound only when an `extend` says it implements the interface. | compile error |
| 2.9 | [generics.kav](generics.kav) | Functions and types can be generic. | runs |
| 2.9, 2.5 | [methods-have-type-parameters.kav](methods-have-type-parameters.kav) | A method may have type parameters of its own, like a function, and each call fills them in. | runs |
| 2.9, 3.3 | [lambdas-keep-type-parameters.kav](lambdas-keep-type-parameters.kav) | A lambda made inside a generic function keeps that call's type arguments wherever it is called later. | runs |
| 2.9, 2.12 | [generic-methods-know-their-type-arguments.kav](generic-methods-know-their-type-arguments.kav) | A generic method runs with the type arguments of its call, written or inferred, as a generic function does. | runs |
| 2.9, 3.4 | [type-arguments-come-through-function-arguments.kav](type-arguments-come-through-function-arguments.kav) | A type parameter shown only by a function-typed argument is found from that function's type. | runs |
| 2.9 | [generic-types-have-methods.kav](generic-types-have-methods.kav) | Methods of a generic type work for every type argument. | runs |
| 2.10 | [parameter-types-required.kav](parameter-types-required.kav) | Function parameters must have written types. | compile error |
| 2.11 | [secret-cannot-be-printed.kav](secret-cannot-be-printed.kav) | A secret value must not flow into print. | compile error |
| 2.1 | [numbers-read-from-text-at-their-limits.kav](numbers-read-from-text-at-their-limits.kav) | Reading a number from text takes every value its type holds, trims spaces, and gives an error for anything else. | runs |
| 2.12 | [json-reads-never-panic.kav](json-reads-never-panic.kav) | Reading a `Json` never panics: a missing key or index gives `Null`, and the methods give `none` when the value has another kind. | runs |
| 2.12 | [json-reads-find-each-objects-own-field.kav](json-reads-find-each-objects-own-field.kav) | `j["key"]` gives each object's own field, wherever the key is in the object and whatever made it, or `Null` when it has none. | runs |
| 2.12 | [json-parsed-values-act-like-made-ones.kav](json-parsed-values-act-like-made-ones.kav) | A `Json` from `json.parse` reads, matches, compares, prints and writes exactly like the same value made in the program. | runs |
| 2.12 | [json-literals-build-values.kav](json-literals-build-values.kav) | Where a `Json` is expected, literals become `Json` values, and values inside them are encoded. | runs |
| 2.12 | [json-parse-and-stringify.kav](json-parse-and-stringify.kav) | `json.parse` reads JSON text into a `Json`; `json.stringify` writes it back, compact or indented. | runs |
| 2.12 | [json-decode-in-generic-functions.kav](json-decode-in-generic-functions.kav) | Inside a generic function, `json.decode<T>` decodes into the type its call gave `T`, written or inferred. | runs |
| 2.12 | [json-dates-are-iso-text.kav](json-dates-are-iso-text.kav) | `std.time`'s `Date`, `Time` and `DateTime` are ISO 8601 text in JSON, a `DateTime` in UTC; offsets are turned to UTC, anything else is a problem. | runs |
| 2.12 | [json-schema-describes-types.kav](json-schema-describes-types.kav) | `json.schema<T>()` gives the JSON Schema of `T`, fields in declared order, also for a generic function's type argument. | runs |
| 2.12 | [json-schema-carries-doc-comments.kav](json-schema-carries-doc-comments.kav) | A type's doc comment becomes its schema's `"description"`, and so does a field's, for records and unions alike. | runs |
| 2.12 | [json-limits-stop-hostile-text.kav](json-limits-stop-hostile-text.kav) | Parsing stops with an error, not a crash, on text nested too deep, keys given twice and bad syntax; a huge number is read by its digits. | runs |
| 2.12 | [json-numbers-keep-their-digits.kav](json-numbers-keep-their-digits.kav) | A JSON number keeps its digits: big integers and exact decimals survive parsing, decoding and writing. | runs |
| 2.12 | [json-stringify-writes-every-kind-of-value.kav](json-stringify-writes-every-kind-of-value.kav) | `json.stringify` writes every kind of value, compact or indented, as the same text in the interpreter and native code. | runs |
| 2.12 | [json-comments-only-when-asked.kav](json-comments-only-when-asked.kav) | `json.parse(text, comments: true)` takes comments and trailing commas; plain `json.parse` refuses them. | runs |
| 2.12 | [json-of-encodes-typed-values.kav](json-of-encodes-typed-values.kav) | `json.of` encodes records as objects, unions by variant name, optionals as `null`, and maps with text keys as objects. | runs |
| 2.12 | [json-of-an-empty-list.kav](json-of-an-empty-list.kav) | `json.of([])` encodes an empty list literal as `[]`. | runs |
| 2.12, 2.11 | [json-secret-fields-are-not-encoded.kav](json-secret-fields-are-not-encoded.kav) | A record with a `secret` field can't be encoded as JSON. | compile error |
| 2.12 | [json-decode-into-records.kav](json-decode-into-records.kav) | `json.decode<T>` reads JSON straight into a typed value: missing fields take their default or `none`, and unions are read by variant name. | runs |
| 2.12 | [json-decode-problems-say-where.kav](json-decode-problems-say-where.kav) | When JSON doesn't fit the type, decoding fails with `Invalid`, whose problems say where and what was expected; numbers must fit their type. | runs |
| 2.12 | [json-decode-lists-every-problem.kav](json-decode-lists-every-problem.kav) | Decoding goes on after a problem, so `Invalid` lists every problem in the document, in its order. | runs |
| 2.12 | [json-decode-refuses-unknown-fields.kav](json-decode-refuses-unknown-fields.kav) | Decoding refuses a field the type doesn't have, at its path; `unknown: JsonUnknown.Ignore` skips them. | runs |
| 2.12, 7.3 | [json-decode-checks-invariants.kav](json-decode-checks-invariants.kav) | A decoded value is checked against its types' invariants; a broken one is a problem with the record's path, not a panic. | runs |
| 2.12 | [json-decode-keeps-json-fields.kav](json-decode-keeps-json-fields.kav) | Decoding reads a field of type `Json` as written, whatever it holds. | runs |
| 2.13 | [bytes-hold-raw-data.kav](bytes-hold-raw-data.kav) | `Bytes` holds raw data: from text or `[UInt8]`, indexed, sliced, joined, compared, back to text. | runs |
| 2.13 | [bytes-index-out-of-range-panics.kav](bytes-index-out-of-range-panics.kav) | `b[i]` out of range panics. | panic |
| 2.13 | [bytes-crc32c-checksums.kav](bytes-crc32c-checksums.kav) | `b.crc32c()` matches the published CRC32C check vectors. | runs |
| 2.13 | [bytes-xxhash64-hashes.kav](bytes-xxhash64-hashes.kav) | `b.xxhash64()` is the XXH64 hash (seed 0), a `UInt64`, matching the reference implementation. | runs |
| 2.13 | [bytes-read-as-numbers.kav](bytes-read-as-numbers.kav) | `b.as_list<T>()` reads little-endian numbers, or gives `none` for a length that isn't a whole number of them. | runs |
| 2.13 | [as-list-needs-a-fixed-size-number.kav](as-list-needs-a-fixed-size-number.kav) | `b.as_list<T>()` takes only a number of fixed size. | compile error |
| 1.6 | [a-keyword-cant-name-a-field.kav](a-keyword-cant-name-a-field.kav) | A keyword can't name a field, which could never be read. | compile error |
| 2.13 | [bytes-know-when-empty.kav](bytes-know-when-empty.kav) | `b.is_empty()` gives whether `Bytes` holds no bytes. | runs |
| 2.13 | [bytes-index-of-finds-a-part.kav](bytes-index-of-finds-a-part.kav) | `b.index_of(part)` gives where `part` first appears in `b`, or `none`; an empty `part` is at 0. | runs |
| 2.12, 2.6 | [variants-named-like-jsons-are-written-with-their-type.kav](variants-named-like-jsons-are-written-with-their-type.kav) | A program's variant named like one of `Json`'s: each is written with its own union's name. | runs |
| 2.14 | [anonymous-records-need-no-type.kav](anonymous-records-need-no-type.kav) | A record value or type is written inline with no declaration; matching is structural, and `type X = (…)` aliases the shape. | runs |
| 2.14, 2.7 | [anonymous-record-fields-take-optionals.kav](anonymous-record-fields-take-optionals.kav) | Where an anonymous record of known fields is expected, a `T` fills a `T?` field. | runs |
| 2.15 | [interface-values-dispatch-at-run-time.kav](interface-values-dispatch-at-run-time.kav) | An interface is also a value type: any implementing type fits, and method calls dispatch at run time. | runs |
| 2.15 | [interface-methods-can-fail.kav](interface-methods-can-fail.kav) | A method that can fail fails the same way through an interface value, with `try` or `catch`. | runs |
| 2.15 | [generic-interface-values.kav](generic-interface-values.kav) | A generic interface is a value type with its type arguments; its methods give and take them. | runs |
| 2.15 | [generic-interface-arguments-must-match.kav](generic-interface-arguments-must-match.kav) | A type implementing `Getter<String>` doesn't fit a `Getter<Int>`. | compile error |
| 2.15 | [generic-types-as-interface-values.kav](generic-types-as-interface-values.kav) | A value of a generic type fits an interface it implements, whatever its type arguments. | runs |
| 2.15 | [built-in-types-as-interface-values.kav](built-in-types-as-interface-values.kav) | A built-in type that implements an interface fits an interface value, a number literal too. | runs |
| 2.12 | [json-describe-describes-a-function.kav](json-describe-describes-a-function.kav) | `json.describe(f)` gives a function's name, doc comment, parameters, result and error schemas, capabilities and examples. | runs |
