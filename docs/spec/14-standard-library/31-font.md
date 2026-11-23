# 14.31 std.font

Fonts as values ([decision 0121](../../decisions/0121-fonts-through-rustybuzz.md)): read from OpenType and TrueType files, text shaped into glyphs and their places, and glyphs drawn into an atlas the GPU can draw text from at any size. A font is data, so it needs no capability; reading its file needs `Fs.read`. The fonts installed on the machine are found with `System` (14.31.1).

```kavach
use std.font
use std.fs
use std.gpu

const sans = try font.load(try fs.read_bytes("NotoSans.ttf"))
const line = sans.shape("Hello, नमस्ते", 32.0)
print(`{line.glyphs.len()} glyphs, {line.width} pixels wide`)
const atlas = sans.atlas("Hello, नमस्ते", 48.0)
const letters = try gpu.texture(atlas.image.width, atlas.image.height, atlas.image.pixels)
```

| Function or method | Gives |
|---|---|
| `font.load(bytes) -> Font ! FontError` | The font in an OpenType or TrueType file's bytes (`.ttf`, `.otf`; the first font of a `.ttc`) |
| `f.shape(text, size) -> Line` | `text` shaped at `size` pixels to the em: its glyphs, each with its place, and its width |
| `f.atlas(text, size) -> Atlas` | An image holding each glyph that shaping `text` uses, drawn at `size` pixels to the em as distances (below) |
| `f.atlas(text, size, exact: true) -> Atlas` | The same, each glyph drawn for `size` only, as how much of each pixel it covers (below) |
| `atlas.add(f, text) -> Atlas` | The atlas with each glyph of `text` it lacks, drawn as its own were, below them: its own glyphs stay where they are |

- A `Font` has `ascent`, `descent` and `line_gap`, each a `Float` in ems: multiplied by a size they give pixels. `ascent` is how far its letters reach above the baseline, `descent` how far below it (negative), and `line_gap` the space between lines, so lines are `(ascent - descent + line_gap) * size` pixels apart.
- A `Line` is a record of `glyphs: [Glyph]` and `width: Float`, in pixels. A `Glyph` is `id: Int`, the glyph's number in the font; `x: Float` and `y: Float`, where its origin goes: from the start of the line, on the baseline, `y` growing downwards as on the screen; `cluster: Int`, the character of `text` its cluster starts at; and `advance: Float`, how far it moves the pen to the right, in pixels.
- **Clusters.** A cluster is the smallest part of the text that maps to whole glyphs: `"fi"` drawn as one glyph is one cluster of two characters, and `"क्ष"` one of three. Each glyph of a cluster has the cluster's first character, counted from 0 as `s.len()` counts (14.2). So a click on a glyph finds its place in the text, and a caret goes between clusters, never inside one. Left-to-right text gives clusters in rising order; right-to-left text gives them falling, as its glyphs are placed from the right.

```kavach
const line = sans.shape("office", 32.0)
for g in line.glyphs {
    print(g.cluster)                 // 0, 1, 4, 5 with a font that joins "ffi" into one glyph
}
```
- **Shaping** is HarfBuzz's: kerning, ligatures and the joined and reordered forms of scripts such as Devanagari and Arabic. A glyph is not a letter: `"fi"` can be one glyph, and `"क्ष"` one glyph from three letters. The script and direction are found from the text itself; right-to-left text is placed from right to left.
- An `Atlas` is `image: Image` (14.29), `size: Float`, `spread: Float`, `boxes: Dict<Int, GlyphBox>`, by glyph number, and `exact: Bool`. A `GlyphBox` is `x`, `y`, `width` and `height`, where the glyph is in the image, and `left` and `top`, where the box goes from the glyph's origin: `left` pixels to the right and `top` pixels above the baseline, all `Int`; and `color: Bool`, true for a colour glyph (below). A glyph with nothing to draw, as a space, has no box.
- **Distances.** Each pixel of an atlas holds how far its centre is from the glyph's edge, in all four parts: 128 on the edge, more inside, up to 255 at `spread` pixels in, and less outside, down to 0 at `spread` pixels out. `spread` is an eighth of `size`, at least 2. A shader that draws the pixels above 128 draws the glyph sharp at any scale, so one atlas made at 48 draws text at 32 or 96: multiply the boxes by the size wanted over `atlas.size`.
- **Exact atlases.** Distances blur at 10 to 14 pixels, where most of a user interface's text is. `f.atlas(text, size, exact: true)` draws each glyph for `size` only: each pixel holds how much of it the glyph covers, from 0 (none) to 255 (all), in all four parts, from 64 points in the pixel. Drawn at that size on whole pixels (`left` and `top` from a rounded origin), its glyphs stay sharp. `spread` means nothing in it.
- **Colour glyphs.** In an exact atlas, a glyph the font gives in colour (an emoji of a font with a `COLR` table, as Segoe UI Emoji has) is drawn in its colours. Its box's `color` is true, and its pixels are red, green, blue and opacity, the opacity how much it covers. A shader draws such a box's pixels as they are, and another box's in the text's colour. In a distance atlas, every glyph is its outline.
- **Adding glyphs.** `atlas.add(f, text)` draws each glyph of `text` that `atlas` has no box for, as `atlas` was drawn (its `size`, and `exact` or not), in new rows below its image. The image grows taller, and stays as wide, or grows as wide as the widest new glyph; every old box stays where it was. So a text field's atlas grows as letters are typed, and a texture made from it (14.24) needs only the new rows sent again. An atlas holds one font's glyphs: their numbers are that font's. An atlas taller than 4,096 pixels is a panic, as a new one is: make a new atlas then.
- `FontError` is `BadFont(reason: String)`: the bytes aren't a font file, or the file is cut or damaged.
- A `size` outside 1 to 1,000 is a panic (a contract), and so is an atlas that would be wider or taller than 4,096 pixels.

## 14.31.1 Installed fonts

A program can use the fonts the machine has, instead of shipping each one, and find one that has the glyphs another font lacks: a fallback. Kavach shapes and draws them as any font (decision 0121), so text looks the same on every system; the system is only asked where its fonts are.

```kavach
use std.font

fn fonts() ! FontError uses System, Console {
    for f in font.installed().filter(f => f.family == "Segoe UI") {
        print(`{f.family} {f.style}`)           // Segoe UI Regular, Segoe UI Bold, …
    }
    if let hindi = font.fallback("नमस्ते") {
        print(hindi.family)                    // Nirmala UI on Windows
        const devanagari = try hindi.load()
        print(devanagari.shape("नमस्ते", 16.0).width)
    }
}
```

| Function or method | Gives |
|---|---|
| `font.installed() -> [InstalledFont]` | The fonts installed on the machine, by family, then style |
| `font.fallback(text) -> InstalledFont?` | An installed font with a glyph for each character of `text` (spaces and controls aside), or `none` |
| `installed.load() -> Font ! FontError` | The installed font, read from its file |

- An `InstalledFont` is `family: String` (`Segoe UI`), `style: String` (`Regular`, `Bold`), from the font's own names, in English where it has them, and `path: String`, its file.
- They need `System` (8.1), as the machine's other details do (14.25). `installed.load()` reads only an installed font's file, so it needs no `Fs.read`.
- The fonts are those in the system's font folders: on Windows the `Fonts` folder of Windows and the user's own; on Linux `/usr/share/fonts`, `/usr/local/share/fonts`, `~/.local/share/fonts` and `~/.fonts`; on macOS `/System/Library/Fonts`, `/Library/Fonts` and `~/Library/Fonts`. A file holding several fonts (`.ttc`) gives its first, as `font.load` reads it. They are looked for once a run.
- `fallback` tries first the families each system's own text falls back to (on Windows: Segoe UI, Segoe UI Symbol, Segoe UI Emoji, Nirmala UI, Microsoft YaHei, Yu Gothic, Malgun Gothic and others), then the rest by family; each family's regular style before its others. A machine with no fonts installed, as a container, gives `[]` and `none`.

## Open questions

- Breaking text into lines and mixed directions in one line (bidi): packages, or later with their own decision.
- For a maintainer's review (14.31, 14.31.1): installed fonts are found in the font folders, not through each system's font service (fontconfig, DirectWrite, Core Text), so no new dependency is needed; such a service, with its own fallback lists, would need a decision record. Installed fonts need `System`. Exact atlases are not hinted. Colour glyphs' gradients (`COLR` version 1) are drawn in the mean of their colours, and their blend modes as plain layering; colour glyphs that are pictures (`sbix`, `CBDT`) are drawn as their outline, or not at all.
- Variable fonts' axes (weight, width), and OpenType features chosen by the program (small capitals, tabular figures).
