# 14.24 std.gpu

Drawing with the GPU in a window (14.22): a thin typed layer over what GPUs have, with shaders written in Kavach (15.3). A renderer, a scene graph or a game framework is a package built on it ([decision 0106](../../decisions/0106-drawing-on-the-gpu.md)). Drawing is a change to the program's own windows, so it needs `Window` (8.1).

```kavach
use std.window
use std.gpu

fn main() ! Error uses Window {
    const win = try window.open("Hello", 640, 480)
    const screen = try gpu.screen(win)
    let open = true
    while open {
        for e in win.poll() {
            match e {
                Event.Closed => {
                    open = false
                },
                _ => {},
            }
        }
        const frame = try screen.frame()
        frame.clear(0.1, 0.2, 0.3)
        frame.show()
    }
}
```

| Function or method | Gives |
|---|---|
| `gpu.screen(win) -> Screen ! GpuError` | What draws into the window `win`, using the program's GPU |
| `gpu.screen(win, samples: 4) -> Screen ! GpuError` | The same, each pixel drawn from 4 samples, so edges are smooth (14.24.1) |
| `screen.frame() -> Frame ! GpuError` | The next picture to draw, as big as the window is now. It waits for the screen's next refresh when the last picture isn't shown yet (vsync), so a loop of frames runs at the screen's rate |
| `frame.clear(r, g, b)` | Fills the picture with one color; each part is a `Float32` from 0 to 1 |
| `frame.draw(pipeline, vertices)` | Draws `vertices` through `pipeline` (15.3) |
| `frame.draw(pipeline, vertices, instances: items)` | The same once for each item of `items`, which the vertex fn gets as its second parameter |
| `frame.draw(pipeline, vertices, instances: items, shared: value)` | The same, `value` given to every vertex as the vertex fn's last parameter |
| `frame.draw(pipeline, vertices, textures: [t])` | The same, the fragment fn reading the textures `t` (15.3) |
| `frame.draw(pipeline, vertices, indices: corners)` | Triangles of the vertices that `corners`, a `[UInt32]`, names by their place in `vertices`: each three make a triangle, so a corner two triangles share is one vertex |
| `frame.show()` | Puts the picture on the screen |
| `gpu.pipeline(vertex_fn, fragment_fn) -> Pipeline<V, I, S, T>` | A pipeline drawing triangles with a `gpu vertex fn` and a `gpu fragment fn` (15.3). `V`, `I` and `S` are what the vertex fn takes; `NoInput`, an empty record, stands for an instance or shared value it doesn't take. `T` is `Texture` when the fragment fn takes textures, and `NoInput` when not. It is compiled for the GPU when it is made, or when a screen opens, never in the middle of a frame |
| `gpu.pipeline(vertex_fn, fragment_fn, depth: true)` | The same, drawing a pixel only where it is nearer than what is there (15.3) |
| `gpu.texture(width, height, pixels) -> Texture ! GpuError` | A picture for fragment fns to read (15.3): `pixels` is a `[UInt8]`, four for each pixel (red, green, blue and opacity, in sRGB), row by row from the top. Another length than `width * height * 4` is a panic |
| `gpu.texture(width, height, pixels, channels: 1)` | A texture of one byte a pixel: a value from 0 to 1, not a colour, as a glyph atlas's coverage (14.24.1) |
| `gpu.texture(width, height, pixels, nearest: true)` | A texture sampled to the nearest pixel, not blended between pixels (14.24.1) |
| `gpu.update(t, x, y, width, height, pixels)` | Changes part of texture `t`, an `inout` parameter (3.9): from pixel `x`, `y`, `width` × `height` pixels (14.24.1) |
| `gpu.compressed_texture(file) -> Texture ! GpuError` | A texture of the compressed blocks in a KTX2 file's bytes, given to the GPU as they are, mip levels and all: 4 to 8 times less of its memory than `gpu.texture`'s pixels. The blocks are BC1 to BC7, which desktop GPUs read, or ASTC 4×4, which phones read; width and height are each a multiple of 4. A file that isn't such a KTX2 file is `BadTexture`; a GPU that can't read its blocks is `Unavailable`. Only the GPU reads it, so `sample` on the CPU is a panic (a contract) |
| `texture.width`, `texture.height: Int` | Its size in pixels |
| `texture.sample(u, v) -> (Float32, Float32, Float32, Float32)` | The texture's color at `u` (0 left, 1 right) and `v` (0 top, 1 bottom), blended between the nearest pixels; in a `gpu fragment fn` and on the CPU alike |
| `gpu.devices() -> [GpuDevice]` | Each GPU the program could use, the best first, as `system.gpus()` lists them (14.25): `GpuDevice(index:, name:, vendor:, kind:, backend:)`, `index` its number from 0, the rest as `GpuInfo`'s. Needs `Gpu` |
| `gpu.on(device) { … }` | The block, with its GPU work on `device` (15.6): a block given to a call (4.8). Needs `Gpu` |

- `GpuError` is `Unavailable(reason: String)`: there is no GPU, or the window can't be drawn into, as on a server with no screen; or `BadTexture(reason: String)`: a KTX2 file that can't be read. A frame that can't be had (the window is minimized, or its surface was lost) is `Unavailable` too; the next `frame()` tries again.
- A frame not shown before its last holder ends is thrown away: nothing reaches the screen.
- The vertices are a list of a record type; without `indices:`, each three make a triangle. An index past the last vertex is a panic, before anything is drawn. A draw draws over what the frame already holds, blending by opacity.
- A list that a GPU `map` made, and that is still in GPU memory (15.2), is drawn from there with nothing copied, as vertices or as instances: a particle swarm moved by a `map` each frame is drawn where it is, and never comes back to the CPU.
- `frame.draw` without `instances:` or `shared:` that its pipeline's vertex fn takes is a compile error.
- A texture's width and height are each 1 to 8192 pixels.
- Choosing a GPU (15.6) needs `Gpu`, not `Window`: `gpu.devices()` and `gpu.on` draw nothing. `GpuKind` is `std.system`'s.
- `gpu.screen_id`, `frame_id`, `clear_id`, `show_id`, `state_id`, `canvas_id`, `canvas_pixels_id`, `canvas_texture_id`, `texture_id`, `update_id` and `close_id` are what these are built on, by number.
- Generated tests (11.5) skip functions that use `Window`: there is no fake screen yet.

## 14.24.1 Drawing for user interfaces

A user interface draws thousands of small shapes a frame: rectangles with rounded corners, text from a glyph atlas (14.31), pictures. It needs to clip them to a scrolled box, draw into a picture to reuse, and keep a glyph atlas growing as text is typed. These are what `std.gpu` gives for it.

```kavach
use std.gpu

fn panel(frame: Frame, ui: Ui) uses Window {
    frame.set_clip(ui.x, ui.y, ui.width, ui.height)   // the scrolled box
    frame.set_mask(Mask.Write)
    frame.draw(ui.rounded, ui.box)                   // marks the box's rounded shape
    frame.set_mask(Mask.Inside)
    frame.draw(ui.text, ui.quads, textures: [ui.atlas])   // only inside it
    frame.set_mask(Mask.Off)
    frame.clear_clip()
}
```

**How a frame draws.** A frame keeps a state that every draw after it follows, until it is changed; a new frame starts with none of it:

| Method | From now on |
|---|---|
| `frame.set_clip(x, y, width, height)` | Draws change only the pixels inside this rectangle, in pixels from the frame's top left. A rectangle past the frame's edges is cut to them; one of no pixels draws nothing |
| `frame.clear_clip()` | Draws change any pixel again |
| `frame.set_premultiplied(on)` | Draws blend their colours as premultiplied: the fragment fn's red, green and blue are already multiplied by its opacity, as a picture of soft edges keeps them. Off at first: a colour is blended by its opacity, as before |
| `frame.set_mask(mask)` | Draws use the frame's mask: `Mask.Off`, not at all (at first); `Mask.Write`, a draw marks the pixels where its fragment fn gives an opacity above 0, and colours none; `Mask.Inside`, it draws only on marked pixels; `Mask.Outside`, only on others |
| `frame.clear_mask()` | Unmarks every pixel. `frame.clear` unmarks them too |

- The mask is one mark a pixel: marking an area twice marks it once. So a box with rounded corners clips its contents the same way a rectangle does with `set_clip`, which is cheaper.
- `set_clip` with a negative number is a panic (a contract).
- A pipeline is compiled ahead in its usual form (14.24). Its first draw with premultiplied blending or a mask compiles that form then: draw it once while a program loads, so no frame waits for it.

**Smooth edges.** `gpu.screen(win, samples: 4)` and `gpu.canvas(width, height, samples: 4)` draw each pixel from 4 samples, which are blended when the frame is shown, so a triangle's edge is smooth instead of stepped. It costs the GPU about four times the memory for the picture. Any other number than 1 or 4 is a panic (a contract). A picture's depths and mask have the same samples.

**Canvases.** A canvas is a picture off the screen, drawn as a window is, then read back or drawn from:

| Function or method | Gives |
|---|---|
| `gpu.canvas(width, height) -> Canvas ! GpuError` | A canvas `width` × `height` pixels (1 to 8192 each), every pixel transparent black: all four parts 0 |
| `canvas.frame() -> Frame ! GpuError` | The next picture to draw into it; `frame.show()` finishes the drawing, which then shows in its pixels and its texture |
| `canvas.pixels() -> [UInt8]` | Its pixels as its last frame shown left them: four for each pixel (red, green, blue and opacity, in sRGB), row by row from the top. It waits for the GPU |
| `canvas.texture() -> Texture` | The canvas as a texture for fragment fns to read, which changes as the canvas is drawn into; `nearest: true` samples it to the nearest pixel. Only the GPU reads it: `sample` on the CPU is a panic (a contract), as for a compressed texture |
| `canvas.width`, `canvas.height: Int` | Its size in pixels |

- A draw into a canvas that reads the canvas's own texture is a panic: a picture can't be read while it is drawn.
- A canvas's frame is a `Frame` like a screen's, with all its methods. It doesn't wait for the screen's refresh.
- A canvas needs `Window`, as every drawing does (14.24); it opens no window, so it works where no screen is, given a GPU.

**Textures for text and pictures.**

- **One channel.** `gpu.texture(width, height, pixels, channels: 1)` takes one byte a pixel, a quarter of the memory of four. `sample` gives its value as red, `(value, 0.0, 0.0, 1.0)`, as GPUs read one channel; the value is not a colour, so it is not changed from sRGB. A glyph atlas's coverage (14.31) is one channel: take each pixel's first part.
- **Nearest pixel.** A texture made with `nearest: true` is sampled to the pixel under the spot, not blended between the four nearest, on the GPU and by `sample` on the CPU alike: for pixel art, and for an exact glyph atlas drawn at its size. The textures of one draw are all nearest, or none is; mixing them is a panic.
- **Changing part of a texture.** `gpu.update(t, x, y, width, height, pixels)` changes the pixels of texture `t` from pixel `x`, `y` (from the top left), `width` × `height` pixels, with `pixels` as many bytes a pixel as `t` has. `t` is an `inout` parameter (3.9): its pixels change on the GPU and in `t`, so `sample` on the CPU sees them too. A draw already sent keeps what it read. A part past the texture's edges, or another number of bytes, is a panic (a contract), and so is updating a texture only the GPU reads. A glyph atlas that grows (14.31) sends only its new rows:

```kavach
atlas = atlas.add(sans, typed)
if atlas.image.height > letters.height {
    letters = try gpu.texture(atlas.image.width, atlas.image.height, atlas.image.pixels)
} else {
    gpu.update(letters, 0, 0, atlas.image.width, atlas.image.height, atlas.image.pixels)
}
```

## Open questions

- For a maintainer's review (14.24.1): a frame's clip, blending and mask are state the draws after them follow, not arguments of each draw, so `frame.draw` keeps its shape. The mask is one mark a pixel, not nested masks. A canvas needs `Window`, as `gpu.texture` does, though it opens no window. Pipelines' other forms are compiled at their first draw, not ahead.
- Compiling pipelines when the program is built (`kavach build`), kept with it, so no shader compiles while a game runs (decision 0106's next stage).
- Samplers that repeat at the edges, and mipmaps made from `gpu.texture`'s pixels. GPU timers are in the trace (14.12).
- Supercompressed KTX2 files (Basis Universal, Zstandard), which a GPU of any kind can read once they are turned into its blocks.
- Reading image files into textures: left to packages, which give `gpu.texture` the pixels.
- Lines and points as well as triangles, and blending other than drawing over, plain or premultiplied.
- Choosing the present mode (vsync off, mailbox) and the color format.
