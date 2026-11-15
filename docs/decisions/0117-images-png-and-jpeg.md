# 0117. Images in `std.image`: PNG and JPEG, as RGBA pixels

- **Status:** Accepted
- **Date:** 2026-11-16

## Context

Games load textures, tools read screenshots, and the clipboard and a camera give pictures (to-do items 6 and 9). Kavach has no image value: `gpu.texture` (14.24) takes pixels, but nothing reads an image file into them. This is the first half of language request 9 (6); fonts and text shaping come later. A maintainer chose it for this round (2026-11-16).

## Decision

- **`std.image`** has one value, `Image`: its width, its height and its pixels, four `UInt8`s each (red, green, blue, opacity), row by row from the top. That is what `gpu.texture` takes, so `gpu.texture(img.width, img.height, img.pixels)` shows a file on the GPU.
- `image.decode(bytes)` reads **PNG** (through the `png` crate) and **JPEG** (through `jpeg-decoder`), told apart by their first bytes. Both crates are pure Rust, MIT or Apache-2.0. Any image is turned into 8-bit RGBA: grey, colors from a palette, 16-bit samples and missing opacity all become that.
- `image.png(img)` writes an image as PNG, for screenshots and generated pictures.
- An image is data, so reading or writing one needs no capability; reading the file it is in needs `Fs.read`, as always.
- The decoders live in a crate of their own (`runtime/image`), so a program without images links none of them (decision 0024).

## Consequences

- Textures, icons and screenshots work with a few calls.
- Compressed GPU textures (KTX2, BCn, ASTC), more formats (GIF, WebP) and fonts are left for later, as packages or with their own decisions.
- `jpeg-decoder` is used without its threads (`rayon`), so decoding stays on the calling task.

## Alternatives considered

- **The `image` crate:** many formats at once, but large, with many dependencies. Rejected: two small decoders cover what games and tools load most.
- **Formats only in packages:** every program wanting a texture would need one first. Rejected: PNG and JPEG are what nearly everything uses.
