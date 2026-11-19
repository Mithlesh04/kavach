# 14. Standard library

Conformance tests for spec chapter [14. Standard library](../../../docs/spec/14-standard-library/README.md). Format: [../README.md](../README.md).

The tests are listed in four files, by area:

| Area | Sections | List |
|---|---|---|
| The core modules | 14.1 to 14.9 | Below |
| Networking | 14.10, 14.11, 14.14 | [network.md](network.md) |
| Services and tools | 14.12, 14.13, 14.15, 14.16, 14.19 to 14.21, 14.23, 14.32 | [services-and-tools.md](services-and-tools.md) |
| The machine and media | 14.17, 14.18, 14.22, 14.24 to 14.31, 14.33 | [machine-and-media.md](machine-and-media.md) |

## The core modules

| Spec | Test | Rule | Expects |
|---|---|---|---|
| 14.1 | [files-need-fs.kav](files-need-fs.kav) | Calling std.fs needs the Fs capability. | compile error |
| 14.1 | [unknown-modules-are-errors.kav](unknown-modules-are-errors.kav) | Only the modules of chapter 14 exist in `std`. | compile error |
| 14.1 | [std-types-are-named-by-use.kav](std-types-are-named-by-use.kav) | `use std.time.{Date}` names the type itself, which makes values and is checked like any type. | compile error |
| 14.1 | [runtime-functions-are-not-named-alone.kav](runtime-functions-are-not-named-alone.kav) | `use std.fs.{read_text}` is a compile error: a function the runtime provides is called through its module. | compile error |
| 14.2 | [number-methods.kav](number-methods.kav) | Numbers have abs, min, max and clamp. | runs |
| 14.2 | [text-methods.kav](text-methods.kav) | Text has replace, lines, chars and index_of, and slices; positions count characters. | runs |
| 14.2 | [text-split.kav](text-split.kav) | split returns the parts between separators, empty parts included. | runs |
| 14.2 | [list-methods.kav](list-methods.kav) | Lists have join, sort, sort_by, find, count, index_of, min and max, and slices give a reversed copy, the first items and the rest. | runs |
| 14.2 | [list-slice-takes-a-range-of-items.kav](list-slice-takes-a-range-of-items.kav) | `xs[start:end]` gives items `start..end`, both clamped to the list: a computed start below 0 counts as the start. | runs |
| 14.2 | [chars-join-and-text-contains-a-char.kav](chars-join-and-text-contains-a-char.kav) | A `[Char]` joins into text, and `contains` looks for a `Char` as for a `String`. | runs |
| 14.2 | [pop-takes-the-last-item.kav](pop-takes-the-last-item.kav) | `xs.pop()` takes out and gives the last item, or `none` for an empty list; a copy made before keeps it. | runs |
| 14.2 | [pop-on-a-const-list-is-an-error.kav](pop-on-a-const-list-is-an-error.kav) | `xs.pop()` changes `xs`, so `xs` must be a `let`. | compile error |
| 14.2 | [sets-hold-each-value-once.kav](sets-hold-each-value-once.kav) | A set holds each value once and keeps insertion order. | runs |
| 14.2, 2.2 | [sets-walk-like-lists.kav](sets-walk-like-lists.kav) | A set has `any`, `all`, `count`, `sum` and `find` over its values in insertion order, whatever they hold. | runs |
| 14.2, 2.2 | [set-map-and-filter-give-sets.kav](set-map-and-filter-give-sets.kav) | A set's `map` and `filter` give a new set in insertion order, equal results kept once, on sets of any values and inside lists and records. | runs |
| 14.2 | [other-languages-method-names-say-kavachs.kav](other-languages-method-names-say-kavachs.kav) | `some` on a list is a compile error that names `any(test)`. | compile error |
| 14.2 | [every-on-a-set-names-all.kav](every-on-a-set-names-all.kav) | `every` on a set is a compile error that names `all(test)`. | compile error |
| 14.2 | [each-names-for-each.kav](each-names-for-each.kav) | `each` on a list is a compile error that names `for_each(f)`. | compile error |
| 14.2 | [includes-names-contains.kav](includes-names-contains.kav) | `includes` is a compile error that names `contains(x)`. | compile error |
| 14.2, 1.5 | [camel-case-method-names-name-snake-case.kav](camel-case-method-names-name-snake-case.kav) | A camelCase method name is a compile error that names Kavach's `snake_case` method. | compile error |
| 14.2 | [lists-and-sets-walk-with-for-each-and-find-index.kav](lists-and-sets-walk-with-for-each-and-find-index.kav) | `for_each`, `find_index` and `find_last_index` on lists of any values, and `for_each` on sets. | runs |
| 14.2 | [set-changes-need-a-var.kav](set-changes-need-a-var.kav) | Changing a set with `add` or `remove` needs a `let`. | compile error |
| 14.2 | [text-pads-words-and-capitals.kav](text-pads-words-and-capitals.kav) | Text pads, splits into words, capitalizes, finds blank text, trims one side, counts and finds the last part, and removes a prefix or suffix, with methods; a slice cuts it short. | runs |
| 14.2 | [float-near.kav](float-near.kav) | `x.near(y)` is true for floats equal but for rounding, never for `NaN`. | runs |
| 14.2 | [sort-puts-nan-last.kav](sort-puts-nan-last.kav) | `NaN` sorts after every number, so a list holding one still sorts. | runs |
| 14.2 | [sort-by-needs-an-ordered-key.kav](sort-by-needs-an-ordered-key.kav) | `sort_by`'s key must have an order: a record key is a compile error, not a list left unsorted. | compile error |
| 14.2 | [floor-division-and-decimal-rounding.kav](floor-division-and-decimal-rounding.kav) | `x.div_floor(n)` rounds down and `x.mod(n)` has the sign of `n`; `d.round(places)` rounds a half away from zero and `d.round_even(places)` to the even digit. | runs |
| 14.2 | [float-bits-round-trip.kav](float-bits-round-trip.kav) | `x.bits()` gives a `Float`'s IEEE 754 bits; `Float.from_bits(n)` gives it back. | runs |
| 14.2 | [fract-is-the-part-above-the-floor.kav](fract-is-the-part-above-the-floor.kav) | `x.fract()` is `x - math.floor(x)`, never negative, for `Float` and `Float32`. | runs |
| 14.2 | [abs-of-the-smallest-int-panics.kav](abs-of-the-smallest-int-panics.kav) | `abs` of a whole number's smallest value doesn't fit, so it panics. | panic |
| 14.2, 5.6 | [checked-arithmetic-gives-none-on-overflow.kav](checked-arithmetic-gives-none-on-overflow.kav) | `checked_sub`, `checked_mul` and `checked_div` give `none` when the result doesn't fit, and the value when it does. | runs |
| 14.2 | [decimal-round-to-negative-places-panics.kav](decimal-round-to-negative-places-panics.kav) | `d.round(places)` with `places` below 0 panics. | panic |
| 14.2 | [div-floor-by-zero-panics.kav](div-floor-by-zero-panics.kav) | `x.div_floor(n)` with `n` of 0 panics, as `/` does. | panic |
| 14.2 | [find-and-index-of-give-none-when-missing.kav](find-and-index-of-give-none-when-missing.kav) | `find` and `index_of` give `none` when no item matches, and the first match when several do; `count` gives 0 for none. | runs |
| 14.2 | [float-min-max-and-clamp.kav](float-min-max-and-clamp.kav) | `min`, `max` and `clamp` work on floats and sized whole numbers as on `Int`. | runs |
| 14.2 | [join-needs-text-items.kav](join-needs-text-items.kav) | `join` is for lists of `String` or `Char`; a list of numbers is a compile error. | compile error |
| 14.2 | [mod-by-zero-panics.kav](mod-by-zero-panics.kav) | `x.mod(n)` with `n` of 0 panics, as `%` does. | panic |
| 14.2 | [near-allows-for-the-type.kav](near-allows-for-the-type.kav) | `x.near(y)` allows one billionth for a `Float` (`1e-12` near zero), and one hundred-thousandth for a `Float32` (`1e-6` near zero). | runs |
| 14.2 | [set-methods-on-empty-and-missing-values.kav](set-methods-on-empty-and-missing-values.kav) | A set has `contains`, `len`, `is_empty` and `to_list`; removing a value that isn't there changes nothing, and an empty set prints as `Set.of()`. | runs |
| 14.2 | [sets-hold-equal-values-once.kav](sets-hold-equal-values-once.kav) | A set holds each value once by value, as `==` compares: equal lists, tuples, records and maps are one item. | runs |
| 14.2 | [sort-by-keeps-equal-keys-in-order.kav](sort-by-keeps-equal-keys-in-order.kav) | `sort_by` keeps items with equal keys in the order they had, in a long list too. | runs |
| 14.2 | [sort-needs-items-with-an-order.kav](sort-needs-items-with-an-order.kav) | `sort()` is for lists of values with an order; a list of records has none, so sorting it is a compile error. | compile error |
| 14.2 | [sort-and-slices-give-copies.kav](sort-and-slices-give-copies.kav) | `sort`, `sort_by` and the slice `xs[::-1]` give a new list; the list they are made from doesn't change. | runs |
| 14.2 | [first-and-rest-past-the-end.kav](first-and-rest-past-the-end.kav) | `xs[:n]` gives the first `n` items and `xs[n:]` all but them; with `n` past the end, `xs[:n]` gives every item and `xs[n:]` none. | runs |
| 14.2 | [text-already-wide-enough-is-not-padded.kav](text-already-wide-enough-is-not-padded.kav) | `pad_left` and `pad_right` count characters, take any `Char` as fill, and leave text that is already `width` long as it was. | runs |
| 14.2 | [text-capitalize-changes-only-the-first-character.kav](text-capitalize-changes-only-the-first-character.kav) | `capitalize` puts only the first character in upper case and keeps the rest, where `title` lowers the rest of each word; empty and blank text are handled. | runs |
| 14.2 | [text-counts-characters-in-any-script.kav](text-counts-characters-in-any-script.kav) | Text's `len` counts characters, not bytes, and `upper`, `lower`, `starts_with`, `ends_with`, `repeat` and `is_empty` work on text in any script. | runs |
| 14.2 | [text-lines-drop-crlf-endings.kav](text-lines-drop-crlf-endings.kav) | `lines` gives the lines without their line endings, a Windows `\r\n` ending included. | runs |
| 14.2 | [text-pad-width-over-a-million-panics.kav](text-pad-width-over-a-million-panics.kav) | The `width` of `pad_left` and `pad_right` is at most 1,000,000; a wider pad panics instead of filling memory. | panic |
| 14.2 | [text-positions-past-the-end-are-clamped.kav](text-positions-past-the-end-are-clamped.kav) | A slice clamps positions past the end of the text: `s[:max]` gives at most `max` characters, the whole text when it is shorter. | runs |
| 14.2 | [decimal-round-even-on-negatives-and-ties.kav](decimal-round-even-on-negatives-and-ties.kav) | `d.round_even(places)` rounds a half to the even digit for negative values too, while `round` rounds it away from zero; a `Decimal` with fewer places is given back as it is. | runs |
| 14.2 | [decimal-round-even-to-negative-places-panics.kav](decimal-round-even-to-negative-places-panics.kav) | `d.round_even(places)` with `places` below 0 panics, as `round` does. | panic |
| 14.2 | [decimal-round-is-only-for-decimal.kav](decimal-round-is-only-for-decimal.kav) | `round(places)` is a `Decimal` method; a `Float` has none, so calling it is a compile error. | compile error |
| 14.2 | [float-bits-of-infinity-and-nan.kav](float-bits-of-infinity-and-nan.kav) | `bits` gives the IEEE 754 form of infinities too, and `Float.from_bits` gives `NaN` back from a `NaN`'s bits. | runs |
| 14.2 | [float-bits-of-negative-zero.kav](float-bits-of-negative-zero.kav) | `-0.0` is IEEE 754 negative zero, so its `bits` have only the sign bit set, written as a literal or made by negating `0.0`. | runs |
| 14.2 | [float-fract-of-nan-and-infinity-is-nan.kav](float-fract-of-nan-and-infinity-is-nan.kav) | `x.fract()` is `x - math.floor(x)`, so it is `NaN` for `NaN` and for either infinity, and 0.0 for a whole number. | runs |
| 14.2 | [float-from-bits-needs-a-uint64.kav](float-from-bits-needs-a-uint64.kav) | `Float.from_bits(n)` takes a `UInt64`; an `Int` is a compile error, as numbers never convert by themselves. | compile error |
| 14.2 | [float-near-infinity-is-near-only-itself.kav](float-near-infinity-is-near-only-itself.kav) | `x.near(y)` is "equal but for rounding": infinity is near the same infinity, but not the other one and not the largest finite numbers. | runs |
| 14.2 | [float-near-nan-is-near-nothing.kav](float-near-nan-is-near-nothing.kav) | `NaN` is near nothing: not a number, not infinity, not zero, from either side, for `Float32` too. | runs |
| 14.2 | [list-count-needs-a-bool-test.kav](list-count-needs-a-bool-test.kav) | `items.count(f)` takes an `f` that gives a `Bool`; one that gives a number is a compile error. | compile error |
| 14.2 | [list-index-of-compares-by-value.kav](list-index-of-compares-by-value.kav) | `index_of` finds the first item equal to `x` by value, as `==` compares: records, tuples, text and lists. | runs |
| 14.2 | [list-join-of-empty-and-one-item.kav](list-join-of-empty-and-one-item.kav) | `join` puts `sep` only between items: an empty list gives empty text, one item gives that item, and empty items stay. | runs |
| 14.2 | [list-max-needs-items-with-an-order.kav](list-max-needs-items-with-an-order.kav) | `min` and `max` are for lists of values with an order; a list of records has none, so asking for its largest is a compile error. | compile error |
| 14.2 | [list-min-max-of-text-chars-and-tuples.kav](list-min-max-of-text-chars-and-tuples.kav) | `min` and `max` work on any list of values with an order: text and characters by code point, tuples item by item; an empty list gives `none`. | runs |
| 14.2 | [list-sort-by-puts-nan-keys-last.kav](list-sort-by-puts-nan-keys-last.kav) | In `sort_by`, a `NaN` key sorts after every number, infinity too, so a list with one still sorts. | runs |
| 14.2 | [list-sort-is-not-a-text-method.kav](list-sort-is-not-a-text-method.kav) | List methods are not text methods: `sort` on a `String` is a compile error; sort its `chars()`. | compile error |
| 14.2 | [list-sort-orders-chars-decimals-and-sized-numbers.kav](list-sort-orders-chars-decimals-and-sized-numbers.kav) | `sort` orders characters by code point and numbers by value, for negatives, infinities, sized whole numbers and `Decimal` too. | runs |
| 14.2 | [number-abs-of-the-smallest-int8-panics.kav](number-abs-of-the-smallest-int8-panics.kav) | `abs` of a sized whole number's smallest value doesn't fit in its type, so it panics, as for `Int`. | panic |
| 14.2 | [number-abs-on-every-number-type.kav](number-abs-on-every-number-type.kav) | `abs` gives the absolute value for every number type: sized whole numbers, `Float32` and `Decimal`, keeping the type. | runs |
| 14.2 | [number-clamp-keeps-both-ends.kav](number-clamp-keeps-both-ends.kav) | `x.clamp(lo, hi)` limits `x` to `lo..=hi`, both ends included, for sized whole numbers and `Decimal` too. | runs |
| 14.2 | [number-div-floor-and-mod-fit-together.kav](number-div-floor-and-mod-fit-together.kav) | For whole numbers, `x.div_floor(n) * n + x.mod(n)` is `x`, and `x.mod(n)` is 0 or has the sign of `n` and is smaller than `n`, sized types included. | runs |
| 14.2 | [number-div-floor-is-only-for-whole-numbers.kav](number-div-floor-is-only-for-whole-numbers.kav) | `div_floor` is for whole numbers; on a `Float` it is a compile error. | compile error |
| 14.2 | [number-near-is-only-for-floats.kav](number-near-is-only-for-floats.kav) | `near` is for floats; on a whole number it is a compile error. | compile error |
| 14.2 | [ring-empty-and-out-of-range-give-none.kav](ring-empty-and-out-of-range-give-none.kav) | An empty ring is empty: `first`, `last` and `get` give `none`, `contains` is false and `to_list` is `[]`; `get` past either end gives `none`. | runs |
| 14.2 | [set-add-needs-the-item-type.kav](set-add-needs-the-item-type.kav) | A set holds values of one type: `add` with a value of another type is a compile error. | compile error |
| 14.2 | [set-readded-value-goes-last.kav](set-readded-value-goes-last.kav) | A set keeps insertion order: adding a value it holds doesn't move it, and a value removed and added again goes last. | runs |
| 14.2 | [text-chars-are-code-points.kav](text-chars-are-code-points.kav) | `chars` gives one `Char` per character as `len` counts them: a combining mark and each part of a joined emoji are characters of their own, and joining them gives the text back. | runs |
| 14.2 | [text-contains-needs-text-or-a-char.kav](text-contains-needs-text-or-a-char.kav) | `contains` looks for a `String` or a `Char`; a number is a compile error. | compile error |
| 14.2 | [text-count-on-empty-text-and-runs.kav](text-count-on-empty-text-and-runs.kav) | `s.count(part)` doesn't count overlaps, and an empty `part` is counted `s.len() + 1` times, once in empty text, characters counted as `len` counts them. | runs |
| 14.2 | [text-index-of-counts-characters.kav](text-index-of-counts-characters.kav) | `index_of` and `last_index_of` give character positions, past emoji and combining marks too, and `none` in empty text. | runs |
| 14.2 | [text-pad-counts-code-points.kav](text-pad-counts-code-points.kav) | `pad_left` and `pad_right` count characters as `len` does, so a combining mark is one character, and the fill may be an emoji. | runs |
| 14.2 | [text-pad-fill-must-be-a-char.kav](text-pad-fill-must-be-a-char.kav) | The `fill` of `pad_left` and `pad_right` is a `Char`; a `String` is a compile error. | compile error |
| 14.2 | [text-pad-left-width-over-a-million-panics.kav](text-pad-left-width-over-a-million-panics.kav) | `pad_left`'s `width` is at most 1,000,000, as `pad_right`'s is; a wider pad panics instead of filling memory. | panic |
| 14.2 | [text-remove-prefix-takes-one-copy.kav](text-remove-prefix-takes-one-copy.kav) | `remove_prefix` and `remove_suffix` take `p` off once; text that doesn't start or end with `p`, or an empty `p`, gives `s` itself. | runs |
| 14.2 | [text-replace-changes-every-match.kav](text-replace-changes-every-match.kav) | `s.replace(old, new)` replaces every `old`, without overlaps, in any script; with no `old` in `s`, it gives `s` as it was. | runs |
| 14.2 | [text-slice-clamps-both-ends.kav](text-slice-clamps-both-ends.kav) | `s[start:end]` clamps both positions to the text, as a list's slice does: a computed start below 0 counts as the start, and an end before the start gives empty text; positions count characters. | runs |
| 14.2 | [text-title-and-words-in-any-script.kav](text-title-and-words-in-any-script.kav) | `words` splits at spaces, tabs and line breaks in text of any script, and `title` capitalizes each such word, accented letters too, keeping what is between them. | runs |
| 14.2 | [text-trim-start-and-end-take-tabs-and-line-breaks.kav](text-trim-start-and-end-take-tabs-and-line-breaks.kav) | `trim_start` and `trim_end` take off spaces, tabs and line breaks (Windows `\r\n` too) at one side only, as `trim` does at both. | runs |
| 14.2 | [text-slices-count-code-points.kav](text-slices-count-code-points.kav) | `s[:max]` keeps at most `max` characters as `len` counts them: whole emoji, and a combining mark apart from its letter. | runs |
| 14.2 | [text-upper-is-not-a-number-method.kav](text-upper-is-not-a-number-method.kav) | Text methods are not number methods: `upper` on an `Int` is a compile error. | compile error |
| 14.2 | [min-max-and-clamp-give-nan-for-nan.kav](min-max-and-clamp-give-nan-for-nan.kav) | `min`, `max` and `clamp` give `NaN` when any value is `NaN`, on either side. | runs |
| 14.2 | [clamp-with-lo-above-hi-panics.kav](clamp-with-lo-above-hi-panics.kav) | `x.clamp(lo, hi)` with `lo` above `hi` breaks its contract: a panic. | panic |
| 14.2 | [mod-of-the-smallest-int-by-minus-one-is-zero.kav](mod-of-the-smallest-int-by-minus-one-is-zero.kav) | `Int.MIN.mod(-1)` is 0, as `Int.MIN % -1` is; `div_floor` of them overflows. | runs |
| 14.3 | [files-read-and-write.kav](files-read-and-write.kav) | std.fs reads, writes, appends, lists and removes files. | runs |
| 14.3 | [files-are-written-piece-by-piece.kav](files-are-written-piece-by-piece.kav) | `fs.create_writer` writes a file as its pieces come; `finish` gives the bytes written, a writer let go unfinished deletes its file, and `write` after `finish` fails. | runs |
| 14.3 | [missing-files-are-errors.kav](missing-files-are-errors.kav) | A file operation that fails returns an error value, not a crash: an `FsError`. | runs |
| 14.3 | [files-hold-bytes.kav](files-hold-bytes.kav) | `fs.write_bytes` writes any bytes atomically; `fs.read_bytes` reads them back. | runs |
| 14.3 | [files-are-read-as-the-loop-goes.kav](files-are-read-as-the-loop-goes.kav) | `fs.lines` and `fs.chunks` read a file as the loop goes; opening a missing file is an `FsError`. | runs |
| 2.13, 14.3 | [big-chunks-read-as-numbers.kav](big-chunks-read-as-numbers.kav) | `as_list` on a big chunk of a file gives its numbers, whether or not anything else holds the chunk, and a chunk read again keeps its bytes. | runs |
| 14.3 | [big-chunks-read-the-whole-file-in-order.kav](big-chunks-read-the-whole-file-in-order.kav) | Pieces of 1 MB or more come as any others do, and the file can go when the loop stops early. | runs |
| 14.3 | [read-errors-go-to-the-loop-header.kav](read-errors-go-to-the-loop-header.kav) | A read that fails during a loop over `fs.lines` goes to the header's `try` (the error is returned) or `catch` (its handler runs). | runs |
| 14.3, 10.2 | [fs-errors-can-be-matched.kav](fs-errors-can-be-matched.kav) | An `FsError` is a union that code can `match` on, naming its variants through the module (`fs.NotFound`) or alone. | runs |
| 14.3 | [durable-writes-force-the-disk.kav](durable-writes-force-the-disk.kav) | `fs.write_durable` replaces the file atomically and forces it to the disk; file scopes hold for it. | runs |
| 14.3 | [fs-info-and-permissions.kav](fs-info-and-permissions.kav) | `fs.info` gives a path's kind, size and permissions; `set_read_only` and `set_permissions` change them; `copy`, `rename` and `remove_all` move and delete; a mode past `0o7777` fails. | runs |
| 14.3 | [watching-files-needs-fs-read.kav](watching-files-needs-fs-read.kav) | Watching files needs `Fs.read`. | compile error |
| 14.3 | [fs-a-folder-is-not-a-file.kav](fs-a-folder-is-not-a-file.kav) | a folder where a file is expected is an `FsError`: reading it, writing over it, or listing a file as a folder fails, and nothing changes. | runs |
| 14.3 | [fs-a-kept-reader-panics-on-a-bad-read.kav](fs-a-kept-reader-panics-on-a-bad-read.kav) | a `Lines` reader kept in a variable and looped over later has no loop header to send an error to, so a failed read is a panic. | panic |
| 14.3 | [fs-append-creates-a-missing-file.kav](fs-append-creates-a-missing-file.kav) | `fs.append_text` adds to the end of a file, creating it when it is missing. | runs |
| 14.3 | [fs-create-dir-makes-missing-parents.kav](fs-create-dir-makes-missing-parents.kav) | `fs.create_dir` creates a folder and every missing parent on the way to it. | runs |
| 14.3, 14.4 | [fs-info-gives-size-and-a-recent-moment.kav](fs-info-gives-size-and-a-recent-moment.kav) | `fs.info` gives a file's size in bytes and `modified_ms`, a moment (14.4) of its last change. | runs |
| 14.3 | [fs-list-gives-sorted-names.kav](fs-list-gives-sorted-names.kav) | `fs.list` gives the names in a folder, files and folders alike, sorted; a missing folder is `NotFound`. | runs |
| 14.3 | [fs-missing-paths-are-not-found.kav](fs-missing-paths-are-not-found.kav) | a file operation on a path with nothing there fails with `NotFound(path)`, and `fs.exists` gives `false`. | runs |
| 14.3 | [fs-read-text-refuses-text-that-is-not-utf8.kav](fs-read-text-refuses-text-that-is-not-utf8.kav) | `fs.read_text` reads only UTF-8: a file with other bytes is `FsError.NotText`, not text with holes; `fs.read_bytes` still reads it. | runs |
| 14.3 | [fs-errors-name-folders.kav](fs-errors-name-folders.kav) | Reading a folder as a file is `FsError.IsFolder`; removing a folder that isn't empty is `FsError.NotEmpty`. | runs |
| 14.3, 8.1 | [fs-reading-needs-only-fs-read.kav](fs-reading-needs-only-fs-read.kav) | reading calls (`read_text`, `read_bytes`, `lines`, `chunks`, `exists`, `list`, `info`, `watch`) need only `Fs.read`. | runs |
| 14.3 | [fs-remove-refuses-a-folder-with-files.kav](fs-remove-refuses-a-folder-with-files.kav) | `fs.remove` deletes a file or an empty folder; a folder with files in it is an `FsError` and stays, and a missing path is `NotFound`. | runs |
| 14.3 | [fs-rename-and-copy-replace-the-target.kav](fs-rename-and-copy-replace-the-target.kav) | `fs.rename` moves a file or a folder and `fs.copy` copies a file; a file already at `to` is replaced. | runs |
| 14.3 | [fs-symlinks-are-relative-to-their-folder.kav](fs-symlinks-are-relative-to-their-folder.kav) | `fs.symlink` keeps a relative target relative to the link's folder, and `fs.info` describes the link itself; where links aren't allowed it fails with `Denied`. | runs |
| 14.3 | [fs-watch-a-missing-path-is-not-found.kav](fs-watch-a-missing-path-is-not-found.kav) | `fs.watch` fails with `NotFound` when nothing is at the path; a new watcher's `poll()` is empty, and after `close()` `next()` gives `none`. | runs |
| 14.3 | [fs-watch-hears-a-removed-file.kav](fs-watch-hears-a-removed-file.kav) | a watcher on a folder hears changes under it: `poll()` gives each changed path once, written under the watched path, with the kind of its last change, so a removed file is `Removed`. | runs |
| 14.3, 11.5 | [fs-watch-sees-no-changes-under-test.kav](fs-watch-sees-no-changes-under-test.kav) | under `kavach test`, a watcher watches the fake file system and sees no changes: `poll()` is empty and `next()` gives `none` at once. | tests pass |
| 14.3 | [fs-write-text-replaces-the-whole-file.kav](fs-write-text-replaces-the-whole-file.kav) | `fs.write_text` and `fs.write_bytes` replace the whole file: a shorter text leaves nothing of the longer one, and no temporary file stays behind. | runs |
| 14.3 | [fs-writer-empties-a-file-already-there.kav](fs-writer-empties-a-file-already-there.kav) | `fs.create_writer` gives a new, empty file: one already at the path is emptied before the writes, and `finish` counts only the new bytes. | runs |
| 14.3, 8.1 | [fs-writer-needs-fs-write.kav](fs-writer-needs-fs-write.kav) | `fs.create_writer` changes files, so it needs `Fs.write`: a function with only `Fs.read` can't call it. | compile error |
| 14.3, 8.1 | [fs-writer-stays-inside-its-folder.kav](fs-writer-stays-inside-its-folder.kav) | with `uses Fs("folder")`, `fs.create_writer` refuses a path outside the folder with `Denied(path)` and makes no file. | runs |
| 14.4 | [clocks-move-forward.kav](clocks-move-forward.kav) | time.monotonic_ms never goes back, and time.sleep_ms waits. | runs |
| 14.4 | [tests-freeze-the-clock.kav](tests-freeze-the-clock.kav) | A `test` block freezes the clock with `time.freeze`; `sleep` moves it on at once. | kavach test |
| 14.4 | [freeze-is-only-for-tests.kav](freeze-is-only-for-tests.kav) | `time.freeze` outside a `test` block is a compile error. | compile error |
| 14.4 | [intervals-count-rounds.kav](intervals-count-rounds.kav) | A for loop over time.every_ms counts rounds, each starting ms after the one before. | runs |
| 14.4 | [durations-are-written-with-units.kav](durations-are-written-with-units.kav) | A whole number followed by `ms`, `seconds`, `minutes`, `hours` or `days` is a `Duration`; `time.sleep` and `time.every` take one. | runs |
| 14.4 | [durations-have-no-months.kav](durations-have-no-months.kav) | A whole number takes only the units of a `Duration`: `2.months` is a compile error, since a month has no fixed length. | compile error |
| 14.4 | [timers-call-a-function-later.kav](timers-call-a-function-later.kav) | `time.after(d, f)` is a task that calls `f` after `d`, cancelled when the function that made it returns. | runs |
| 14.4 | [zones-show-local-time.kav](zones-show-local-time.kav) | A `Zone` from `time.zone(name)` gives the date and time its clocks show at a moment, daylight saving time included. | runs |
| 14.4 | [zones-move-a-skipped-time-later.kav](zones-move-a-skipped-time-later.kav) | A time of day the clocks skipped when daylight saving time began moves later by the gap, as RFC 5545 does; `resolve` gives both moments. | runs |
| 14.4 | [zones-take-a-repeated-time-earlier.kav](zones-take-a-repeated-time-earlier.kav) | A time of day the clocks showed twice when daylight saving time ended is its earlier moment, as RFC 5545 does; `resolve` gives both. | runs |
| 14.4 | [zones-unknown-name-fails.kav](zones-unknown-name-fails.kav) | `time.zone(name)` fails with `UnknownZone(name)` for a name the IANA database doesn't have. | runs |
| 14.4 | [zones-can-be-fixed-offsets.kav](zones-can-be-fixed-offsets.kav) | `time.zone` also takes a fixed offset from UTC; one that can't be fails with `UnknownZone`. | runs |
| 14.4 | [zones-write-moments-with-their-offset.kav](zones-write-moments-with-their-offset.kav) | `z.iso(ms)` writes a moment with its offset, and `parse_moment` reads it back. | runs |
| 14.4 | [times-are-read-from-text.kav](times-are-read-from-text.kav) | `parse_time` and `parse_datetime` read what `iso` writes; anything else fails with `Invalid`. | runs |
| 14.4 | [moments-are-read-only-with-an-offset.kav](moments-are-read-only-with-an-offset.kav) | `parse_moment` needs `Z` or an offset: a time with no zone is no moment. | runs |
| 14.4 | [dates-follow-the-gregorian-calendar.kav](dates-follow-the-gregorian-calendar.kav) | Dates follow the Gregorian calendar with months 1 to 12; adding a month keeps the day or gives the month's last; a moment becomes a `DateTime` in UTC. | runs |
| 14.4, 8.1 | [reading-the-clock-needs-no-capability.kav](reading-the-clock-needs-no-capability.kav) | Reading the clock needs no capability, even in a `pub` function. | runs |
| 14.4 | [contracts-cannot-read-the-clock.kav](contracts-cannot-read-the-clock.kav) | A contract may not read the clock. | compile error |
| 14.4 | [impossible-dates-cannot-be-made.kav](impossible-dates-cannot-be-made.kav) | A `Date` that can't exist breaks its invariant. | panic |
| 14.4 | [durations-add-and-compare.kav](durations-add-and-compare.kav) | `Duration` is `Add`, `Sub`, `Mul<Int>` and `Comparable`. | runs |
| 14.4 | [a-frozen-clock-gives-today-and-now.kav](a-frozen-clock-gives-today-and-now.kav) | On a frozen clock, `time.today()` and `time.now()` give the frozen moment's date and time in UTC, and `sleep` moves them on. | tests pass |
| 14.4, 5.6 | [a-unit-too-large-overflows.kav](a-unit-too-large-overflows.kav) | A whole number too large for its unit overflows like any `Int` arithmetic, so it panics. | panic |
| 14.4 | [dates-move-across-months-and-years.kav](dates-move-across-months-and-years.kav) | `add_days` and `add_months` move across months and years, back for a negative `n`, a month too short giving its last day; `days_until` is negative when the other date comes first. | runs |
| 14.4 | [dates-read-expanded-years.kav](dates-read-expanded-years.kav) | A year before 0 or after 9999 is read as `iso` writes it, with a sign or more digits, as ISO 8601's expanded years are. | runs |
| 14.4 | [durations-stay-within-their-range.kav](durations-stay-within-their-range.kav) | `Duration`'s `require`s keep a product within ±10^15 ms, so scaling a duration past that is a contract failure, not an overflow. | panic |
| 14.4 | [freeze-is-not-for-helpers-of-tests.kav](freeze-is-not-for-helpers-of-tests.kav) | `time.freeze` is only directly in a `test` block: in a function a test calls, it is a compile error. | compile error |
| 14.4 | [leap-years-and-month-lengths.kav](leap-years-and-month-lengths.kav) | `time.is_leap_year` and `time.days_in_month` follow the Gregorian calendar: every fourth year is a leap year, but not a century unless it divides by 400. | runs |
| 14.4 | [months-start-at-one.kav](months-start-at-one.kav) | Months are 1 to 12, never from 0: a `Date` with month 0 can't exist, so making one breaks its invariant. | panic |
| 14.4 | [parse-date-refuses-anything-else.kav](parse-date-refuses-anything-else.kav) | `time.parse_date` fails with `Invalid(text)` for anything but a date as `iso` writes it: other separators, missing digits, extra text, an impossible day. | runs |
| 14.4 | [times-as-text-have-exactly-the-digits-shown.kav](times-as-text-have-exactly-the-digits-shown.kav) | Times as text have exactly the digits shown, and a fraction of a second at least one digit; anything else fails with `Invalid`. | runs |
| 14.4 | [utc-moments-round-trip.kav](utc-moments-round-trip.kav) | `time.utc(ms)` gives the `DateTime` a moment is in UTC, with its milliseconds, and `dt.utc_ms()` gives the moment back, before 1970 too. | runs |
| 14.4, 8.1 | [waiting-needs-the-clock-capability.kav](waiting-needs-the-clock-capability.kav) | Waiting (`sleep`, `every`, `after`) needs the `Clock` capability, where reading the clock doesn't. | compile error |
| 14.4 | [zone-offset-is-only-for-named-zones.kav](zone-offset-is-only-for-named-zones.kav) | `time.zone_offset(name, ms)` gives a named zone's offset in seconds, and `none` for any other name, a fixed offset too. | runs |
| 14.5 | [math-functions.kav](math-functions.kav) | std.math has sqrt, pow, floor, ceil, round, pi and e. | runs |
| 14.5 | [math-angles-exp-and-ln.kav](math-angles-exp-and-ln.kav) | `std.math` has `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`, `exp` and `ln`, for `Float` and `Float32`. | runs |
| 14.5 | [math-roots-logs-and-gamma.kav](math-roots-logs-and-gamma.kav) | `std.math` has `log10`, `cbrt`, `hypot`, `remainder`, `erfc` and `lgamma`; `remainder` rounds to even at a tie. | runs |
| 14.5 | [math-module.kav](math-module.kav) | `std.math` has the runtime's `sqrt` and `pi` and, written in Kavach, `gcd`, `lcm`, `factorial`, `is_prime` and `mean`. | runs |
| 14.5 | [math-atan2-covers-every-quarter.kav](math-atan2-covers-every-quarter.kav) | `math.atan2(y, x)` is the angle of the point `(x, y)`, from -π to π, in each of the four quarters. | runs |
| 14.5 | [math-constants-are-full-precision-floats.kav](math-constants-are-full-precision-floats.kav) | `math.pi` and `math.e` are `Float` constants, the nearest `Float` to each number. | runs |
| 14.5 | [math-erfc-is-exact-far-out.kav](math-erfc-is-exact-far-out.kav) | `math.erfc(x)` is `1 - erf(x)`, exact even where `erf(x)` is near 1, so it does not drop to 0. | runs |
| 14.5 | [math-factorial-stops-at-20.kav](math-factorial-stops-at-20.kav) | `math.factorial(n)` is `n!` for `0 <= n <= 20`; 21! doesn't fit in `Int`, so asking for it panics. | panic |
| 14.5 | [math-hypot-does-not-vanish.kav](math-hypot-does-not-vanish.kav) | `math.hypot(x, y)` is the length of `(x, y)` without the squares vanishing on the way: tiny lengths are not 0. | runs |
| 14.5 | [math-lgamma-of-big-and-negative-numbers.kav](math-lgamma-of-big-and-negative-numbers.kav) | `math.lgamma(x)` is the natural logarithm of the absolute value of the gamma function: `ln(n!)` for big `n`, and defined where gamma is negative. | runs |
| 14.5 | [math-mean-and-primes-at-the-edges.kav](math-mean-and-primes-at-the-edges.kav) | `math.mean` of an empty list is `none`; `math.is_prime` is false for 0, 1 and negative numbers. | runs |
| 14.5 | [math-out-of-domain-gives-nan.kav](math-out-of-domain-gives-nan.kav) | `asin` and `acos` of a number past -1 to 1, and `ln` of a negative number, are `NaN`; `ln` of 0 is minus infinity. | runs |
| 14.5 | [math-remainder-differs-from-percent.kav](math-remainder-differs-from-percent.kav) | `math.remainder(x, y)` is `x - n * y` with `n` the whole number nearest `x / y`, an even one at a tie; `%` keeps `x`'s sign. | runs |
| 14.5 | [math-round-floor-and-ceil-of-negatives.kav](math-round-floor-and-ceil-of-negatives.kav) | `floor` rounds down, `ceil` up, and `round` to the nearest, a half away from zero; all three are exact. | runs |
| 14.5 | [math-sqrt-and-pow-keep-float32.kav](math-sqrt-and-pow-keep-float32.kav) | `math.sqrt` and `math.pow` of a `Float32` give a `Float32`, worked out as a `Float` and rounded once. | runs |
| 14.6 | [cli-parse-reads-options.kav](cli-parse-reads-options.kav) | cli.parse reads options into a record; fields with defaults are optional. | runs |
| 14.6 | [cli-parse-reports-missing-options.kav](cli-parse-reports-missing-options.kav) | A field with no default is a required option; cli.parse says which is missing, then the usage. | runs |
| 14.6 | [cli-parse-reads-a-command.kav](cli-parse-reads-a-command.kav) | cli.parse into a union reads the first argument as the command and the rest as its options. | runs |
| 14.6 | [cli-parse-reports-unknown-commands.kav](cli-parse-reports-unknown-commands.kav) | An unknown command is an error: the problem, then the usage of every command. | runs |
| 14.6 | [cli-parse-command-errors-show-its-usage.kav](cli-parse-command-errors-show-its-usage.kav) | A problem with a command's options gives that command's usage. | runs |
| 14.6 | [exit-ends-the-program-while-tasks-wait.kav](exit-ends-the-program-while-tasks-wait.kav) | `cli.exit(code)` ends the program at once with `code`, even while other tasks wait. | runs |
| 14.6 | [io-colors-need-a-terminal.kav](io-colors-need-a-terminal.kav) | `io.colors()` is `false` when standard output is not a terminal. | runs |
| 5.8, 14.6 | [cli-on-stop-needs-no-capability.kav](cli-on-stop-needs-no-capability.kav) | `cli.on_stop(grace)` gives a hold that isn't asked until a stop signal comes; taking and letting go of one needs no capability. | runs |
| 14.6 | [cli-args-are-the-arguments-without-the-name.kav](cli-args-are-the-arguments-without-the-name.kav) | `cli.args()` gives the program's arguments, in order, without the program's own name. | runs |
| 14.6 | [cli-args-need-env.kav](cli-args-need-env.kav) | `cli.args()` reads the process arguments, so it needs `Env`. | compile error |
| 14.6 | [cli-parse-a-command-without-fields-takes-no-options.kav](cli-parse-a-command-without-fields-takes-no-options.kav) | a variant without fields is a command without options: it parses alone, and an option after it is a `CliError`. | runs |
| 14.6 | [cli-parse-a-missing-command-shows-every-command.kav](cli-parse-a-missing-command-shows-every-command.kav) | with no command given, `cli.parse` of a union gives an error whose text is the usage of every command with its options. | runs |
| 14.6 | [cli-parse-fields-are-plain-values.kav](cli-parse-fields-are-plain-values.kav) | an option field may be `String`, a number, `Bool` or an optional of these; `cli.parse` of a record with a list field is refused. | compile error |
| 14.6 | [cli-parse-help-after-a-command-shows-its-usage.kav](cli-parse-help-after-a-command-shows-its-usage.kav) | `--help` after a command gives that command's usage (`usage: tool add-item [options]`), with its own options only. | runs |
| 14.6 | [cli-parse-help-in-place-of-a-command.kav](cli-parse-help-in-place-of-a-command.kav) | `--help` in place of a command gives an error whose text is the usage of every command with its options. | runs |
| 14.6 | [cli-parse-help-lists-every-option.kav](cli-parse-help-lists-every-option.kav) | `--help` makes `cli.parse` give an error whose text is the usage, listing every option with its type and default. | runs |
| 14.6 | [cli-parse-help-shows-a-flags-default.kav](cli-parse-help-shows-a-flags-default.kav) | In the usage, a `Bool` flag shows its default, not its type: `(default false)` when none is written. | runs |
| 14.6 | [cli-parse-needs-env.kav](cli-parse-needs-env.kav) | `cli.parse<T>()` reads the arguments, so it needs `Env`. | compile error |
| 14.6 | [cli-parse-options-cant-be-unions.kav](cli-parse-options-cant-be-unions.kav) | commands don't nest: a command's option can't be a union, so `cli.parse` of such a type is refused. | compile error |
| 14.6 | [cli-parse-reads-every-field-kind.kav](cli-parse-reads-every-field-kind.kav) | `cli.parse` reads a field `max_count` as `--max-count`, takes `String`, numbers, `Bool` flags with no value and optionals, and leaves fields not given at their defaults. | runs |
| 14.6 | [cli-parse-refuses-a-bad-value.kav](cli-parse-refuses-a-bad-value.kav) | an option whose value isn't of the field's type is a `CliError`, and the program does not run on with a made-up value. | runs |
| 14.6 | [cli-parse-refuses-an-option-without-its-value.kav](cli-parse-refuses-an-option-without-its-value.kav) | an option that needs a value but comes last, with none, is a `CliError` naming it. | runs |
| 14.6 | [cli-parse-refuses-an-unknown-option.kav](cli-parse-refuses-an-unknown-option.kav) | an option that is no field of the type is a `CliError` naming it, not silently skipped. | runs |
| 14.6 | [io-input-ends-with-none.kav](io-input-ends-with-none.kav) | at the end of standard input, `io.read_line` and `io.read_hidden` give `none`; `io.write` prints without a line end; `io.is_terminal` is `false` under a test runner. | runs |
| 14.6 | [io-needs-console.kav](io-needs-console.kav) | `std.io` reads and writes the console, so it needs `Console`. | compile error |
| 11.8, 14.7 | [test-a-failed-assert-fails-the-test.kav](test-a-failed-assert-fails-the-test.kav) | A `test` block whose `assert` is false fails the test run. | tests fail |
| 14.7 | [test-gen-int-from-above-to-fails.kav](test-gen-int-from-above-to-fails.kav) | `gen.int(from, to)` needs `from` no larger than `to`; a `sample` that asks for an empty range fails the test run. | tests fail |
| 14.7 | [test-gen-int-stays-in-its-range.kav](test-gen-int-stays-in-its-range.kav) | `gen.int(from, to)` gives an `Int` from `from` to `to`, both included, so generated values meet the `ensure`s that count on it. | tests pass |
| 14.7 | [test-gen-pick-from-no-items-fails.kav](test-gen-pick-from-no-items-fails.kav) | `gen.pick(items)` needs at least one item; a `sample` that picks from an empty list fails the test run. | tests fail |
| 14.7 | [test-gen-word-is-lowercase-letters.kav](test-gen-word-is-lowercase-letters.kav) | `gen.word()` gives a word of 1 to 8 lowercase letters; `gen.bool()` gives `true` or `false`; `gen.pick` gives one of its items. | tests pass |
| 14.7, 2.8 | [test-sample-needs-its-sample-method.kav](test-sample-needs-its-sample-method.kav) | `Sample` is an interface with `fn sample(gen: Gen) -> Self`; a type that says it implements `Sample` must have that method. | compile error |
| 14.8 | [regex-methods.kav](regex-methods.kav) | A Regex finds, captures, replaces and splits text. | runs |
| 14.8 | [regex-is-linear-time.kav](regex-is-linear-time.kav) | regex.new matches in time that grows with the text only, whatever the pattern. | runs |
| 14.8 | [lookaround-needs-advanced.kav](lookaround-needs-advanced.kav) | Lookaround and backreferences need regex.advanced. | runs |
| 14.8 | [advanced-regex-gives-up.kav](advanced-regex-gives-up.kav) | An advanced pattern that needs too much work on some text panics instead of hanging. | panic |
| 14.8 | [invalid-patterns-are-errors.kav](invalid-patterns-are-errors.kav) | An invalid pattern is an error value, not a crash. | runs |
| 14.8 | [invalid-patterns-are-errors-in-both-engines.kav](invalid-patterns-are-errors-in-both-engines.kav) | A pattern that isn't valid is an error value from `regex.new` and from `regex.advanced` alike, never a crash. | runs |
| 14.8 | [regex-captures-mark-groups-that-did-not-take-part.kav](regex-captures-mark-groups-that-did-not-take-part.kav) | `captures` gives `none` for a group that didn't take part, and `named_captures` leaves such a group out; both give `none` without a match. | runs |
| 14.8 | [regex-flags-laziness-and-unicode.kav](regex-flags-laziness-and-unicode.kav) | Patterns take the common syntax: `(?i)` ignores case, `*?` and `+?` are lazy, `{2,3}` repeats, and classes such as `\w` cover Unicode letters. | runs |
| 14.8 | [regex-replace-changes-only-the-first-match.kav](regex-replace-changes-only-the-first-match.kav) | `replace` changes only the first match and `replace_all` every one; `$1` inserts a group, and text without a match comes back as it was. | runs |
| 14.8 | [regexes-are-equal-when-their-patterns-are.kav](regexes-are-equal-when-their-patterns-are.kav) | Two `Regex` values are equal when their patterns are, even when the patterns match the same text. | runs |
| 14.9 | [text-finds-numbers.kav](text-finds-numbers.kav) | `std.text` finds numbers inside other text. | runs |
| 14.9 | [collections-module.kav](collections-module.kav) | `std.collections` makes chunks, zips two lists, keeps each item once, counts items and flattens lists. | runs |
| 14.9 | [text-normal-forms.kav](text-normal-forms.kav) | `text.nfc`, `nfd`, `nfkc` and `nfkd` give a text in a Unicode normal form. | runs |
| 14.9 | [chunks-of-size-zero-panics.kav](chunks-of-size-zero-panics.kav) | `collections.chunks` needs a `size` above 0: its contract says so, and a call that breaks it panics. | panic |
| 14.9 | [collections-module-takes-empty-lists.kav](collections-module-takes-empty-lists.kav) | `std.collections` gives empty results for empty input; `zip` stops at the shorter list, and `chunks` makes one short list for fewer items than `size`. | runs |
| 14.9 | [text-finds-numbers-after-any-letter.kav](text-finds-numbers-after-any-letter.kav) | A `-` that follows a letter or digit, in any script, is not a sign; `find_ints` and `find_float` read the numbers on each side of it. | runs |
| 14.9 | [text-finds-the-smallest-int.kav](text-finds-the-smallest-int.kav) | `find_ints` leaves out a number too big for `Int`, either way, but keeps every number that fits, the smallest `Int` too. | runs |
| 14.9 | [text-nfkd-splits-compatibility-characters.kav](text-nfkd-splits-compatibility-characters.kav) | `text.nfkd` splits accents off their letters and turns compatibility characters into plain ones; `nfd` leaves those characters as they are. | runs |
| 14.2 | [negative-counts-count-as-zero.kav](negative-counts-count-as-zero.kav) | A negative count or width counts as 0: `repeat` and `pad_left` don't panic on one, and a slice's computed negative end is before the start. | runs |
| 14.2 | [text-methods-on-empty-text.kav](text-methods-on-empty-text.kav) | `"".lines()` is `[]`; an empty part is found at once: `contains("")` is true, `index_of("")` is 0. | runs |
| 14.2 | [split-on-empty-text-panics.kav](split-on-empty-text-panics.kav) | `s.split("")` panics and says to use `chars()`. | panic |
