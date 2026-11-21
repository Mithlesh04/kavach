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
- use in its body only: `const` and `let` of those types, assignment, `if` and `else`, `while`, `for` over a range (`0..n`), `return`, the operators of 1.8 (`**` on `Float32` only), conversions between those types (`Float32(n)`, `Int32(x)`), the methods `abs`, `min`, `max` and `clamp`, `math.sqrt`, `floor`, `ceil`, `round`, `pow`, `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`, `exp` and `ln` (14.5), `std.vec`'s types of `Float32` with their operators and methods (14.28), calls to other `gpu fn`s, and the fields of its records and tuples: reading one (`p.x`, `t.0`), and making a new record or tuple (`Particle(x: x, y: y)`, `(a, b)`).

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

Anything else (lists, text, dicts, `print`, capabilities, a function that isn't a `gpu fn`, recursion) is a compile error that names what isn't allowed. So a `gpu fn` never fails on the GPU in a way it couldn't on the CPU. It can't fail on purpose either: `!` in its signature, and `try`, `catch` and `fail` in its body, are compile errors, since the GPU has no error to give back; it checks values with `if` and gives a result for every item.

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

Otherwise the same `map` runs on the CPU. The code is the same either way, and so are the results.

A lambda that would fit but reads a field of a captured record (`q => step(q, peer.x)`) runs on the CPU too, and the compiler warns, naming the field: copy it into a `const` first (`const px = peer.x`, then `q => step(q, px)`).

The code the GPU runs and the code the CPU runs give the same results:

- Whole-number arithmetic is checked on the GPU too: overflow and division by zero panic with the same message as on the CPU, naming the same line inside the `gpu fn`. When several items fail, the GPU may report a different one first.
- A `map` whose list stays in GPU memory (below) doesn't wait for the GPU to finish, so frames run back to back ([decision 0061](../decisions/0061-gpu-maps-dont-wait.md)). A panic in its `gpu fn` is reported the next time the program reads the list on the CPU, or when another GPU call waits for its own result, or when the program ends, with the message and the `gpu fn`'s line as always; code after the `map` may have run by then.
- `+`, `-` and `*` on `Float32` give the same bits as on the CPU, unless the GPU fuses a multiply with an add (one rounding instead of two). `/`, `%`, `**`, `math.pow`, `math.sqrt` and the angle, `exp` and `ln` functions may differ in the last bits, as WGSL allows; `floor`, `ceil` and `round` are exact.

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

`sum`, `min` and `max` of `Int32` or `UInt32` items, and `sum` of `Float32` items, can end such a pipeline on the GPU: when the list is in GPU memory, they run there, and only the result comes back. They give exactly what the CPU gives. A sum of `Float32`s is exact and rounded once (14.2), so the GPU's order of adding can't change its bits. A whole-number sum adds in another order on the GPU, but every partial sum is checked. If any running sum, as the CPU adds the items in order, leaves the type, the CPU adds the list again in order, so the panic names the same item and line. A list on the CPU stays there: copying it over would cost more than the CPU's pass.

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

The environment variable `KAVACH_GPU` changes where it runs, for debugging: `off` (or `cpu`) runs everything on the CPU, `on` uses the GPU for any length, and `check` runs both and panics at the first item whose results differ (floats compared with `near`, 14.2). `[run] gpu = "cpu"` in `kavach.toml` keeps a program's maps on the CPU the same way, as on a machine whose GPU it shouldn't use; `KAVACH_GPU` at launch wins (decision 0126). Both can name a device too (15.6). Both are read in any case: `KAVACH_GPU=CPU` is `cpu`, and `gpu = "Off"` is `"off"`.

A `map` too big for one pass of the GPU runs in several passes, with the same results. For tests, `KAVACH_GPU_WORDS=1000` lets one pass hold at most 1000 32-bit words, so small lists test this on any machine.

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
frame.draw(dots, square, instances: swarm, shared: Camera(zoom: 1.0))
```

- Both follow the rules of a `gpu fn` (15.1), and are ordinary functions on the CPU too.
- A `gpu vertex fn` takes the vertex, then, if the draw has them, the instance and the value shared by the whole draw, in that order: one, two or three records. It gives a record whose field `position` is four `Float32`s, as a tuple or a `Vec4<Float32>` (14.28): where the corner lands, from -1 to 1 across the window (x right, y up), then depth and `w`, as GPUs take them. Its other fields, `Float32`s, go to the fragment fn, blended between the triangle's corners.
- A `gpu fragment fn` takes the vertex fn's result and gives the pixel's color: red, green, blue and opacity, each from 0 to 1, as four `Float32`s or a `Vec4<Float32>`.
- `gpu.pipeline(v, f)` is a compile error unless `v` is a `gpu vertex fn`, `f` a `gpu fragment fn`, and `f` takes what `v` gives. `frame.draw` is a compile error unless its vertices, instances and shared value are what `v` takes.
- `vertex` and `fragment` are words only between `gpu` and `fn`.
- A vertex's, instance's or shared value's fields are `Float32`s, `Int32`s or `UInt32`s, 1 to 8 of them.
- A draw can't stop the program, so in a vertex or fragment fn a whole-number overflow or a division by zero gives a wrong value for that corner or pixel, not a panic (5.6): the GPU has no way to say so in the middle of a frame. On the CPU, the same fn panics as any other.

### Depth

A pipeline made with `depth: true` draws a pixel only where it is nearer than what is already there, so a 3D scene can be drawn in any order:

```kavach
const solid = gpu.pipeline(place, shade, depth: true)
frame.clear(0.0, 0.0, 0.0)
frame.draw(solid, far_wall)
frame.draw(solid, near_box)
```

- A pixel's depth is the third part of `position` divided by the fourth: 0 is nearest, 1 farthest. Only pixels from 0 to 1 are drawn.
- Each frame keeps one depth for each pixel. They start at 1, the farthest, and `frame.clear` sets them back to 1. A pixel drawn with depth keeps its depth there, and a later one is drawn only where its depth is less.
- A pipeline without depth (the default) neither looks at the depths nor changes them: it draws over what is there, as before.
- `depth:` is written `true` or `false`: it is part of the pipeline, which is compiled ahead (14.24). Any other value is a compile error.

### Textures

A `gpu fragment fn` can read pictures, `Texture`s, which it takes after its input. `t.sample(u, v)` gives the color of texture `t` at `u` (0 left, 1 right) and `v` (0 top, 1 bottom):

```kavach
type Spot {
    position: (Float32, Float32, Float32, Float32)
    u: Float32
    v: Float32
}

gpu fragment fn paint(s: Spot, skin: Texture) -> (Float32, Float32, Float32, Float32) {
    skin.sample(s.u, s.v)
}

const tiles = gpu.pipeline(place, paint)
const skin = try gpu.texture(2, 2, pixels: checkers)
frame.draw(tiles, square, textures: [skin])
```

- A color from `sample` is blended between the four pixels nearest the spot, red, green, blue and opacity, each from 0 to 1. A spot past an edge takes the edge's pixels.
- A texture's colors are in sRGB, as image files and screens have them: a texture sampled and drawn as it is shows its own pixels.
- `frame.draw` gives one texture for each `Texture` the fragment fn takes, in order, as `textures:`. Leaving `textures:` out when the fragment fn takes one is a compile error; a list of another length is a panic.
- `sample` is the one call on a `Texture` a `gpu fn` can make, and only a `gpu fragment fn` takes textures. On the CPU, `sample` reads the same pixels and gives the same colors, up to rounding.

## 15.4 Slicing on the GPU

*Not built yet.*

Slices (2.19) of a list in GPU memory (15.2) run on the GPU, so a pipeline doesn't stop to copy the list back ([decision 0134](../decisions/0134-slicing-and-indexing.md)).

- **A slice of a list in GPU memory is made there.** `ys[a:b:step]` copies nothing back to the CPU, and the slice stays in GPU memory like a `map`'s result, so a `map`, `filter`, `sum`, `min` or `max` can follow it there.
- **`xs[a:b:step].map(f)` reads the slice where it is**: from its first item, every `step`th item, with nothing copied in between. When `xs` is in GPU memory, the slice is never made as a list of its own. The same holds for `filter` and the other calls of 15.2 after a slice.
- **Tensors** in GPU memory, once they can stay there (14.33), are sliced there the same way: `t[0:2, 1:]`, `t[:, #1]`.
- **A program's own `Index` and `Slice` methods** (2.16) may be called in a `gpu fn` (`v[i]`, `v[1:3]`) when the method fits a `gpu fn` itself (15.1): pure, and only numbers, records and tuples of the four types.
- **The results are those of the CPU:** the same `#k`, the same clamping, the same step. The step and the ends are worked out on the CPU before the GPU starts, so a step of 0 panics at the same line as on the CPU. An index out of range in a `gpu fn`'s `index` method panics with the CPU's message and line, as overflow does (15.2).

```kavach
for _ in 0..100 {
    xs = xs.map(x => step(x))          // xs stays in GPU memory
}
const firsts = xs[0:1000]              // made on the GPU: no copy back
const lit = xs[::2].map(x => glow(x, 0.5))   // reads every other item in place
```

## 15.5 Grouping and tables on the GPU

*Not built yet.*

The methods that group, count, summarize and join a list (14.2) run on the GPU too, and so do a table's (14.34) ([decision 0135](../decisions/0135-tables-and-grouping.md)). They follow the rules of 15.2: the items are of the four types, or records and tuples of them; every lambda fits a `gpu fn`; and the work is big enough. Otherwise they run on the CPU. Either way the results are the same, in the same order.

```kavach
type Hit {
    sensor: UInt32
    energy: Int32
}

const per_sensor = hits.count_by(.sensor)                                      // on the GPU
const energy = hits.group_by(.sensor).map_values(g => g.sum(.energy))          // one pass: no groups made
const strongest = hits.max_by(.energy)
```

- **Grouping.** `count_by`, `unique` and `unique_by` run on the GPU, with keys of the four types or tuples of them; `Float32` keys follow the rules of 2.2 (`0.0` and `-0.0` one key, NaN one key). The keys come back in the order first seen, as on the CPU.
- **A reduction per key.** `group_by(key).map_values(g => …)`, where the lambda is `g.len()`, `g.sum(f)`, `g.count(test)`, `g.min_by(f)` or `g.max_by(f)`, runs as one pass, as chained `map`s do (15.2): the groups are never made. A whole-number sum is checked as `sum` is in 15.2: if a running sum would leave its type as the CPU adds the group in order, the CPU adds that group again, so the panic names the same item and line.
- **Sums and means.** `sum` and `mean` of `Int32`, `UInt32` and `Float32` run on the GPU, per key too: the exact sum is the same in any order (14.2), and the one rounded division is done on the CPU. `var` and `std` add their squared differences exactly too.
- **Order.** `min_by`, `max_by`, `median`, the quartiles of `describe`, and `sort_by` sort or compare on the GPU, which is exact, so they pick the same items as the CPU; ties go to the first item, as on the CPU.
- **Joins.** `inner_join` and `left_join` with keys of the four types match on the GPU. The `row` lambda runs on the GPU when it fits a `gpu fn`, and on the CPU otherwise. The rows come in the CPU's order: that of `items`, then that of `other`.
- A result stays in GPU memory until the CPU reads it, as a `map`'s does ([decision 0055](../decisions/0055-gpu-results-stay-until-read.md)). `KAVACH_GPU=check` (15.2) runs both and compares them.

## 15.6 Choosing a GPU

A machine may have several GPUs, of any vendors: an NVIDIA or AMD card, the processor's own Intel or AMD GPU, a software one. Each is a **device**, numbered from 0 as `system.gpus()` (14.25) and `gpu.devices()` (14.24) list them, the best first ([decision 0139](../decisions/0139-choosing-gpus.md)). GPUs of different vendors share no memory, so a list one GPU holds and another must read goes through main memory.

**By default the system decides.** A program needs no code to use the right GPU:

- **The devices are ranked once, at the first GPU work:** discrete GPUs, then integrated ones, then virtual and other ones, each kind in the order the system lists them. The CPU comes last. A software GPU (`GpuKind.Software`, which draws on the processor) runs work only when chosen, as the CPU runs Kavach's own code faster.
- **New work goes to the least busy device:** the one with the least work in its queue, by Kavach's own count of the work it sent and hasn't finished; on a tie, the best ranked. A list already in one GPU's memory (15.2) is worked on there.
- **A device that fails is skipped from then on:** one that can't be opened, is lost, or runs out of memory. Its work goes to the next device in the ranking, and to the CPU after the last. The results are the same on any device (15.2), so only the time shows it.

**A developer can choose**, in four ways:

```kavach
use std.gpu

gpu fn step(x: Float32) -> Float32 {
    x * 0.5 + 1.0
}

gpu(1) fn shade(x: Float32) -> Float32 {     // this function's maps on device 1
    x * x
}

fn main() uses Console, Gpu {
    const devices = gpu.devices()            // [GpuDevice], the best first
    for d in devices {
        print(`{d.index}: {d.name} ({d.vendor})`)
    }
    const a = gpu.on(devices[0]) {
        xs.map(step)                         // device 0, as every GPU map in the block
    }
    const b = xs.map(step, on: devices[0])   // this call on device 0
    const c = on gpu(1) xs.map(step)         // device 1, by its number (4.9)
    const d = xs.map(shade)                  // device 1: `shade` says so
}
```

- **A block:** `gpu.on(device) { … }` is a block given to a call (4.8). Every GPU `map`, `filter`, `sum`, `min` and `max` in it runs on `device`, and so does the GPU work of the tasks it spawns (9.1).
- **An argument on a call:** `xs.map(f, on: device)` runs that one `map` on `device`, with any GPU work in working out `xs`. It wins over a block around it. `on:` comes after `f`.
- **A value:** `on gpu(n) x` (4.9) runs the GPU work of `x` on device `n`, as a block does.
- **A function:** `gpu(n) fn` runs the GPU work of the function on device `n`: a `map` given the function, or a lambda calling it (15.2). `n` is a whole number written in the code, 0 or more. A lambda calling `gpu(n) fn`s of two numbers is a compile error. Called from another `gpu fn`, its number means nothing: the outer function's work decides.

Where several choices cover one call, the nearest wins: the call's `on:`, then the innermost `gpu.on`, `on gpu(n)` or `on cpu` around it, then `gpu(n) fn`, then the settings below, then the system. A choice says only where: whether a `map` is big enough to run on a GPU at all is still decided as 15.2 says, and the values are the same as without it.

- **Choosing a device or listing them needs `uses Gpu`** (8.1): `gpu.devices()`, `gpu.on`, `on:`, `on gpu(n)`, and a `map` of a `gpu(n) fn`. A `gpu fn` and its `map` with no choice need nothing, as before.
- **A missing device stops the program at its start,** before any of its code runs, with exit code 1 and a message naming the device and the ones the machine has: a `gpu(n) fn` or an `on gpu(n)` with a number written in the code, or `KAVACH_GPU` or `[run] gpu` naming a device that isn't there. A number worked out while running that names no device is a panic. A device the list has but that can't be opened is skipped, as a device that fails.

**With no code change**, for one run or for a project:

- `KAVACH_GPU` at launch takes a vendor's name (`nvidia`, `amd`, `intel`, `apple`, `qualcomm`, `arm` or `microsoft`, in any case: the best device of that maker), a device's number (`KAVACH_GPU=1`), or `cpu`. A name or number takes the system's place; the code's own choices still win. `cpu` (or `off`) keeps every GPU `map` on the CPU, the code's choices too, as do `on` and `check` (15.2).
- `[run] gpu` in `kavach.toml` takes the same: `gpu = "amd"`, `gpu = "1"`, `gpu = "cpu"`, or `"auto"`, the default, for the system's choice (10.11). `KAVACH_GPU` at launch wins.
- Drawing (15.3, 14.24) uses the program's GPU: the device `KAVACH_GPU` or `[run] gpu` names, or else the best one. A list a `map` left on another device is copied over to be drawn.
- `KAVACH_PAR_LOG=1` notes each device when it opens, and each device skipped, with why.

*Not built yet:*

- **A big job splits across all GPUs:** a large `map`, `filter`, sum or matrix product is cut into parts, one for each device, sized to each device's speed, when its size pays for moving the data, as an `on cores` loop is cut across cores (4.9). The results are the same, in the same order.
- **Ranking by memory:** within a kind, the device with the most memory first. The GPU interfaces Kavach uses don't report it yet, so ties keep the system's order.

## Open questions

- A `Slice` method in a `gpu fn`: its `Part` holds `Int?`s, which a `gpu fn` can't take (15.1).
- `min` and `max` of `Float32` on the GPU, and `filter` of records and tuples. Float `min` and `max` must keep `NaN` last (14.2). Float sums are decided: exact and rounded once, in any order ([decision 0135](../decisions/0135-tables-and-grouping.md), which replaces [decision 0031](../decisions/0031-loops-on-all-cores.md)'s in-order float sums for lists).
- Whether code that keeps GPU memory itself, as the platform plan describes, needs `uses Gpu` too, as choosing a device does (15.6).
