# runtime/font/

Fonts for `std.font` (spec 14.31): OpenType and TrueType files read through `ttf-parser`, text shaped through `rustybuzz`, and glyphs drawn as distances into an atlas by our own code (decision 0121). Apart from `native/`, so a program without fonts doesn't link the shaper (decision 0024).

| File | Job |
|---|---|
| `Cargo.toml` | The crate `kavach-runtime-font`: `rustybuzz`, which brings `ttf-parser` |
| `src/lib.rs` | `metrics`, `shape` (glyphs placed in pixels, with their clusters and advances), `atlas` and `atlas_add`, and atlases kept by number for `std.font`: `atlas_kept`, `keep_base` (an atlas a program holds, to add to), `atlas_boxes`, `take_atlas_pixels` |
| `src/sdf.rs` | A glyph's outline cut into lines, each pixel's signed distance to it, and the glyphs packed into one image in shelves, or below an atlas's image |
| `src/coverage.rs` | Glyphs drawn for one size, each pixel how much the glyph covers; a colour glyph of a `COLR` font in its colours, layer over layer |
| `src/system.rs` | The fonts installed on the machine, from the system's font folders, each file's `name` table read alone; and a fallback for text, from their `cmap` tables |
| `src/sample.rs` | A tiny TrueType font made by our own code, for the tests, with a colour glyph and its names |
| `src/sample.ttf` | That font as a file, for `tools/cli/tests/fonts.rs`; written again by the tests when `sample.rs` changes |
| `src/tests.rs` | The sample font's metrics, shaping (clusters and advances), distances (a hole is outside), coverage, its colour glyph, glyphs added to an atlas, the font found as installed, and what isn't a font |
