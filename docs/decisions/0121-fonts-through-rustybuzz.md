# 0121. Fonts in `std.font`: loaded by `ttf-parser`, shaped by `rustybuzz`

- **Status:** Accepted
- **Date:** 2026-11-16

## Context

Games, tools and charts draw text on the GPU (to-do 9 (6)). A font file holds the outlines of glyphs, not of letters: turning text into glyphs and their places is *shaping*. Latin text needs it for kerning and ligatures; Hindi, Arabic and many other scripts can't be shown without it. Shaping well is a large job that HarfBuzz has done for decades. Kavach has `std.image` and `gpu.texture` (14.24, 14.29), but no way to read a font. A maintainer chose this design on 2026-11-16, from options with a recommendation.

## Decision

- **`std.font`** reads OpenType and TrueType files (`.ttf`, `.otf`) through the **`ttf-parser`** crate and shapes text through **`rustybuzz`**, a pure-Rust port of HarfBuzz. Both are pure Rust, MIT or Apache-2.0, and `ttf-parser` is already in the build through `winit`.
- **Glyphs are drawn by Kavach's own code** into a **signed distance field (SDF) atlas**: one image holding each glyph a text needs, where each pixel tells how far it is from the glyph's edge. A GPU shader then draws the text sharp at any size from one atlas. The atlas is an `Image` (14.29), so `gpu.texture` takes it as it is.
- A font is data, so it needs no capability; reading its file needs `Fs.read`.
- The font code lives in a crate of its own (`runtime/font`), so a program without fonts links none of it (decision 0024).
- Drawing text on the screen (a pipeline, its shaders, layout of many lines) is a package's job, as renderers are (decision 0106).

## Consequences

- Text in any script HarfBuzz supports can be shaped and drawn by a few calls.
- Line breaking, mixed directions in one line (bidi), font fallback and colour emoji are not in `std.font`; they are open questions of spec 14.31.
- The atlas is made on the CPU when it is asked for; a game makes it once, at loading.

## Alternatives considered

- **`cosmic-text`:** shaping, fallback, line breaking and bidi in one crate. Rejected: large, with its own layout and bitmap rendering, where Kavach wants SDF glyphs and layout in packages.
- **`fontdue`:** loads fonts and draws glyphs, but doesn't shape. Rejected: Hindi or Arabic text would come out wrong.
- **Fonts only in packages:** a font parser and shaper written in Kavach. Rejected: years of work to match HarfBuzz, and every program showing text would need it.
