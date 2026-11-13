# 14.21 std.compress

Bytes made smaller, and made whole again, in the formats programs exchange: gzip and deflate (HTTP, PNG, zip files), zstd and brotli (HTTP, packages), and lz4 (game assets and packets, where speed matters most). It is built into the runtime and needs no capability: it changes nothing outside the program (8.1). The formats come from Rust crates ([decision 0103](../../decisions/0103-std-compress.md)).

| Function | Gives |
|---|---|
| `compress.encode(data, format, level: none) -> Bytes` | `data` (`Bytes`) compressed in `format`, at `level`, or the format's usual level when `none` |
| `compress.decode(data, format, most: 64_000_000) -> Bytes ! CompressError` | The bytes `data` holds, which must be in `format`; at most `most` of them |
| `compress.level_fits(format, level) -> Bool` | Whether `level` is one of `format`'s levels (below); `none` always is |
| `compress.from_name(name) -> Format?` | The format a name in an HTTP header stands for: `gzip` (or `x-gzip`), `deflate`, `zstd`, `br` or `lz4`, in any case; `none` for any other |
| `format.name() -> String` | The format's name in a header: `Format.Brotli.name()` is `br` |

`Format` is `Gzip | Deflate | Zstd | Brotli | Lz4`. Each format has its own levels; a higher one is smaller and slower:

| `Format` | Levels | Usual level |
|---|---|---|
| `Gzip`, `Deflate` | 0 (stored, not compressed) to 9 | 6 |
| `Zstd` | 1 to 22 | 3 |
| `Brotli` | 0 to 11 | 5 |
| `Lz4` | one level: `level` must be `none` or 0 | 0 |

A level outside its format's range breaks `encode`'s contract, `require level_fits(format, level)` (7.1), so it panics: it is a mistake in the program, not in the data.

**Decoding stops at `most` bytes,** 64 MB unless the call says otherwise. A small input can expand to gigabytes (a "zip bomb"); past `most`, decoding fails with `TooLarge` instead of filling memory. The errors:

| `CompressError` | When |
|---|---|
| `TooLarge(most: Int)` | The data holds more than `most` bytes |
| `Corrupt(reason: String)` | The data isn't in `format`, or was cut short |

```kavach
use std.compress

const packed = compress.encode(body, Format.Zstd)
const back = try compress.decode(packed, Format.Zstd)

// A server answering in the format the client asked for.
const format = compress.from_name("br") ?? Format.Gzip
const answer = compress.encode(page, format, level: 4)

// A request body, refused when it expands past 1 MB.
const sent = compress.decode(r.body, Format.Gzip, most: 1_000_000) catch e => match e {
    CompressError.TooLarge(_) => return http.text(413, "too large"),
    CompressError.Corrupt(why) => return http.text(400, "not gzip: {why}"),
}
```

- gzip data made of several members one after another decodes as one, as `gunzip` does. lz4 is its frame format (`.lz4` files), not raw blocks.
- `compress.encode_bytes(data, format, level)` and `compress.decode_bytes(data, format, most)` are what `encode` and `decode` are built on, with the format as its number (`Gzip` 0 to `Lz4` 4) and every argument given.
