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
| `screen.frame() -> Frame ! GpuError` | The next picture to draw, as big as the window is now. It waits for the screen's next refresh when the last picture isn't shown yet (vsync), so a loop of frames runs at the screen's rate |
| `frame.clear(r, g, b)` | Fills the picture with one color; each part is a `Float32` from 0 to 1 |
| `frame.draw(pipeline, vertices)` | Draws `vertices` through `pipeline` (15.3) |
| `frame.draw(pipeline, vertices, instances: items)` | The same once for each item of `items`, which the vertex fn gets as its second parameter |
| `frame.draw(pipeline, vertices, instances: items, shared: value)` | The same, `value` given to every vertex as the vertex fn's last parameter |
| `frame.show()` | Puts the picture on the screen |
| `gpu.pipeline(vertex_fn, fragment_fn) -> Pipeline<V, I, S>` | A pipeline drawing triangles with a `gpu vertex fn` and a `gpu fragment fn` (15.3). `V`, `I` and `S` are what the vertex fn takes; `NoInput`, an empty record, stands for an instance or shared value it doesn't take |

- `GpuError` is `Unavailable(reason: String)`: there is no GPU, or the window can't be drawn into, as on a server with no screen. A frame that can't be had (the window is minimized, or its surface was lost) is `Unavailable` too; the next `frame()` tries again.
- A frame not shown before its last holder ends is thrown away: nothing reaches the screen.
- The vertices are a list of a record type; each three make a triangle. A draw draws over what the frame already holds, blending by opacity.
- A list that a GPU `map` made, and that is still in GPU memory (15.2), is drawn from there with nothing copied, as vertices or as instances: a particle swarm moved by a `map` each frame is drawn where it is, and never comes back to the CPU.
- `frame.draw` without `instances:` or `shared:` that its pipeline's vertex fn takes is a compile error.
- A pipeline is compiled at its first draw, into frames of each color format.
- `gpu.screen_id`, `frame_id`, `clear_id`, `show_id` and `close_id` are what these are built on, by number.
- Generated tests (11.5) skip functions that use `Window`: there is no fake screen yet.

## Open questions

- Compiling pipelines when the program is built (`kavach build`), kept with it, so no shader compiles while a game runs (decision 0106's next stage).
- Depth, textures, samplers, drawing into a texture, and GPU timers: next, in this module.
- Lines and points as well as triangles, and blending other than drawing over.
- Choosing the present mode (vsync off, mailbox) and the color format.
