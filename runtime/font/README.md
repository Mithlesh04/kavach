# runtime/font/

Fonts for `std.font` (spec 14.31): OpenType and TrueType files read through `ttf-parser`, text shaped through `rustybuzz`, and glyphs drawn as distances into an atlas by our own code (decision 0121). Apart from `native/`, so a program without fonts doesn't link the shaper (decision 0024).

| File | Job |
|---|---|
| `Cargo.toml` | The crate `kavach-runtime-font`: `rustybuzz`, which brings `ttf-parser` |
| `src/lib.rs` | `metrics`, `shape` (glyphs placed in pixels), `atlas`, and atlases kept by number for `std.font`: `atlas_kept`, `atlas_boxes`, `take_atlas_pixels` |
| `src/sdf.rs` | A glyph's outline cut into lines, each pixel's signed distance to it, and the glyphs packed into one image in shelves |
| `src/sample.rs` | A tiny TrueType font made by our own code, for the tests |
| `src/sample.ttf` | That font as a file, for `tools/cli/tests/fonts.rs`; written again by the tests when `sample.rs` changes |
| `src/tests.rs` | The sample font's metrics, shaping, distances (a hole is outside) and what isn't a font |
