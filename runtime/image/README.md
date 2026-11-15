# runtime/image/

Images for `std.image` (spec 14.29): PNG and JPEG files decoded to RGBA pixels, and PNG written, through `png` and `jpeg-decoder` (decision 0117). Apart from `native/`, so a program without images doesn't link the decoders (decision 0024).

| File | Job |
|---|---|
| `Cargo.toml` | The crate `kavach-runtime-image`: `png`, and `jpeg-decoder` without its threads |
| `src/lib.rs` | `decode` (any PNG or JPEG to 8-bit RGBA), `png`, and pictures kept by number for `std.image`: `decode_kept`, `size`, `take_pixels` |
| `src/tests.rs` | A PNG written and read back, JPEG files of a known color, and what isn't an image refused |
| `src/red.jpg`, `src/grey.jpg` | 8 × 8 JPEG files of one color, made with ImageMagick, for the tests |
