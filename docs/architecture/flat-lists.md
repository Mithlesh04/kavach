# Flat lists

How native Kavach stores a list of plain records or of text, and why. The code is in [runtime/native/src/flat.rs](../../runtime/native/src/flat.rs) and [compiler/backend/cranelift/src/flat.rs](../../compiler/backend/cranelift/src/flat.rs). The rule for which lists are flat is `flat_width` in [compiler/kir/src/ir.rs](../../compiler/kir/src/ir.rs), shared by both.

## The problem

Every heap value is an object with a 24-byte header. A list of records normally holds a pointer to one object per record:

```
points: [ptr0 ptr1 ptr2 …]      8 bytes per item
          │
          └─> [header 24 bytes | x | y]   40 bytes per Point
```

So 10 million points take 480 MB, and reading `p.x` follows a pointer to memory far from the last one. C++'s `vector<Point>` stores the fields themselves: 160 MB, read in order.

## The design

A list is **flat** when its items are records or tuples whose fields are all scalars (integers, floats, booleans, characters), with at most 8 fields, and of a type that doesn't implement `Drop` (each `Drop` value is an object of its own, so it can wait for its `drop`, spec 6.1). Its buffer holds the fields directly, `width` words per item:

```
points: [x0 y0 x1 y1 x2 y2 …]   16 bytes per Point, no objects
```

- The list object is the same as for other lists. Its length and capacity count items, so `len()` and bounds checks don't change.
- Records have no identity in Kavach (spec 2.4): two records with the same fields are the same value. So a record can be copied out of a list, or into one, without anyone noticing.

### In generated code

| Code | What runs |
|---|---|
| `points[i].x` | One load at `buffer + (i × width + field) × 8`, after the bounds check |
| `for p in points { … p.x … }` | `p` is a *view*: the item's address minus the header size, so field reads work unchanged. A view is never retained or released. |
| `points.push(Point(x: 1, y: 2))` | The fields are stored at the end of the buffer; no object is made |
| `const p = points[i]`, or `p` used whole in a loop | The item is *boxed*: copied into a new record object |
| `points[i] = p`, `points.push(p)` | The fields of `p` are copied in, and `p` is released |

### In the runtime

Generic runtime code (printing, comparing, hashing, sorting, `reverse`, `take`, …) works on objects. For a flat list:

- `items_of` boxes every item into an object, and releases them when it is dropped.
- `with_slots` boxes the items, lets the caller change them as a `Vec`, then unboxes them back into the buffer.
- `alloc_slots` takes objects and unboxes them into a new flat list.

Boxing costs one allocation per item, so the hot operations (`push`, `list[i] = v`) work on the fields directly instead.

## Lists of text

A list of text is flat too, with two words (16 bytes) per item, like C++'s `std::string` with its short-string buffer:

```
short:  [w o r d 4 2 0 0 | 0 0 0 0 0 0 0 tag]   tag = inline, ASCII, length 6
long:   [pointer to a text object | 0]
```

- Text up to 15 bytes is stored in the item. The last byte is a tag: inline, ASCII, and the length.
- Longer text is a pointer to an ordinary text object, which the list holds a reference to.
- `split` and `lines` write short parts straight into the items: splitting a text into a million words makes no objects.
- `join` copies each short item's 16 bytes at once and moves past its length.
- `w.len()` on a `for` variable that is only asked its length reads the tag: no object, no call, for ASCII text.
- `words.push("w{i}")` builds the text in a 16-byte item on the stack (`kv_item_*`) and copies the two words into the list: no object for short text. The LLVM backend does this; Cranelift still builds a text object first.
- Reading an item as a whole value (`words[i]`, or a `for` variable used another way) boxes it into a text object; a long item's object is just retained.

The code is in [runtime/native/src/text_items.rs](../../runtime/native/src/text_items.rs) and [compiler/backend/cranelift/src/text_lists.rs](../../compiler/backend/cranelift/src/text_lists.rs).

## Results

Times in [benchmarks/loops](../../benchmarks/loops/README.md).

| Benchmark | Before | After |
|---|---|---|
| `records`: 10 million points, then sum `p.x * p.y` | 1.03 (476 MB) | **0.45** (164 MB) |
| `text`: build, join and split a million words | 1.51 | **0.62** |

## Open questions

- **A reused object for `for` variables.** A `for` variable over a list of text that is used as a whole is boxed for every item. The loop could keep one object and refill it while nothing else holds it.
- **Records with text fields.** They hold references, so they are not flat. A layout that keeps the scalar fields inline and the text as pointers would still save the per-item object.
- **Nested flat records.** A record holding another plain record could be flattened into one row of words.
