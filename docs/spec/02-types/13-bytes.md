# 2.13 Bytes

`Bytes` holds raw data: a file's contents, an image, a network message. It is a value like `String` (chapter 6), stored compactly, one byte per byte.

```kavach
const greeting = "hi".bytes()            // text as UTF-8
const header = Bytes([137, 80, 78, 71])  // from a list of `UInt8`
print(greeting.len())                  // 2
print(header[1])                       // 80
print(greeting)                        // Bytes(6869)
print(greeting.text() ?? "?")          // hi
```

| Written | Gives |
|---|---|
| `s.bytes()` | The text `s` as UTF-8 `Bytes` |
| `Bytes(list)` | `Bytes` from a `[UInt8]` |
| `Bytes<len: 32>` | The type of exactly 32 bytes (below) (not built yet) |
| `b.len()` | `Int`: how many bytes |
| `b.is_empty()` | `Bool`: whether there are none |
| `b[i]` | `UInt8`: byte `i`, and `b[#1]` the last (2.19); panics when `i` is out of range |
| `b.get(i)` | `UInt8?`: byte `i`, or `none` |
| `b[start:end:step]` | `Bytes`: a slice (2.19); positions past the end are clamped |
| `b.index_of(part)` | `Int?`: where the bytes `part` first appear in `b`, or `none`; an empty `part` is at 0 |
| `b.crc32c()` | The CRC32C checksum (Castagnoli, as iSCSI, ext4 and gRPC use), an `Int` in `0..2^32`; the processor computes it where it can |
| `b.xxhash64()` | The XXH64 hash (seed 0), a `UInt64`: a fast check that data didn't change by accident, as storage engines use it; not for passwords or signatures |
| `b.text()` | `String?`: the bytes as UTF-8 text, or `none` when they aren't valid UTF-8 |
| `b.text(start, end)` | `String?`: the bytes `start..end` as UTF-8 text, positions clamped as a slice clamps them, or `none` when they aren't valid UTF-8; the text is copied once, with no `Bytes` in between. `b[start:end].text()` does the same |
| `b.to_list()` | `[UInt8]` |
| `b.as_list<T>()` | `[T]?`: the bytes read as numbers of type `T`, or `none` when their length isn't a whole number of them |
| `b.read<T>(at, big_endian: false)` | `T?`: the number of type `T` whose bytes start at `at`, or `none` when they don't all fit |
| `Bytes.of<T>(n, big_endian: false)` | `Bytes`: the bytes of the number `n` of type `T` |
| `Bytes.of_list<T>(xs, big_endian: false)` | `Bytes`: the bytes of every number in the list `xs` of type `[T]`, in order, copied at once |
| `a + b` | `a` followed by `b` |
| `a == b` | Whether they hold the same bytes |

`Bytes` prints as its bytes in hexadecimal: `Bytes(6869)`.

`b.as_list<T>()` reads data that holds numbers, such as a chunk of a file (`fs.chunks`, 14.3), without going through it byte by byte. `T` is a number of fixed size: `Int8`, `Int16`, `Int32`, `Int`, `UInt8`, `UInt16`, `UInt32`, `UInt64`, `Float32` or `Float`. The bytes are little-endian, the order nearly all files and processors use, so data reads the same on every machine.

```kavach
const data = Bytes([0, 0, 128, 63, 0, 0, 0, 64])
print(data.as_list<Float32>() ?? [])     // [1.0, 2.0]
print(data.as_list<Int>() ?? [])         // [4611686019492741120]
print(data[0:3].as_list<Int32>())       // none: 3 bytes are no whole `Int32`
```

`b.read<T>(at)` reads one number where a file format or a network message puts it, and `Bytes.of<T>(n)` gives a number's bytes to build one, joined with `+`. `T` is a number of fixed size, as for `as_list`. The bytes are little-endian unless `big_endian: true`, the order network protocols and some file formats (GDSII, PNG) use. A `read` out of range gives `none`, as `b.get(i)` does; a negative `at` too.

```kavach
const header = Bytes([0, 6, 0, 2, 0, 0, 0, 1])
print(header.read<UInt16>(0, big_endian: true) ?? 0)   // 6
print(header.read<Int32>(4) ?? 0)                    // 16777216
print(header.read<UInt32>(6))                       // none: 2 bytes are left
const packet = Bytes.of<UInt16>(6, big_endian: true) + Bytes.of<Float32>(1.5)
print(packet)                                       // Bytes(00060000c03f)
```

`Bytes.of_list<T>(xs)` is the reverse of `as_list`: a whole list of numbers as bytes, to write to a file or send, copied at once instead of a `Bytes.of` a number. Each number's bytes are as `Bytes.of<T>` gives them, little-endian unless `big_endian: true`. `as_list` reads them back.

```kavach
const data = Bytes.of_list<Float32>([1.0, 2.0])
print(data)                                          // Bytes(0000803f00000040)
print(data.as_list<Float32>() ?? [])                 // [1.0, 2.0]
print(Bytes.of_list<UInt16>([1, 2], big_endian: true)) // Bytes(00010002)
```

*Not built yet:* **A fixed length: `Bytes<len: n>`** ([decision 0137](../../decisions/0137-numbers-in-types.md), 2.9). `Bytes<len: 32>` is exactly 32 bytes: a hash, a key, a field of a file format. `crypto.sha256` gives one (14.19). An index written as a number is checked while compiling.

```kavach
use std.crypto

fn key_of(text: String) -> Bytes<len: 32> {
    crypto.sha256(text.bytes())
}

const k = key_of("asha")
print(k[31])                     // the last byte
print(k[32])                     // compile error: index 32 is past `Bytes<len: 32>`
const plain: Bytes = k           // a fixed length may go where any length may
const back = Bytes<len: 32>(plain) catch e => k   // `SizeError` when `plain` isn't 32 bytes
```

- *Not built yet:* A plain `Bytes` becomes a `Bytes<len: n>` only through `try` or `catch`, which fails with a `SizeError` when the length is another (2.9).
- *Not built yet:* `Bytes([…])` of a list literal of `n` numbers is a `Bytes<len: n>`.
- *Not built yet:* `a + b` and the methods that make new bytes give a plain `Bytes`.
