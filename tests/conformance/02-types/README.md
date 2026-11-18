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
| 2.2 | [an-empty-sum-is-zero-of-its-type.kav](an-empty-sum-is-zero-of-its-type.kav) | An empty list's `sum` is the zero of the lambda's result type: `0`, or `0.0` for `Float` and `Float32`. | runs |
| 2.2 | [list-and-map-access.kav](list-and-map-access.kav) | list.get(i) and map[key] return optionals. | runs |
| 2.2 | [a-list-mapped-into-itself-keeps-its-copies.kav](a-list-mapped-into-itself-keeps-its-copies.kav) | `xs = xs.map(f)` gives `xs` the new items and leaves every copy of the old list as it was. | runs |
| 2.2 | [maps-keep-insertion-order.kav](maps-keep-insertion-order.kav) | Maps keep insertion order when iterated. | runs |
| 2.2 | [map-keys-read-from-lists-of-text.kav](map-keys-read-from-lists-of-text.kav) | A map with text keys finds a key read from a list of text, short or long. | runs |
| 2.16 | [matmul-calls-the-types-own-method.kav](matmul-calls-the-types-own-method.kav) | `a @ b` calls `matmul` of a type with `MatMul`; `@=` too. | runs |
| 2.17 | [matmul-is-not-for-numbers.kav](matmul-is-not-for-numbers.kav) | `@` on numbers is a compile error that says to use `*`. | compile error |
| 2.17 | [matrices-multiply-and-work-item-by-item.kav](matrices-multiply-and-work-item-by-item.kav) | `a @ b`, `+ - *` and `-` item by item, transposes, sums, slices, rows and columns, for floats and whole numbers. | runs |
| 2.17 | [matrix-products-give-the-same-bits.kav](matrix-products-give-the-same-bits.kav) | `a @ b` gives the bits of a loop written by hand; `fast: true` comes close. | runs |
| 2.17, 2.9 | [matrix-holds-only-numbers.kav](matrix-holds-only-numbers.kav) | A `Matrix<T>` holds numbers: a matrix of text is a compile error. | compile error |
| 2.17 | [matrix-sizes-must-match.kav](matrix-sizes-must-match.kav) | A product whose inner sizes differ breaks `matmul`'s `require`. | panic |
| 2.12 | [json-of-a-generic-is-checked-where-it-is-used.kav](json-of-a-generic-is-checked-where-it-is-used.kav) | `json.of` of a type parameter is a compile error at the generic call that gives it a record with a `secret` field. | compile error |
| 2.12 | [json-of-a-generic-method-is-checked-where-it-is-used.kav](json-of-a-generic-method-is-checked-where-it-is-used.kav) | A generic method writing its type parameter as JSON is checked at each call. | compile error |
| 2.12 | [json-schema-marks-secret-fields.kav](json-schema-marks-secret-fields.kav) | `json.schema<T>()` marks a `secret` field `"writeOnly": true`. | runs |
| 2.2 | [push-needs-a-var.kav](push-needs-a-var.kav) | Methods that change a collection (`push`, `remove`) need a `let`. | compile error |
| 2.2 | [lists-hold-any-text.kav](lists-hold-any-text.kav) | A list of text gives back every item as it was put in, short or long, ASCII or not, and copies of the list are independent. | runs |
| 2.2 | [capacity-holds-at-most-n.kav](capacity-holds-at-most-n.kav) | `List.capacity(n)`, `Map.capacity(n)` and `Set.capacity(n)` hold at most `n` entries; `is_full()`; first entries; below 0 counts as 0. | runs |
| 2.2 | [capacity-full-list-panics.kav](capacity-full-list-panics.kav) | Adding to a full list made by `List.capacity(n)` panics. | panic |
| 2.2 | [capacity-full-map-panics.kav](capacity-full-map-panics.kav) | A new key in a full map panics; changing a key it holds does not. | panic |
| 2.2 | [capacity-full-set-panics.kav](capacity-full-set-panics.kav) | A new item in a full set panics; adding one it holds does not. | panic |
| 2.2 | [capacity-too-many-first-entries-panics.kav](capacity-too-many-first-entries-panics.kav) | `capacity(n, first)` with more first entries than `n` panics. | panic |
| 2.2 | [capacity-mixed-and-nested.kav](capacity-mixed-and-nested.kav) | Items of different kinds through a union, collections with limits inside others; a copy keeps the limit, `xs + ys` has none. | runs |
| 2.2 | [capacity-copy-keeps-limit-panics.kav](capacity-copy-keeps-limit-panics.kav) | A copy keeps the limit. | panic |
| 2.2 | [capacity-kept-by-plus-equals-panics.kav](capacity-kept-by-plus-equals-panics.kav) | `xs += ys` keeps the limit, in a variable or a field. | panic |
| 2.2 | [ring-keeps-the-last-n.kav](ring-keeps-the-last-n.kav) | `Ring.capacity(n)` keeps the last `n` items pushed, oldest first; first items beyond `n` keep the last `n`. | runs |
| 2.2 | [ring-holds-records.kav](ring-holds-records.kav) | A ring holds records and text, and may be a record field. | runs |
| 2.2 | [ring-is-not-a-list.kav](ring-is-not-a-list.kav) | `Ring<T>` is not a list: a function that takes `[T]` is not given one. | compile error |
| 2.2 | [small-signed-lists-keep-their-sign.kav](small-signed-lists-keep-their-sign.kav) | A list of `Int8` holds negative numbers, which print, compare and sort as numbers. | runs |
| 2.2 | [small-number-lists-keep-every-value.kav](small-number-lists-keep-every-value.kav) | A list of 16- or 32-bit numbers holds every value of its item type, which it gives back, sorts and copies unchanged. | runs |
| 2.2 | [plus-joins-two-lists.kav](plus-joins-two-lists.kav) | `xs + ys` is a new list, the items of `xs` then those of `ys`; neither changes. | runs |
| 2.2 | [plus-equals-adds-to-a-list.kav](plus-equals-adds-to-a-list.kav) | `xs += ys` adds the items of `ys` to the end of `xs`; a copy made before doesn't change. | runs |
| 2.2 | [plus-equals-on-a-const-list-is-an-error.kav](plus-equals-on-a-const-list-is-an-error.kav) | `xs += ys` changes `xs`, so `xs` must be a `let`. | compile error |
| 2.2 | [ranges-have-list-methods.kav](ranges-have-list-methods.kav) | A range has the list methods that read it, giving what the list of its numbers would; `to_list()` gives that list. | runs |
| 2.2 | [a-float-key-has-one-place.kav](a-float-key-has-one-place.kav) | Any value can be a key; 0.0 and -0.0 are one key, and NaN equals NaN as a key or set item |
| 2.2 | [a-map-slot-takes-a-value-not-an-optional.kav](a-map-slot-takes-a-value-not-an-optional.kav) | `m[k] = v` takes the map's value type: an optional map read must be unwrapped first. | compile error |
| 2.2 | [enumerate-gives-index-and-item.kav](enumerate-gives-index-and-item.kav) | `xs.enumerate()` gives `(index, item)` pairs, as a value too. | runs |
| 2.3 | [tuples.kav](tuples.kav) | Tuples group values and can be destructured. | runs |
| 2.4 | [record-defaults.kav](record-defaults.kav) | Records are created with named arguments; fields with defaults may be left out. | runs |
| 2.4 | [record-needs-every-field.kav](record-needs-every-field.kav) | A record field without a default must be given. | compile error |
| 2.4 | [record-needs-named-arguments.kav](record-needs-named-arguments.kav) | Records are created with named arguments, not positional ones. | compile error |
| 2.4 | [records-are-values.kav](records-are-values.kav) | Changing a field through a var changes only that variable's value. | runs |
| 2.4 | [lists-of-records-are-values.kav](lists-of-records-are-values.kav) | A list of records holds values: changing a copy, or an item taken out, never changes the original. | runs |
| 2.4 | [a-record-made-in-a-loop-is-new-each-time.kav](a-record-made-in-a-loop-is-new-each-time.kav) | A record made inside a loop holds that pass's field values, so reading one field gives this pass's value. | runs |
| 2.4 | [with-makes-a-copy.kav](with-makes-a-copy.kav) | `with` creates a copy with some fields changed. | runs |
| 2.4 | [record-fields-that-may-be-left-out.kav](record-fields-that-may-be-left-out.kav) | `summary?: String` is short for `summary: String? = none`: a record made or decoded without it holds `none`, and `ask` doesn't require it. | runs |
| 2.4 | [optional-field-takes-one-question-mark.kav](optional-field-takes-one-question-mark.kav) | A field that may be left out takes its `?` once: `summary?: String?` is an error. | compile error |
| 2.4 | [optional-field-has-no-other-default.kav](optional-field-has-no-other-default.kav) | A field that may be left out is `none` when left out, so it takes no other default: `count?: Int = 3` is an error. | compile error |
| 2.4, 2.9 | [generic-fields-take-defaults.kav](generic-fields-take-defaults.kav) | A field whose type uses the record's type parameter can have a default. | runs |
| 2.5 | [extend-adds-methods.kav](extend-adds-methods.kav) | `extend` adds methods to an existing type. | runs |
| 2.5 | [generic-types-are-extended-elsewhere/](generic-types-are-extended-elsewhere/src/main.kav) | `extend<S> Box<S>` adds methods to a generic type from another module, `S` standing for the type's own parameter. | runs |
| 2.5 | [collections-take-added-methods.kav](collections-take-added-methods.kav) | `extend<T> [T]`, `[K: V]`, `Set<T>` and `Ring<T>` add methods for every item type; a range has a list's. | runs |
| 2.5 | [collection-extend-names-its-item-types.kav](collection-extend-names-its-item-types.kav) | An extend of a collection names each item type: `extend [Int]` is an error. | compile error |
| 2.5 | [added-methods-keep-built-in-names.kav](added-methods-keep-built-in-names.kav) | A method added to a collection can't take the name of one it has. | compile error |
| 2.5 | [methods.kav](methods.kav) | Methods take self; functions without self are called on the type. | runs |
| 2.9 | [generic-type-methods-know-their-type-arguments.kav](generic-type-methods-know-their-type-arguments.kav) | Inside a generic type's methods, its type parameter is the type the value was made with, so `json.decode<I>` decodes into it. | runs |
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
| 2.7 | [optionals-do-not-nest.kav](optionals-do-not-nest.kav) | Optionals don't nest: `?.` to an optional field gives a `T?`, not a `T??`. | runs |
| 2.7 | [question-dot-calls-methods.kav](question-dot-calls-methods.kav) | `?.` calls methods too, giving `none` when the left side is; a `none` branch makes an `if` optional |
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
| 2.9 | [number-bounds-do-arithmetic.kav](number-bounds-do-arithmetic.kav) | `T: Number` and `T: Floating` let generic code do arithmetic, use literals and `T(x)`, and call `std.math` on `T`; each type it is used with gets its own copy. | runs |
| 2.9 | [a-number-bound-takes-only-numbers.kav](a-number-bound-takes-only-numbers.kav) | A type given for a parameter with a number bound must be a number type; for `Floating`, `Float` or `Float32`. | compile error |
| 2.9 | [a-fraction-needs-floating.kav](a-fraction-needs-floating.kav) | A literal with a fraction is a `T` only when `T` is `Floating`. | compile error |
| 2.9 | [a-method-can-add-a-bound.kav](a-method-can-add-a-bound.kav) | A method may add a bound to its type's parameter with `where`; a bare literal becomes a `Float` there. | runs |
| 2.9 | [a-method-bound-must-be-met.kav](a-method-bound-must-be-met.kav) | Calling a method whose `where` bound isn't met is a compile error. | compile error |
| 2.10 | [parameter-types-required.kav](parameter-types-required.kav) | Function parameters must have written types. | compile error |
| 2.11 | [secret-cannot-be-printed.kav](secret-cannot-be-printed.kav) | A secret value must not flow into print. | compile error |
| 2.1 | [numbers-read-from-text-at-their-limits.kav](numbers-read-from-text-at-their-limits.kav) | Reading a number from text takes every value its type holds, trims spaces, and gives an error for anything else. | runs |
| 2.1 | [floats-give-the-same-bits-everywhere.kav](floats-give-the-same-bits-everywhere.kav) | A multiply and an add are never fused, and `std.math`'s functions are Kavach's own code, so floats give the same bits on every machine |
| 2.12 | [json-reads-never-panic.kav](json-reads-never-panic.kav) | Reading a `Json` never panics: a missing key or index gives `Null`, and the methods give `none` when the value has another kind. | runs |
| 2.12 | [json-writes-bytes-as-base64.kav](json-writes-bytes-as-base64.kav) | `Bytes` in JSON are base64 text: written standard and padded, read in either alphabet, padded or not, bare or in a field; other text expects "base64 text"; the schema says so. | runs |
| 2.12 | [json-reads-find-each-objects-own-field.kav](json-reads-find-each-objects-own-field.kav) | `j["key"]` gives each object's own field, wherever the key is in the object and whatever made it, or `Null` when it has none. | runs |
| 2.12 | [json-parsed-values-act-like-made-ones.kav](json-parsed-values-act-like-made-ones.kav) | A `Json` from `json.parse` reads, matches, compares, prints and writes exactly like the same value made in the program. | runs |
| 2.12 | [json-literals-build-values.kav](json-literals-build-values.kav) | Where a `Json` is expected, literals become `Json` values, and values inside them are encoded. | runs |
| 2.12 | [json-parse-and-stringify.kav](json-parse-and-stringify.kav) | `json.parse` reads JSON text into a `Json`; `json.stringify` writes it back, compact or indented. | runs |
| 2.12 | [json-decode-in-generic-functions.kav](json-decode-in-generic-functions.kav) | Inside a generic function, `json.decode<T>` decodes into the type its call gave `T`, written or inferred. | runs |
| 2.12 | [json-dates-are-iso-text.kav](json-dates-are-iso-text.kav) | `std.time`'s `Date`, `Time` and `DateTime` are ISO 8601 text in JSON, a `DateTime` in UTC; offsets are turned to UTC, anything else is a problem. | runs |
| 2.12 | [json-schema-describes-types.kav](json-schema-describes-types.kav) | `json.schema<T>()` gives the JSON Schema of `T`, fields in declared order, also for a generic function's type argument. | runs |
| 2.12 | [json-schema-carries-doc-comments.kav](json-schema-carries-doc-comments.kav) | A type's doc comment becomes its schema's `"description"`, and so does a field's, for records and unions alike; a field whose type has one too gets one description, the field's first. | runs |
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
| 2.12 | [json-decode-problems-in-list-items.kav](json-decode-problems-in-list-items.kav) | A list item that doesn't fit is a problem at its index, also in a list of records stored flat. | runs |
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
| 2.13 | [bytes-read-numbers-at-a-place.kav](bytes-read-numbers-at-a-place.kav) | `b.read<T>(at, big_endian: false)` reads the number whose bytes start at `at`, in either order, or gives `none` when they don't all fit. | runs |
| 2.13 | [bytes-of-gives-a-numbers-bytes.kav](bytes-of-gives-a-numbers-bytes.kav) | `Bytes.of<T>(n, big_endian: false)` gives a number's bytes, in either order. | runs |
| 2.13 | [read-needs-a-fixed-size-number.kav](read-needs-a-fixed-size-number.kav) | `b.read<T>(at)` and `Bytes.of<T>(n)` take only a number of fixed size. | compile error |
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
| 2.12 | [json-describe-takes-function-values.kav](json-describe-takes-function-values.kav) | `json.describe(value)` on a function value gives `FnSchema?`: a named function's description, `none` for a lambda. | runs |
| 2.12 | [generic-records-have-a-json-schema.kav](generic-records-have-a-json-schema.kav) | A generic record's instance has a JSON Schema, with its type arguments in its fields' places. | runs |
| 2.16 | [operators-call-the-types-own-methods.kav](operators-call-the-types-own-methods.kav) | A type that implements `Add`, `Neg` or `Mul<Float>` gets `+`, unary `-` or `* Float`, which call its methods; `x op= v` is `x = x op v`. | runs |
| 2.16 | [an-operator-can-give-another-type.kav](an-operator-can-give-another-type.kav) | `Mul<Meters, Area>` takes a `Meters` and gives an `Area`. | runs |
| 2.16 | [comparable-gives-ordering-operators.kav](comparable-gives-ordering-operators.kav) | A type that implements `Comparable` gets `<`, `<=`, `>` and `>=` from its `compare`. | runs |
| 2.16 | [operator-methods-are-pure.kav](operator-methods-are-pure.kav) | An operator method can't use capabilities or fail. | compile error |
| 2.16 | [no-operator-without-its-interface.kav](no-operator-without-its-interface.kav) | A program's own type has no operator unless it implements the operator's interface. | compile error |
| 2.16 | [own-type-goes-on-the-left.kav](own-type-goes-on-the-left.kav) | A program's own type is on the left of its operator: `2.0 * v` is an error. | compile error |
| 2.16 | [built-in-types-keep-their-operators.kav](built-in-types-keep-their-operators.kav) | A built-in type can't implement an operator interface. | compile error |
| 2.16 | [operators-in-generic-code.kav](operators-in-generic-code.kav) | A type parameter bounded by an operator interface may use the operator; numbers meet Add, Sub, Mul, Div, Rem, Neg and Comparable, String meets Comparable, and a program's own type its own. | runs |
| 2.16 | [an-operator-bound-needs-the-operator.kav](an-operator-bound-needs-the-operator.kav) | A type given for a `T` with operator bounds must have those operators. | compile error |
| 2.18 | [text-views-read-without-copying.kav](text-views-read-without-copying.kav) | `s.view(…)` gives a `StrView` whose `trim`, `split` and `lines` give views; views compare, order and convert by their text. | runs |
| 2.18 | [text-builders-change-in-place.kav](text-builders-change-in-place.kav) | A `TextBuilder` changes in place at clamped character positions; a view or copy of it keeps the text it had. | runs |
| 2.18 | [text-builder-start-after-end-panics.kav](text-builder-start-after-end-panics.kav) | A builder's `remove` or `replace` from a start after its end is a panic. | panic |
| 2.18 | [text-builder-changes-need-let.kav](text-builder-changes-need-let.kav) | Changing a builder held by a `const` is an error. | compile error |
| 2.12, 1.8 | [json-objects-are-equal-whatever-the-key-order.kav](json-objects-are-equal-whatever-the-key-order.kav) | Two `Json` values are equal when they are the same variant with equal contents; objects compare like maps, so key order doesn't count, but array order does. | runs |
| 2.1 | [a-surrogate-is-not-a-char.kav](a-surrogate-is-not-a-char.kav) | a whole number that isn't a character's code point (here a surrogate) panics when converted to `Char`. | panic |
| 2.1 | [a-negative-number-is-not-a-char.kav](a-negative-number-is-not-a-char.kav) | A negative number isn't a character, so converting it to `Char` panics. | panic |
| 2.1 | [a-code-point-past-10ffff-is-not-a-char.kav](a-code-point-past-10ffff-is-not-a-char.kav) | A whole number above `0x10FFFF` isn't a character, so converting it to `Char` panics. | panic |
| 2.1, 1.8, 5.6 | [nan-comparisons-are-false.kav](nan-comparisons-are-false.kav) | an order comparison with `NaN` is false, not a panic: `<`, `<=`, `>` and `>=` on floats never stop the program. | runs |
| 2.1 | [a-float32-literal-must-fit.kav](a-float32-literal-must-fit.kav) | A number literal must fit its type: `1e300` doesn't fit `Float32`. | compile error |
| 2.9 | [a-type-argument-nothing-fills-is-an-error.kav](a-type-argument-nothing-fills-is-an-error.kav) | A type argument no field holds and nothing else fixes must be written: `Box(raw: text)` alone is a compile error. | compile error |
| 1.11 | [a-type-implements-display.kav](a-type-implements-display.kav) | A type may implement the prelude's `Display` interface, even in a program that uses `std.system`, whose screen type has its own name. | runs |
| 14.2 | [bigint-has-abs-min-max-and-clamp.kav](bigint-has-abs-min-max-and-clamp.kav) | `abs`, `min`, `max` and `clamp` work on `BigInt` as on the other whole numbers. | runs |
| 2.1, 1.8 | [bit-operators-are-for-integers-only.kav](bit-operators-are-for-integers-only.kav) | The bit operators work on integers only, so `&` on a `Float` is a compile error. | compile error |
| 2.1, 1.8 | [chars-have-no-arithmetic.kav](chars-have-no-arithmetic.kav) | Arithmetic is for numbers: a `Char` has no `+`; its code point is `Int(c)`. | compile error |
| 2.1, 1.8 | [nan-is-not-equal-to-itself.kav](nan-is-not-equal-to-itself.kav) | `NaN` is equal to nothing, itself included, and `-0.0` equals `0.0`; `==` on floats worked out while running warns. | warning |
| 2.1, 1.8 | [plus-joins-text-only-with-text.kav](plus-joins-text-only-with-text.kav) | `+` joins two `String`s; a number is not turned into text by itself. | compile error |
| 2.1, 1.8 | [signed-sized-integers-do-arithmetic.kav](signed-sized-integers-do-arithmetic.kav) | `Int8`, `Int16`, `Int32` and `Int` take `+ - * / % **`; `/` truncates toward zero, `%` keeps the sign of the left side, and each holds its own range. | runs |
| 2.1, 1.8, 5.6 | [uint16-power-overflow-panics.kav](uint16-power-overflow-panics.kav) | A power past `UInt16`'s largest value, 65535, panics. | panic |
| 2.1, 1.8 | [unsigned-integers-do-arithmetic.kav](unsigned-integers-do-arithmetic.kav) | `UInt8`, `UInt16`, `UInt32` and `UInt64` take `+ - * / % **` up to their largest value, and never go below 0. | runs |
| 1.11 | [floats-print-with-a-point-or-an-exponent.kav](floats-print-with-a-point-or-an-exponent.kav) | a `Float` prints in the shortest form that reads back exactly, always with a `.` or an exponent, from the smallest to the largest. | runs |
| 2.1, 5.6 | [a-bigint-too-big-for-int-panics.kav](a-bigint-too-big-for-int-panics.kav) | Converting a `BigInt` that doesn't fit `Int` panics. | panic |
| 2.1, 1.8 | [a-char-is-not-a-string.kav](a-char-is-not-a-string.kav) | A `Char` is one character, not text: it can't be compared with a `String` or joined to one with `+`. | compile error |
| 2.1, 1.7 | [a-char-literal-holds-one-character.kav](a-char-literal-holds-one-character.kav) | A `Char` literal holds exactly one character; two are text and need double quotes. | compile error |
| 2.1, 5.6 | [a-float-too-big-for-int-panics.kav](a-float-too-big-for-int-panics.kav) | a float past `Int`'s range doesn't fit it, so converting it panics instead of saturating. | panic |
| 2.1, 1.7 | [a-literal-takes-the-type-of-the-other-operand.kav](a-literal-takes-the-type-of-the-other-operand.kav) | A literal next to a sized number takes that number's type, so it must fit it. | compile error |
| 2.1 | [a-negative-literal-does-not-fit-an-unsigned-type.kav](a-negative-literal-does-not-fit-an-unsigned-type.kav) | A negative constant doesn't fit an unsigned type, so writing one there is a compile error. | compile error |
| 2.1, 5.6 | [a-negative-number-to-unsigned-panics.kav](a-negative-number-to-unsigned-panics.kav) | a negative number doesn't fit an unsigned type, so converting it panics. | panic |
| 2.1, 5.6 | [a-number-that-does-not-fit-its-new-type-panics.kav](a-number-that-does-not-fit-its-new-type-panics.kav) | converting a number to a type it doesn't fit panics. | panic |
| 2.1, 2.18 | [a-string-cannot-change-in-place.kav](a-string-cannot-change-in-place.kav) | A `String` never changes: it has no methods that change it in place; a `TextBuilder` is for that. | compile error |
| 2.1 | [a-typed-constant-past-int-is-an-error.kav](a-typed-constant-past-int-is-an-error.kav) | A constant worked out from literals that doesn't fit the type written for it is a compile error, not a `BigInt`. | compile error |
| 2.1, 1.11 | [any-value-converts-to-its-printed-text.kav](any-value-converts-to-its-printed-text.kav) | `String(value)` gives the value's text, as `print` shows it, for every kind of value. | runs |
| 2.1 | [bigint-and-int-do-not-mix.kav](bigint-and-int-do-not-mix.kav) | A `BigInt` and an `Int` can't be added: the `Int` must be converted with `BigInt(n)`. | compile error |
| 2.1 | [bigint-holds-huge-values.kav](bigint-holds-huge-values.kav) | A `BigInt` holds a whole number of any size, negative too, and keeps every digit through arithmetic and comparison. | runs |
| 2.1 | [bigint-holds-very-large-values.kav](bigint-holds-very-large-values.kav) | `BigInt` never overflows: a product of the largest `Int`s, or `2 ** 200`, keeps every digit; `/` truncates and `%` keeps the left side's sign, as for `Int`. | runs |
| 2.1, 2.13 | [bytes-cannot-change.kav](bytes-cannot-change.kav) | `Bytes` are immutable: a byte can't be set, even in a `let`. | compile error |
| 2.1, 1.7, 1.8 | [chars-hold-any-unicode-character.kav](chars-hold-any-unicode-character.kav) | A `Char` holds any one Unicode character, written plainly or with `\u{…}`; it converts to and from its code point and orders by it. | runs |
| 2.1 | [decimal-and-float-do-not-mix.kav](decimal-and-float-do-not-mix.kav) | A `Decimal` and a `Float` can't be added: there are no implicit number conversions. | compile error |
| 2.1 | [decimal-arithmetic-is-exact-while-running.kav](decimal-arithmetic-is-exact-while-running.kav) | `Decimal` arithmetic is exact base 10 while the program runs too: 0.1 + 0.2 is 0.3, unlike `Float`. | runs |
| 2.1 | [float-and-float32-do-not-mix.kav](float-and-float32-do-not-mix.kav) | Arithmetic between `Float` and `Float32` is a compile error: no implicit conversions. | compile error |
| 2.1 | [float-to-whole-number-cuts-the-fraction.kav](float-to-whole-number-cuts-the-fraction.kav) | a float or `Decimal` becomes a whole number by cutting off the fraction, toward zero. | runs |
| 2.1, 1.8 | [int-and-float-do-not-compare.kav](int-and-float-do-not-compare.kav) | An `Int` and a `Float` can't be compared with `==` or `<` without a conversion. | compile error |
| 2.1 | [int-and-float-do-not-mix.kav](int-and-float-do-not-mix.kav) | An `Int` and a `Float` can't be added: there are no implicit number conversions. | compile error |
| 2.1, 5.6 | [int-overflow-at-its-minimum-panics.kav](int-overflow-at-its-minimum-panics.kav) | Going below `Int`'s smallest value panics. | panic |
| 2.1, 5.6 | [int16-overflow-at-its-maximum-panics.kav](int16-overflow-at-its-maximum-panics.kav) | Going above `Int16`'s largest value, 32767, panics. | panic |
| 2.1, 5.6 | [int32-multiply-overflow-panics.kav](int32-multiply-overflow-panics.kav) | A product past `Int32`'s range panics. | panic |
| 2.1, 5.6 | [int8-overflow-at-its-minimum-panics.kav](int8-overflow-at-its-minimum-panics.kav) | Going below `Int8`'s smallest value, -128, panics. | panic |
| 2.1, 1.11 | [negative-zero-and-infinities.kav](negative-zero-and-infinities.kav) | floats follow IEEE 754: zero has a sign that prints, dividing by a signed zero gives an infinity of that sign, and `Float32` overflows to infinity without a panic. | runs |
| 2.1 | [numbers-convert-between-floats-and-decimal.kav](numbers-convert-between-floats-and-decimal.kav) | Any number converts to any other with `T(value)`: whole numbers to floats and `Decimal`, a `Decimal` to a float, a float to a `Decimal`. | runs |
| 2.1 | [reading-text-as-a-number-needs-try-or-catch.kav](reading-text-as-a-number-needs-try-or-catch.kav) | Converting text that isn't a literal to a number can fail, so it must be handled with `try`, `catch` or `match`. | compile error |
| 2.1 | [sized-integers-do-not-mix.kav](sized-integers-do-not-mix.kav) | Two different sized integers can't be added: there are no implicit number conversions. | compile error |
| 2.1, 1.7 | [sized-integers-hold-their-full-range.kav](sized-integers-hold-their-full-range.kav) | Each sized integer type takes every literal from its smallest to its largest value. | runs |
| 2.1, 1.11 | [text-converts-to-decimal-keeping-its-digits.kav](text-converts-to-decimal-keeping-its-digits.kav) | `Decimal(text)` reads the text exactly, keeping its scale; `BigInt(text)` reads a whole number of any size; other text fails. | runs |
| 2.1 | [text-literal-that-does-not-fit-is-a-compile-error.kav](text-literal-that-does-not-fit-is-a-compile-error.kav) | A literal text read into a sized number is checked while compiling: `Int8("300")` doesn't fit, so it is a compile error. | compile error |
| 2.1 | [text-to-bool-reads-only-true-or-false.kav](text-to-bool-reads-only-true-or-false.kav) | `Bool(text)` reads `true` or `false`, with spaces around allowed; any other text fails. | runs |
| 2.1, 5.6 | [uint32-overflow-at-its-maximum-panics.kav](uint32-overflow-at-its-maximum-panics.kav) | Going above `UInt32`'s largest value, 4294967295, panics. | panic |
| 2.1, 5.6 | [uint64-overflow-at-its-maximum-panics.kav](uint64-overflow-at-its-maximum-panics.kav) | Going above `UInt64`'s largest value panics. | panic |
| 2.1, 5.6 | [uint8-below-zero-panics.kav](uint8-below-zero-panics.kav) | An unsigned number can't go below 0: `UInt8` 0 - 1 panics. | panic |
| 2.1 | [unsigned-and-signed-do-not-compare.kav](unsigned-and-signed-do-not-compare.kav) | Comparing two different number types is a compile error: no implicit conversions. | compile error |
| 2.1 | [whole-numbers-convert-exactly-between-types.kav](whole-numbers-convert-exactly-between-types.kav) | A whole number converts exactly to any whole-number type it fits, `BigInt` included, and back. | runs |
| 2.2 | [a-key-set-again-keeps-its-place.kav](a-key-set-again-keeps-its-place.kav) | Maps and sets keep the order in which items were first added: setting a key again, or adding a value again, does not move it. | runs |
| 2.2 | [a-map-key-has-the-maps-key-type.kav](a-map-key-has-the-maps-key-type.kav) | A map is read and set with keys of its own key type; a key of another type is a compile error. | compile error |
| 2.2 | [a-map-read-is-an-optional.kav](a-map-read-is-an-optional.kav) | Reading a map with `ages[key]` gives an optional (`Int?`), never a plain value, even for a key that is there. | compile error |
| 2.2 | [a-negative-index-panics.kav](a-negative-index-panics.kav) | `items[i]` panics for an index outside the list; a negative index is outside it, not counted from the end. | panic |
| 2.2 | [a-range-has-no-push.kav](a-range-has-no-push.kav) | Methods that change a list (`push`, `pop`) are not a range's. | compile error |
| 2.2 | [copies-of-maps-and-sets-keep-their-items.kav](copies-of-maps-and-sets-keep-their-items.kav) | A copy of a map or set is a value of its own: changing one, or a collection held inside it, leaves the other as it was. | runs |
| 2.2 | [list-first-last-and-get-of-nothing-are-none.kav](list-first-last-and-get-of-nothing-are-none.kav) | `first`, `last` and `get` give an optional: `none` for an empty list, and `get` gives `none` for any index outside the list, below 0 too. | runs |
| 2.2 | [list-items-share-one-type.kav](list-items-share-one-type.kav) | The items of a list have one type: items of different kinds need a union, so a list literal that mixes them is a compile error. | compile error |
| 2.2 | [list-plus-needs-the-same-item-type.kav](list-plus-needs-the-same-item-type.kav) | `xs + ys` joins two lists of the same type; lists of different item types are a compile error. | compile error |
| 2.2 | [list-reading-methods-on-empty-and-full-lists.kav](list-reading-methods-on-empty-and-full-lists.kav) | `filter`, `map`, `flat_map`, `any`, `all`, `contains` and `enumerate` read a list without changing it; an empty list gives an empty list, `any` false and `all` true. | runs |
| 2.2 | [map-methods-read-keys-and-values.kav](map-methods-read-keys-and-values.kav) | A map has `len`, `is_empty`, `get`, `contains_key`, `keys`, `values` and `remove`; a missing key gives `none`, keys and values come in insertion order, and removing a missing key changes nothing. | runs |
| 2.2 | [nested-collections-change-in-place.kav](nested-collections-change-in-place.kav) | An item of an item can be changed in place (`grid[0][1] = v`, `grid[1].push(v)`) in a `let`; a copy made before keeps its items. | runs |
| 2.2 | [ranges-at-the-edges.kav](ranges-at-the-edges.kav) | A range reads as the list of its numbers: `a..b` with `b` at or below `a` is empty, `a..=a` holds `a` alone, and `first`, `last` and `get` read it as that list. | runs |
| 2.2 | [remove-from-a-const-map-is-an-error.kav](remove-from-a-const-map-is-an-error.kav) | `remove` changes the map it is called on, so it needs a `let`. | compile error |
| 2.2 | [removing-a-key-keeps-the-order-of-the-rest.kav](removing-a-key-keeps-the-order-of-the-rest.kav) | Removing from a map or set leaves the other items in the order they were added, in large ones too. | runs |
| 2.2 | [setting-a-key-of-a-const-map-is-an-error.kav](setting-a-key-of-a-const-map-is-an-error.kav) | `ages[name] = v` changes the map, so like assignment it needs a `let`. | compile error |
| 2.2 | [setting-an-item-of-a-const-list-is-an-error.kav](setting-an-item-of-a-const-list-is-an-error.kav) | `items[i] = v` changes the list, so like assignment it needs a `let`. | compile error |
| 2.2 | [setting-an-item-past-the-end-panics.kav](setting-an-item-past-the-end-panics.kav) | `items[i] = v` changes an item that is there; an index at or past the end panics, it never adds an item. | panic |
| 2.3 | [a-tuple-has-no-item-past-its-size.kav](a-tuple-has-no-item-past-its-size.kav) | A tuple's size is fixed by its type: reading an item past it is a compile error. | compile error |
| 2.3 | [tuple-destructuring-needs-every-item.kav](tuple-destructuring-needs-every-item.kav) | A tuple is taken apart by position, one name for each item: a pattern with more names than the tuple has items is a compile error. | compile error |
| 2.3 | [tuple-items-are-read-by-position.kav](tuple-items-are-read-by-position.kav) | A tuple's items are read by position, `t.0`, `t.1`, …, each with its own type; tuples are values compared by `==` item by item. | runs |
| 2.3 | [tuple-items-must-have-their-types.kav](tuple-items-must-have-their-types.kav) | Each item of a tuple has its own type: a tuple whose item has another type is a compile error. | compile error |
| 2.4, 1.4 | [field-of-a-const-record-cannot-change.kav](field-of-a-const-record-cannot-change.kav) | Fields are assigned only through a `let`: assigning a field of a `const` record is an error. | compile error |
| 2.4 | [optional-typed-field-must-still-be-given.kav](optional-typed-field-must-still-be-given.kav) | A field written `nickname: String?` (no `?` after the name, no default) must still be given, perhaps as `none`. | compile error |
| 2.4 | [record-default-has-the-field-type.kav](record-default-has-the-field-type.kav) | A record field's default must have the field's type. | compile error |
| 2.4 | [record-defaults-are-fresh-for-each-value.kav](record-defaults-are-fresh-for-each-value.kav) | Records are values: a default is made afresh for each record, and a copy from `with` or a function call never shares its lists or maps. | runs |
| 2.4 | [record-field-must-have-its-type.kav](record-field-must-have-its-type.kav) | Each named argument of a record must have its field's type. | compile error |
| 2.4 | [record-refuses-an-unknown-field.kav](record-refuses-an-unknown-field.kav) | A record is made only from its own fields: a named argument the type has no field for is an error. | compile error |
| 2.4 | [records-are-copied-into-calls-and-fields.kav](records-are-copied-into-calls-and-fields.kav) | Records are values: a record passed to a function, or stored in another record, is a copy, so changing it never changes the original. | runs |
| 2.4 | [with-names-only-existing-fields.kav](with-names-only-existing-fields.kav) | `with` changes fields the record has: naming a field it doesn't have is an error. | compile error |
| 2.5 | [extend-adds-methods-to-a-union.kav](extend-adds-methods-to-a-union.kav) | `extend` adds methods to a union, and they work on every variant. | runs |
| 2.5 | [extend-names-only-the-types-parameters.kav](extend-names-only-the-types-parameters.kav) | The names after `extend<…>` stand for the type's own parameters: a name that isn't one of them is an error. | compile error |
| 2.6 | [a-types-own-enum-helper-is-used.kav](a-types-own-enum-helper-is-used.kav) | A function or method an enum declares itself with a helper's name (`all`, `name`, `parse`, `value`, `from`) is used instead of the built-in one. | runs |
| 2.6 | [enum-numbers-are-unique.kav](enum-numbers-are-unique.kav) | No two variants of an enum have the same number. | compile error |
| 2.6 | [match-must-cover-variants-with-fields.kav](match-must-cover-variants-with-fields.kav) | A `match` on a union must cover every variant, those that carry fields too. | compile error |
| 2.6 | [variant-needs-its-fields.kav](variant-needs-its-fields.kav) | A variant field without a default must be given, as a record field must. | compile error |
| 2.7 | [a-field-of-an-optional-is-read-with-question-dot.kav](a-field-of-an-optional-is-read-with-question-dot.kav) | A field of a `T?` is read with `?.` or after unwrapping it: `.` on an optional is an error. | compile error |
| 2.7 | [an-optional-is-not-a-plain-value.kav](an-optional-is-not-a-plain-value.kav) | A `T?` is not a `T`: passing an optional where a plain value is expected is an error; handle `none` first. | compile error |
| 2.7 | [match-with-a-none-arm-is-optional.kav](match-with-a-none-arm-is-optional.kav) | A `match` whose one arm is `none` and the others a `T` gives a `T?`. | runs |
| 2.7 | [question-dot-skips-arguments-when-none.kav](question-dot-skips-arguments-when-none.kav) | `x?.m(a)` works out its arguments only when `x` is not `none`. | runs |
| 2.8 | [interface-methods-fail-as-declared.kav](interface-methods-fail-as-declared.kav) | A method that implements an interface's method can fail exactly when the interface's can: one that fails where the interface's can't is an error. | compile error |
| 2.8 | [interface-methods-take-as-many-values.kav](interface-methods-take-as-many-values.kav) | A method that implements an interface's method takes as many values as the interface's. | compile error |
| 2.8 | [interface-self-is-the-implementing-type.kav](interface-self-is-the-implementing-type.kav) | `Self` in an interface's method is read as the implementing type. | runs |
| 2.9 | [a-generic-type-takes-its-number-of-type-arguments.kav](a-generic-type-takes-its-number-of-type-arguments.kav) | A generic type is written with as many type arguments as it has parameters. | compile error |
| 2.9 | [a-generic-types-bound-must-be-met.kav](a-generic-types-bound-must-be-met.kav) | A generic type's bound must be met by the type it is made with: a record that doesn't implement `Comparable` can't fill `T: Comparable`. | compile error |
| 2.9 | [a-number-bound-is-floating-not-float.kav](a-number-bound-is-floating-not-float.kav) | A bound is an interface: `Float` is a type, so `<T: Float>` is an error, and the bound for floating-point numbers is `Floating`. | compile error |
| 2.9 | [a-where-bound-is-a-number-bound.kav](a-where-bound-is-a-number-bound.kav) | A method's `where` bound is `Number` or `Floating`; an interface there is an error. | compile error |
| 2.9 | [one-type-parameter-is-one-type.kav](one-type-parameter-is-one-type.kav) | Every use of a type parameter in one call is the same type: `same(1, "one")` finds no `T`. | compile error |
| 2.9 | [where-is-a-name-elsewhere.kav](where-is-a-name-elsewhere.kav) | `where` is a keyword only after a method's result type: elsewhere it may be a name. | runs |
| 2.9 | [whole-number-overflow-panics-in-generic-code.kav](whole-number-overflow-panics-in-generic-code.kav) | Whole-number overflow panics in generic code as it does elsewhere. | panic |
| 2.10, 3.1 | [a-value-return-needs-an-arrow.kav](a-value-return-needs-an-arrow.kav) | A function that returns a value must write its return type; with no `->` it returns nothing. | compile error |
| 2.10 | [lambda-and-local-types-are-inferred.kav](lambda-and-local-types-are-inferred.kav) | Local variables and lambda parameters and results need no written types: they are inferred. | runs |
| 2.10 | [record-field-types-required.kav](record-field-types-required.kav) | Record fields must have written types. | compile error |
| 2.11, 8.7 | [secret-cannot-be-debugged.kav](secret-cannot-be-debugged.kav) | A secret value must not flow into `debug`. | compile error |
| 2.11, 8.7 | [secret-cannot-go-into-a-panic.kav](secret-cannot-go-into-a-panic.kav) | A secret value must not flow into an error message: `panic(secret)` is an error. | compile error |
| 2.11, 8.7 | [secret-cannot-go-into-an-error-value.kav](secret-cannot-go-into-an-error-value.kav) | A secret value must not flow into an error's field unless that field is itself `secret`. | compile error |
| 2.11, 1.11 | [secret-fields-print-as-secret.kav](secret-fields-print-as-secret.kav) | Printing a record shows a secret field as `<secret>`, never its value. | runs |
| 2.12 | [json-bad-escapes-and-control-characters-are-errors.kav](json-bad-escapes-and-control-characters-are-errors.kav) | In JSON text, invalid escapes, raw control characters and unpaired surrogates are syntax errors. | runs |
| 2.12 | [json-char-is-text-of-one-character.kav](json-char-is-text-of-one-character.kav) | A `Char` is written as text of one character, and decoding takes only text of one character. | runs |
| 2.12 | [json-decode-reads-maps-with-text-keys.kav](json-decode-reads-maps-with-text-keys.kav) | A map with `String` keys decodes from an object, keeping its keys in order, bare or as a field. | runs |
| 2.12 | [json-decode-refuses-a-key-given-twice.kav](json-decode-refuses-a-key-given-twice.kav) | A key that appears twice in one object is an error when decoding too, at any depth, with where it is. | runs |
| 2.12 | [json-decode-refuses-an-unknown-variant.kav](json-decode-refuses-an-unknown-variant.kav) | A union is read by variant name; a name the union doesn't have, or a variant with fields given without them, is a problem at its path. | runs |
| 2.12 | [json-decode-text-booleans-arrays-and-objects-must-match.kav](json-decode-text-booleans-arrays-and-objects-must-match.kav) | Decoding is strict about values: text, booleans, arrays and objects must match the field's type, and `null` fits only an optional, which it makes `none`. | runs |
| 2.12 | [json-describe-refuses-a-parameter-with-no-schema.kav](json-describe-refuses-a-parameter-with-no-schema.kav) | `json.describe(f)` of a named function whose parameter type has no schema (a map with other keys) is a compile error. | compile error |
| 2.12 | [json-huge-exponents-are-out-of-range.kav](json-huge-exponents-are-out-of-range.kav) | A number whose exponent puts it beyond 10^10_000, or below 10^-10_000, is out of range: `decimal()` gives `none`, and a `Decimal` field refuses it. | runs |
| 2.12 | [json-infinities-are-written-as-null.kav](json-infinities-are-written-as-null.kav) | `NaN` and the infinities are written as `null`, as JavaScript does, bare, in lists and in record fields. | runs |
| 2.12 | [json-int-and-bigint-read-only-whole-numbers.kav](json-int-and-bigint-read-only-whole-numbers.kav) | `int()` reads a `Number` written without a fraction or exponent that fits `Int`, and `bigint()` one of any size; any other number gives `none`. | runs |
| 2.12 | [json-len-counts-only-arrays-and-objects.kav](json-len-counts-only-arrays-and-objects.kav) | `len()` gives the items of an `Array` or the fields of an `Object`, and 0 for every other kind, text included. | runs |
| 2.12 | [json-limits-can-be-raised-up-to-a-cap.kav](json-limits-can-be-raised-up-to-a-cap.kav) | A call can raise `max_depth` above 128, but never past 10_000; decoding has the same limits. | runs |
| 2.12 | [json-limits-on-size-and-fields.kav](json-limits-on-size-and-fields.kav) | Text longer than `max_bytes`, or an object with more than `max_fields` fields, is an error, not a value; text within the limits reads. | runs |
| 2.12 | [json-literals-encode-typed-values-inside.kav](json-literals-encode-typed-values-inside.kav) | Inside a `Json` literal, any expression whose type can be encoded is converted, at any depth: records, unions, optionals and lists of them. | runs |
| 2.12, 2.11 | [json-literals-refuse-secret-fields.kav](json-literals-refuse-secret-fields.kav) | A value inside a `Json` literal is encoded, so a record with a `secret` field there is a compile error. | compile error |
| 2.12 | [json-nan-infinity-and-hex-are-refused-even-in-jsonc.kav](json-nan-infinity-and-hex-are-refused-even-in-jsonc.kav) | `NaN`, `Infinity`, hex numbers and the rest of JSON5 are syntax errors, even with `comments: true`. | runs |
| 2.12 | [json-of-a-function-is-a-compile-error.kav](json-of-a-function-is-a-compile-error.kav) | A function can't be encoded as JSON: `json.of` of one is a compile error. | compile error |
| 2.12 | [json-of-a-map-with-other-keys-is-a-compile-error.kav](json-of-a-map-with-other-keys-is-a-compile-error.kav) | Only maps with `String` keys can be encoded as JSON: a map with other keys is a compile error, also inside a record. | compile error |
| 2.12 | [json-parse-refuses-text-that-is-not-standard-json.kav](json-parse-refuses-text-that-is-not-standard-json.kav) | `json.parse` takes only standard JSON (RFC 8259): anything else is a `Syntax` error, never a value. | runs |
| 2.12 | [json-schema-of-a-map-with-other-keys-is-a-compile-error.kav](json-schema-of-a-map-with-other-keys-is-a-compile-error.kav) | A map with keys other than `String` has no schema: `json.schema<T>()` of a record holding one is a compile error. | compile error |
| 2.12 | [json-schema-of-a-tuple-is-a-compile-error.kav](json-schema-of-a-tuple-is-a-compile-error.kav) | A type with no schema, such as a tuple, is a compile error for `json.schema<T>()`. | compile error |
| 2.12 | [json-schema-of-maps-and-numbers.kav](json-schema-of-maps-and-numbers.kav) | In a schema, a map with `String` keys is an object with `additionalProperties`, sized integers are `integer` (unsigned ones with `"minimum": 0`), `Float` and `Decimal` are `number`, and `Bool` is `boolean`. | runs |
| 2.12, 2.11 | [json-stringify-refuses-secret-fields-in-lists.kav](json-stringify-refuses-secret-fields-in-lists.kav) | A secret can't leak into JSON by accident: `json.stringify` of a list of records with a `secret` field is a compile error. | compile error |
| 2.12 | [json-text-escapes-are-read.kav](json-text-escapes-are-read.kav) | `json.parse` reads every JSON escape in text, a surrogate pair as one character, and `json.stringify` writes text that reads back the same. | runs |
| 2.12 | [json-values-of-every-kind-round-trip.kav](json-values-of-every-kind-round-trip.kav) | Decoding is the reverse of encoding: records, unions, optionals, lists, maps, sets, sized numbers, `Char`, `Decimal`, `BigInt` and nesting come back equal. | runs |
| 2.12 | [json-whole-number-fields-refuse-fractions-and-overflow.kav](json-whole-number-fields-refuse-fractions-and-overflow.kav) | An integer field takes only a number written without a fraction or exponent that fits its type; anything else is a problem naming the type, while a `Float` field takes any number. | runs |
| 2.13, 2.1 | [a-byte-is-a-uint8.kav](a-byte-is-a-uint8.kav) | `b[i]` is a `UInt8`, so using it as an `Int` needs a conversion. | compile error |
| 2.13, 2.1 | [bytes-are-made-from-uint8-values.kav](bytes-are-made-from-uint8-values.kav) | `Bytes(list)` takes a `[UInt8]`, so a literal item outside 0 to 255 is a compile error. | compile error |
| 2.13 | [bytes-get-and-slice-at-the-edges.kav](bytes-get-and-slice-at-the-edges.kav) | `b.get(i)` gives `none` outside the bytes, a negative `i` too; `b.slice` clamps positions past the end; empty `Bytes` print as `Bytes()`. | runs |
| 2.13 | [bytes-join-only-with-bytes.kav](bytes-join-only-with-bytes.kav) | `a + b` joins two `Bytes`; text must be turned into bytes first. | compile error |
| 2.13, 2.1 | [bytes-need-a-list-of-uint8.kav](bytes-need-a-list-of-uint8.kav) | `Bytes(list)` takes a `[UInt8]`; a `[Int]` must be converted item by item. | compile error |
| 2.14 | [anonymous-record-fields-change-through-let.kav](anonymous-record-fields-change-through-let.kav) | An anonymous record's fields are assigned through a `let`, like any record's, and a copy made before keeps its value. | runs |
| 2.14 | [anonymous-record-fields-must-have-their-types.kav](anonymous-record-fields-must-have-their-types.kav) | Anonymous record types match only with the same field types. | compile error |
| 2.14, 1.4 | [anonymous-record-fields-of-a-const-cannot-change.kav](anonymous-record-fields-of-a-const-cannot-change.kav) | An anonymous record's field can't be assigned through a `const`. | compile error |
| 2.14 | [anonymous-records-match-by-field-names.kav](anonymous-records-match-by-field-names.kav) | Anonymous record types match only with the same field names: `(x: 1, z: 2)` is not a `(x: Int, y: Int)`. | compile error |
| 2.14 | [declared-types-stay-nominal.kav](declared-types-stay-nominal.kav) | A declared `type` is nominal: an anonymous record of the same shape is not one. | compile error |
| 2.14 | [one-named-field-makes-a-record.kav](one-named-field-makes-a-record.kav) | One named field still makes an anonymous record, and a parenthesized expression `(x)` with no field name is just that expression. | runs |
| 2.15 | [an-interface-value-is-not-cast-back.kav](an-interface-value-is-not-cast-back.kav) | There is no cast back from an interface value to its concrete type. | compile error |
| 2.15 | [interface-values-have-only-the-interfaces-methods.kav](interface-values-have-only-the-interfaces-methods.kav) | On an interface value only the interface's methods exist: the concrete type's fields can't be read through it. | compile error |
| 2.15, 2.8 | [only-implementers-fit-an-interface-value.kav](only-implementers-fit-an-interface-value.kav) | Only a value of a type that implements the interface fits an interface value, even when it has the methods. | compile error |
| 2.16, 1.8 | [decimal-remainder-keeps-the-sign.kav](decimal-remainder-keeps-the-sign.kav) | `Decimal` is a number type, so it meets `Rem`: `%` works on it, keeping the left side's sign as for other numbers. | runs |
| 2.16 | [float-compare-of-nan-is-zero.kav](float-compare-of-nan-is-zero.kav) | a float's `compare` gives below 0, 0 or above 0 by order, and 0 when neither comes first, as with a `NaN`. | runs |
| 2.17 | [matrix-fast-product-of-whole-numbers-is-exact.kav](matrix-fast-product-of-whole-numbers-is-exact.kav) | for whole numbers, `a.product(b, fast: true)` gives the same result as `a @ b`. | runs |
| 2.17 | [matrix-identity-and-filled-of-floats.kav](matrix-identity-and-filled-of-floats.kav) | `matrix.identity(n, one)` has `one` on the diagonal and zeros of its type elsewhere; `matrix.filled` repeats its value. | runs |
| 2.17 | [matrix-index-out-of-range-breaks-require.kav](matrix-index-out-of-range-breaks-require.kav) | an index outside a matrix breaks a `require`, here a row past the last. | panic |
| 2.17 | [matrix-item-by-item-needs-the-same-size.kav](matrix-item-by-item-needs-the-same-size.kav) | `a + b`, `a - b` and `a * b` of matrices of different sizes break a `require`. | panic |
| 2.17 | [matrix-product-needs-columns.kav](matrix-product-needs-columns.kav) | `a @ b` when `a` has no columns breaks a `require`, even though `a.cols` equals `b.rows`. | panic |
| 2.17 | [matrix-rows-must-be-as-long.kav](matrix-rows-must-be-as-long.kav) | `matrix.of` with rows of different lengths panics. | panic |
| 2.17, 5.6 | [matrix-whole-number-product-overflow-panics.kav](matrix-whole-number-product-overflow-panics.kav) | a matrix product of whole numbers panics on a sum that doesn't fit, as overflow does anywhere. | panic |
| 2.18 | [a-view-cannot-change-its-string.kav](a-view-cannot-change-its-string.kav) | A view can't change the string it is part of: it has no changing methods. | compile error |
| 2.18 | [a-view-is-not-a-string.kav](a-view-is-not-a-string.kav) | A `StrView` is its own type: `to_string()` copies its text out when a `String` is needed. | compile error |
| 2.18 | [a-view-keeps-its-string-alive.kav](a-view-keeps-its-string-alive.kav) | A view keeps its string alive and keeps the text it had: it outlives the function that made the string, and the variable's next value doesn't change it; views can be stored in lists and records. | runs |
| 2.18 | [builder-push-takes-only-text.kav](builder-push-takes-only-text.kav) | A builder's `push` adds text (a `String` or a `StrView`); a number must be turned into text first. | compile error |
| 2.18 | [text-builder-replace-start-after-end-panics.kav](text-builder-replace-start-after-end-panics.kav) | A builder's `replace` from a start after its end is a panic (a contract), as `remove`'s is. | panic |
| 2.18 | [views-of-views-count-from-their-own-start.kav](views-of-views-count-from-their-own-start.kav) | `v.view(start, end)` counts characters from the view's own start and clamps as `slice` does; `trim_start`, `trim_end` and `to_string` work on views of any script. | runs |
| 2.1, 5.6 | [a-sized-remainder-by-zero-panics.kav](a-sized-remainder-by-zero-panics.kav) | Whole-number remainder by zero panics for a sized type too, here `UInt8`. | panic |
| 2.1, 5.6 | [float-division-by-zero-does-not-panic.kav](float-division-by-zero-does-not-panic.kav) | only whole-number division by zero panics; a float divided by zero is an IEEE infinity or `NaN`. | runs |
| 2.2 | [capacity-above-the-limit-panics.kav](capacity-above-the-limit-panics.kav) | `List.capacity(n)`, `Map.capacity(n)` and `Set.capacity(n)` with `n` above 4,294,967,294 (2³² − 2) panic. | runs |
| 2.2 | [capacity-below-zero-holds-nothing.kav](capacity-below-zero-holds-nothing.kav) | A capacity below 0 counts as 0, for maps and sets too: they are full at once, and adding to one panics. | panic |
| 2.12 | [json-objects-past-max-fields-are-an-error.kav](json-objects-past-max-fields-are-an-error.kav) | By default, parsing an object with more than `max_fields: 100_000` fields is an error, not a memory blow-up; 100_000 fields are read. | runs |
| 2.12 | [json-text-past-max-bytes-is-an-error.kav](json-text-past-max-bytes-is-an-error.kav) | By default, parsing text longer than `max_bytes: 64_000_000` is an error, `TooLarge`, not a memory blow-up. | runs |
| 2.2 | [a-ring-past-the-limit-panics.kav](a-ring-past-the-limit-panics.kav) | `Ring.capacity(n)` with `n` above 4,294,967,294 is a panic, as for a list. | panic |
