# 14.31 std.font

Fonts as values ([decision 0121](../../decisions/0121-fonts-through-rustybuzz.md)): read from OpenType and TrueType files, text shaped into glyphs and their places, and glyphs drawn into an atlas the GPU can draw text from at any size. A font is data, so it needs no capability; reading its file needs `Fs.read`.

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

- A `Font` has `ascent`, `descent` and `line_gap`, each a `Float` in ems: multiplied by a size they give pixels. `ascent` is how far its letters reach above the baseline, `descent` how far below it (negative), and `line_gap` the space between lines, so lines are `(ascent - descent + line_gap) * size` pixels apart.
- A `Line` is a record of `glyphs: [Glyph]` and `width: Float`, in pixels. A `Glyph` is `id: Int`, the glyph's number in the font, and `x: Float` and `y: Float`, where its origin goes: from the start of the line, on the baseline, `y` growing downwards as on the screen.
- **Shaping** is HarfBuzz's: kerning, ligatures and the joined and reordered forms of scripts such as Devanagari and Arabic. A glyph is not a letter: `"fi"` can be one glyph, and `"क्ष"` one glyph from three letters. The script and direction are found from the text itself; right-to-left text is placed from right to left.
- An `Atlas` is `image: Image` (14.29), `size: Float`, `spread: Float` and `boxes: Dict<Int, GlyphBox>`, by glyph number. A `GlyphBox` is `x`, `y`, `width` and `height`, where the glyph is in the image, and `left` and `top`, where the box goes from the glyph's origin: `left` pixels to the right and `top` pixels above the baseline, all `Int`. A glyph with nothing to draw, as a space, has no box.
- **Distances.** Each pixel of an atlas holds how far its centre is from the glyph's edge, in all four parts: 128 on the edge, more inside, up to 255 at `spread` pixels in, and less outside, down to 0 at `spread` pixels out. `spread` is an eighth of `size`, at least 2. A shader that draws the pixels above 128 draws the glyph sharp at any scale, so one atlas made at 48 draws text at 32 or 96: multiply the boxes by the size wanted over `atlas.size`.
- `FontError` is `BadFont(reason: String)`: the bytes aren't a font file, or the file is cut or damaged.
- A `size` outside 1 to 1,000 is a panic (a contract), and so is an atlas that would be wider or taller than 4,096 pixels.

## Open questions

- Breaking text into lines, mixed directions in one line (bidi), falling back to another font for a missing glyph, and colour emoji: packages, or later with their own decision.
- Variable fonts' axes (weight, width), and OpenType features chosen by the program (small capitals, tabular figures).
