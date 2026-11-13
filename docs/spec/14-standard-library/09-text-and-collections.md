# 14.9 std.text and std.collections

Written in Kavach, with contracts and examples like any other code.

| Function | Gives |
|---|---|
| `text.pad_left(s, width, fill)`, `text.pad_right(s, width, fill)` | `s` filled with `fill` (a `Char`, default `' '`) up to `width` characters |
| `text.words(s)` | The words of `s`: the parts between runs of spaces, tabs and line breaks |
| `text.capitalize(s)` | `s` with its first character in upper case |
| `text.truncate(s, max)` | At most `max` characters of `s` |
| `text.is_blank(s)` | Whether `s` has only spaces, tabs and line breaks |
| `text.find_ints(s)` | Every whole number written inside `s`, in order: `"a12b34"` gives `[12, 34]` |
| `text.find_int(s)` | The first of them, or `none`: `"sd23"` gives `23` |
| `text.find_float(s)` | The first number with its fraction, or `none`: `"pi is 3.14!"` gives `3.14` |
| `text.nfc(s)`, `text.nfd(s)`, `text.nfkc(s)`, `text.nfkd(s)` | `s` in a Unicode normal form (UAX #15), so texts written with different code points compare equal: `nfc` joins a letter and its accent into one code point, `nfd` splits them, and the `k` forms also turn compatibility characters into plain ones (`ﬁ` into `fi`, full-width `１` into `1`). The runtime gives these four ([decision 0102](../../decisions/0102-unicode-normalization.md)); the rest of `std.text` is written in Kavach |
| `collections.chunks(items, size)` | The items in lists of `size` (the last may be shorter) |
| `collections.zip(a, b)` | Pairs of items at the same position, as long as the shorter list |
| `collections.unique(items)` | Each item once, in the order first seen |
| `collections.counts(items)` | How many times each item appears, in the order first seen |
| `collections.flatten(lists)` | The items of all the lists, in order |

**Finding numbers in text.** `Int(s)` reads the whole text strictly and fails on anything else (2.1). The `find_` functions look for numbers *inside* other text instead, by these rules, so the result never depends on guesswork:

- A number is a run of the digits `0` to `9`. `find_float` also takes one `.` followed by more digits.
- A `-` just before the digits is its sign, unless that `-` follows a letter or digit: `"-5 °C"` is `-5`, but `"item-3"` is `3` and `"10-20"` is `10` and `20`.
- A comma or any other mark ends a number: `"₹1,299"` gives `1` and `299`. Digit groups are never joined, since `,` means a thousands separator in one place and a decimal point in another.
- A whole number too big for `Int` is left out.

```kavach
use std.text

const total = text.find_int("Total: 42 items") ?? 0    // 42
const all = text.find_ints("rooms 12, 14 and 15")       // [12, 14, 15]
```

