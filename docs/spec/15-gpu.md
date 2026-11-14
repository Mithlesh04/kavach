# 15. GPU

Kavach runs data-parallel work on the GPU with no buffers, indices or launches in the code ([decision 0044](../decisions/0044-gpu-through-wgpu.md)). Mark a function `gpu fn`, and `map` a list with it. Why it works this way, and what it costs, is in [architecture/gpu.md](../architecture/gpu.md).

```kavach
gpu fn glow(x: Float32, gain: Float32) -> Float32 {
    let v = x
    for _ in 0..100 {
        v = v * 0.99 + gain
    }
    v
}

const bright = pixels.map(p => glow(p, 0.5))    // on the GPU for a long list
```

## 15.1 gpu fn

A function marked `gpu fn` may run on the GPU, so it uses only what a GPU can do. It **must**:

- take and give only `Float32`, `Int32`, `UInt32` and `Bool` (2.1), and records and tuples of 1 to 8 fields of those types. GPUs compute 32-bit numbers fast; `Float` and `Int` (64 bits) are slow or missing there;
- use in its body only: `const` and `let` of those types, assignment, `if` and `else`, `while`, `for` over a range (`0..n`), `return`, the operators of 1.8 (`**` on `Float32` only), conversions between those types (`Float32(n)`, `Int32(x)`), the methods `abs`, `min`, `max` and `clamp`, `math.sqrt`, `floor`, `ceil`, `round` and `pow` (14.5), calls to other `gpu fn`s, and the fields of its records and tuples: reading one (`p.x`, `t.0`), and making a new record or tuple (`Particle(x: x, y: y)`, `(a, b)`).

A `gpu fn` that gives a record moves a whole item at a time, so a game's frame is one `map` of the list of the last frame (15.2):

```kavach
type Ball {
    y: Float32
    vy: Float32
}

gpu fn fall(b: Ball) -> Ball {
    const vy = b.vy - 9.8 * 0.016
    Ball(y: b.y + vy * 0.016, vy: vy)
}

for _ in 0..300 {
    balls = balls.map(fall)        // stays in GPU memory between frames
}
```

Inside a `gpu fn`, a number with no other type is an `Int32` or a `Float32`, not an `Int` or a `Float`: `for _ in 0..100` counts in `Int32`, and `const k = 0.5` is a `Float32`.

Anything else (lists, text, maps, `print`, capabilities, a function that isn't a `gpu fn`, recursion) is a compile error that names what isn't allowed. So a `gpu fn` never fails on the GPU in a way it couldn't on the CPU. It can't fail on purpose either: `!` in its signature, and `try`, `catch` and `fail` in its body, are compile errors, since the GPU has no error to give back; it checks values with `if` and gives a result for every item.

A `gpu fn` gives its result back: `inout` parameters (3.9) are a compile error, with a message showing the function that returns the new value instead (`gpu fn bump(x: Float32) -> Float32`). This holds for a `gpu fn` called from another too, so every `gpu fn` follows one rule.

A `gpu fn` is also an ordinary function: it can be called anywhere, and runs on the CPU when called directly.

## 15.2 Running on the GPU

`list.map(f)` runs on the GPU when:

1. `f` is a `gpu fn` taking one value, or a lambda made only of calls to `gpu fn`s, its parameter or the parameter's fields, numbers, and captured values of the four types above (`p => glow(p, gain)`, `p => step(p.x, p.v)`);
2. the list's items are one of those types, or tuples or records of 1 to 8 fields of those types. So two lists of numbers can run together through `collections.zip` (14.9):

```kavach
type Particle {
    x: Float32
    v: Float32
}

const moved = particles.map(p => step(p.x, p.v))
const sums = collections.zip(xs, ys).map(t => add(t.0, t.1))
```

3. the work is big enough for the GPU to win, and the machine has a GPU. The runtime decides from the list's length and the work per item, which the compiler estimates from the `gpu fn` (its operations, with a loop counted once per round). While the GPU is still opening, at the program's start, only work that would take the CPU longer than opening it waits for it.

Otherwise the same `map` runs on the CPU. The code is the same either way, and so are the results:

- Whole-number arithmetic is checked on the GPU too: overflow and division by zero panic with the same message as on the CPU, naming the same line inside the `gpu fn`. When several items fail, the GPU may report a different one first.
- A `map` whose list stays in GPU memory (below) doesn't wait for the GPU to finish, so frames run back to back ([decision 0061](../decisions/0061-gpu-maps-dont-wait.md)). A panic in its `gpu fn` is reported the next time the program reads the list on the CPU, or when another GPU call waits for its own result, or when the program ends, with the message and the `gpu fn`'s line as always; code after the `map` may have run by then.
- `+`, `-` and `*` on `Float32` give the same bits as on the CPU, unless the GPU fuses a multiply with an add (one rounding instead of two). `/`, `%`, `**`, `math.pow` and `math.sqrt` may differ in the last bits, as WGSL allows; `floor`, `ceil` and `round` are exact.

A list that a GPU `map` read or made may stay in GPU memory for a while (the runtime decides how long), so a later `map` of it doesn't copy it over again. This is invisible to the program: the runtime holds such a list, so changing it copies it first, as changing any list held twice does (2.2).

```kavach
const steps = xs.map(p => glow(p, 0.5))      // xs and steps stay in GPU memory
const more = steps.map(p => glow(p, 0.25))   // reads steps there: no copy over
```

A `map`'s result kept in a variable stays only in GPU memory until the CPU reads it ([decision 0055](../decisions/0055-gpu-results-stay-until-read.md)). So a loop of `map`s copies the list back once, at the first read after the loop, and a list that only the GPU reads is never copied back. This is invisible too: every read sees the list's items. A panic in such a `map` shows later, as the list above says.

```kavach
for _ in 0..100 {
    xs = xs.map(x => step(x))   // stays on the GPU
}
print(xs[7])                    // the one copy back
```

`sum`, `min` and `max` of `Int32` or `UInt32` items can end such a pipeline on the GPU: when the list is in GPU memory, they run there, and only the result comes back. They give exactly what the CPU gives. A sum adds in another order on the GPU, but every partial sum is checked. If any running sum, as the CPU adds the items in order, leaves the type, the CPU adds the list again in order, so the panic names the same item and line. A list on the CPU stays there: copying it over would cost more than the CPU's pass.

```kavach
for _ in 0..100 {
    xs = xs.map(x => step(x))
}
print(xs.sum(x => x))           // on the GPU: only the sum comes back
print(xs.max())
```

A loop that adds up results doesn't wait for each one. When a statement like `total += xs.map(f).sum(g)` (with `sum`, `min` or `max`, on a list a GPU `map` or `filter` made) is the only place the loop mentions `total`, and the statement only works out numbers from values the loop doesn't change, the loop goes on while the GPU works, so the next chunk is read and copied over in the meantime ([decision 0067](../decisions/0067-streaming-data-through-the-gpu.md)). The statement still runs once for each round, in order, with its checks, before any code after the loop. A few rounds' work may be under way at once, fewer for big chunks. This is invisible: nothing can read `total` until its statements have run. Only the moment of a panic shows, as for a `map` that doesn't wait: code after the statement in the loop, and later rounds, may have run by then. A `print(total)` in the loop keeps the loop as written: each round waits for its sum. `kavach check` says which loops wait, and why: `main.kav:7: note: this loop waits for the GPU each round: the loop also reads what it adds up to`.

```kavach
let total = 0
for chunk in try fs.chunks(path, 67_108_864) {
    const xs = chunk.as_list<Int32>() ?? []
    total += Int(xs.map(x => work(x)).sum(x => x))   // the next chunk is read meanwhile
}
print(total)                                         // every chunk's sum has been added
```

`filter` of a list in GPU memory, of one of the four types, runs there too, with a lambda that fits a `gpu fn` (it may call one). It keeps the items the CPU keeps, in their order, and its result stays in GPU memory like a `map`'s, so a pipeline can go on after it.

```kavach
const lit = xs.filter(x => x > 0)   // on the GPU: the kept items stay there
print(lit.map(x => step(x)).sum(x => x))
```

`map`s chained in one expression run as one: `xs.map(f).map(g)` works out `g(f(x))` for each item in a single pass on the GPU, so the list between them is never made, copied back or stored. The results are those of the two `map`s; a panic in either stops the work with its message, as on the CPU. On the CPU the two `map`s run as written.

```kavach
const lit = pixels.map(p => scale(p, gain)).map(p => glow(p, 0.5))   // one pass on the GPU
```

The environment variable `KAVACH_GPU` changes where it runs, for debugging: `off` runs everything on the CPU, `on` uses the GPU for any length, and `check` runs both and panics at the first item whose results differ (floats compared with `near`, 14.2).

## 15.3 Drawing

A picture is drawn by two kinds of `gpu fn` ([decision 0106](../decisions/0106-drawing-on-the-gpu.md)): a `gpu vertex fn` works out where each corner of a triangle goes on the screen, and a `gpu fragment fn` gives the color of each pixel inside it. `gpu.pipeline` puts the two together, and a frame draws with it (14.24).

```kavach
type Corner {
    x: Float32
    y: Float32
}

type Dot {
    x: Float32
    y: Float32
    heat: Float32
}

type Camera {
    zoom: Float32
}

type Lit {
    position: (Float32, Float32, Float32, Float32)
    heat: Float32
}

gpu vertex fn place(c: Corner, d: Dot, cam: Camera) -> Lit {
    Lit(position: ((c.x + d.x) * cam.zoom, (c.y + d.y) * cam.zoom, 0.0, 1.0), heat: d.heat)
}

gpu fragment fn glow(l: Lit) -> (Float32, Float32, Float32, Float32) {
    (l.heat, 0.2, 1.0 - l.heat, 1.0)
}

const dots = gpu.pipeline(place, glow)
frame.draw(dots, square, instances: swarm, with: Camera(zoom: 1.0))
```

- Both follow the rules of a `gpu fn` (15.1), and are ordinary functions on the CPU too.
- A `gpu vertex fn` takes the vertex, then, if the draw has them, the instance and the value shared by the whole draw, in that order: one, two or three records. It gives a record whose field `position` is four `Float32`s: where the corner lands, from -1 to 1 across the window (x right, y up), then depth and `w`, as GPUs take them. Its other fields, `Float32`s, go to the fragment fn, blended between the triangle's corners.
- A `gpu fragment fn` takes the vertex fn's result and gives the pixel's color: red, green, blue and opacity, each from 0 to 1.
- `gpu.pipeline(v, f)` is a compile error unless `v` is a `gpu vertex fn`, `f` a `gpu fragment fn`, and `f` takes what `v` gives. `frame.draw` is a compile error unless its vertices, instances and shared value are what `v` takes.
- `vertex` and `fragment` are words only between `gpu` and `fn`.

## Open questions

- `sum`, `min` and `max` of `Float32` on the GPU, and `filter` of records and tuples. Decided (Mithlesh, 2026-11-03): a `sum` of floats keeps its adding order ([decision 0031](../decisions/0031-loops-on-all-cores.md)), so on the GPU it runs only when the program asks for a reordered sum. Float `min` and `max` must keep `NaN` last (14.2).
- A `Gpu` capability for code that chooses the device or keeps GPU memory, as the platform plan describes.
