# benchmarks/gpu

`map` with a `gpu fn` on the GPU ([spec 15](../../docs/spec/15-gpu.md), [decision 0044](../../docs/decisions/0044-gpu-through-wgpu.md)), against the same `map` on the CPU and against C++ on one core and on all cores. The work is heavy per item (1,000 dependent multiply-adds), the kind a GPU is built for; light work, where copying the list costs more than computing it, stays on the CPU ([architecture/gpu.md](../../docs/architecture/gpu.md)).

| File | What it measures |
|---|---|
| [kavach/glow.kav](kavach/glow.kav) | 4 million `Float32` items, each through 1,000 steps of `v * 0.999 + gain`, mapped twice |
| [cpp/glow.cpp](cpp/glow.cpp) | The same in C++, with an OpenMP loop |
| [kavach/frames.kav](kavach/frames.kav) | A game-style loop: 1 million particles (records of four `Float32`s) moved and bounced for 300 frames, each frame a `map` of the last |
| [kavach/frames-lambda.kav](kavach/frames-lambda.kav) | The same step written in the `map`'s lambda, on the CPU |
| [kavach/stream.kav](kavach/stream.kav) | A file of `Int32`s worked on by the GPU in chunks of 64 MB, the sum of each kept |
| [cpp/frames.cpp](cpp/frames.cpp) | The same in C++, an array of structs, with an OpenMP loop |
| [cpp/frames.cu](cpp/frames.cu) | The same in CUDA: one kernel launch per frame, the particles kept on the GPU |
| [cpp/stream.cu](cpp/stream.cu) | `stream.kav` in CUDA, by hand: a ring of slots, each with pinned memory and a stream of its own |

## Results

Windows 11 laptop: Intel i7-13700HX (24 threads), NVIDIA GeForce RTX 4050 Laptop GPU (6 GB), through Vulkan. Kavach built by `kavach build` and run on Windows; C++ built by g++ 13 under WSL Ubuntu 24.04 on the same CPU. Typical of 3 runs, in ms.

| Way | First `map` | Second `map` |
|---|---|---|
| Kavach, CPU (`KAVACH_GPU=off`) | 5,588 | 5,601 |
| C++ `-O2`, one core | 5,992 | 5,833 |
| C++ `-O3 -march=native -fopenmp`, 24 threads | 278 | 263 |
| **Kavach, GPU** | **201** | **33** |

All four give the same results to the digits printed.

- Against one core, the GPU is **170× faster**; against the best C++ on all 24 threads, **8× faster**, and faster even on the first `map`, which includes opening the GPU.
- The first `map` waits for the GPU to open (about 170 ms through Vulkan): a program with GPU kernels starts opening it, and compiling them, as it starts, so other work hides that time.
- Of the second `map`'s 33 ms, the GPU computes and copies the results back for about 30. A list of `Float32` keeps 4 bytes per item ([decision 0045](../../docs/decisions/0045-packed-lists-of-small-numbers.md)), so it goes to the GPU and comes back with no conversion; and `xs`, copied over for the first `map`, is still in GPU memory for the second (spec 15.2). Before these, the second `map` took 61 ms.
- The hardware sets the ceiling against all CPU cores: about 12 TFLOPS of `Float32` on this GPU against about 1.3 on this CPU, so about 9×.

## Streams: a file bigger than a chunk

A 2 GB file of `Int32`s, 32 chunks of 64 MB, read with `fs.chunks` and worked on by a `gpu fn` ([decision 0067](../../docs/decisions/0067-streaming-data-through-the-gpu.md)). Measured 2026-11-07 on Windows, with `KAVACH_GPU=on` and the file already in the system's cache, so the disk here is the fastest it can be. Each row ran 5 times, reading ahead off and on in turns, in ms:

| Work | Reading ahead off | On |
|---|---|---|
| Read and `as_list`, no GPU | 1,817–1,935 | 1,135–1,184 |
| A `gpu fn` of 1 step, then `sum` | 3,775–4,397 | 2,908–3,386 |
| A `gpu fn` of 200 steps, then `sum` (`stream.kav`) | 5,801–7,775 | 5,120–5,673 |

**What the numbers say:** `fs.chunks` reads the next piece on a helper thread while the loop works on this one, which costs nothing to write. The gain on the 200 steps is larger than the disk's share of the time explains, and that isn't understood yet.

### Sums a loop doesn't wait for

The same loop, `total += Int(xs.map(x => work(x)).sum(x => x))`, now doesn't wait for each sum (spec 15.2). Measured 2026-11-07, 7 runs in turns against a compiler built from the commit before, in ms, with the same sum every time and the same as CUDA's:

| Work on 2 GB | Waiting for each sum | Not waiting | CUDA, 2 slots |
|---|---|---|---|
| Read and `as_list`, no GPU | 1,392–1,581 | 660–894 | |
| A `gpu fn` of 1 step, then `sum` | 3,124–3,760 | 1,371–1,896 | 1,012–1,083 |
| A `gpu fn` of 200 steps, then `sum` | 5,525–8,743 | 3,952–4,942 | 1,148–1,222 |

After this table, a kernel's multiply stopped dividing to check two numbers of 16 bits, which can't overflow: over 5 runs in turns, the 200 steps went from 4,008–4,062 to 2,785–2,991, and the 1 step stayed at 1,217–1,297 against 1,235–1,468. Two runs far from the rest are left out of the table, both of the old compiler (15,384 for reading, 5,466 for 1 step): this laptop is in use while it measures. CUDA's column is 5 runs, from a comparison in turns the same day. A file of 8 GB, larger than the GPU's 6 GB and too large to stay in the system's cache, took 63–65 s in all four cases, Kavach and CUDA, whichever the work: there the disk sets the speed, and neither loop holds it up.

**What the numbers say:** four changes, in the order they mattered.

- The list between `map` and `sum` no longer comes back to the CPU (64 MB a chunk).
- The next chunk is read and copied over while the GPU works. Two rounds under way were as fast as three or four, so two it is; one round was slower on the 200 steps (5.4–6.9 s against 4.4–4.7 s).
- A big block of memory freed is kept for the next of the same size, so its pages needn't be asked for again (`runtime/native/src/rt_memory.rs`): `as_list` of 64 MB went from 42 ms to about 14. The loops and everyday benchmarks stay level with it, with the same output.
- An upload to the GPU no longer writes zeros first, and reuses the buffer of an earlier chunk (`wgpu` zeroes a buffer made with its contents).

CUDA is still about 1.2× ahead on 1 step and 2.4× on 200. The first is the CPU's work per chunk: `as_list` copies, and the upload copies again, where CUDA reads the file straight into pinned memory. The second is the kernel: each 32-bit step is checked for overflow, which CUDA's isn't. Left of decision 0067: `as_list` as a view, and network chunks (its open question).

## Frames: records that stay on the GPU

2026-11-04, the same machine. A `gpu fn` takes and gives a record (spec 15.1), and each frame's list stays in GPU memory for the next (decision 0055); only the last frame comes back. Typical of 3 runs, in ms.

| Way | 300 frames |
|---|---|
| Kavach, CPU (`KAVACH_GPU=off`) | 841 |
| Kavach, CPU, the step in the `map`'s lambda | 776 |
| C++ `-O2`, one core | 330 |
| C++ `-O3 -march=native -fopenmp`, 24 threads | 206–1,179 (it varies from run to run under WSL) |
| **Kavach, GPU** | **250** |
| CUDA 12.4 (`nvcc -O2`), on Windows | 17, after 90 to open the GPU |

All give the same last particle. Before records could be a `gpu fn`'s result, this loop couldn't run on the GPU at all; with results copied back every frame it took 6,400 ms.

Kavach's GPU now beats one C++ core, but most of its time is spent before the first frame, and CUDA's frames are faster:

- **On the GPU, most of the time is opening it.** The first `map` waits about 235 ms for the GPU to open and compile the kernel, once per program (opening starts with the program, on another thread). Timed from after a first frame, the 300 frames take about 70 ms, 0.23 ms each, against 1.1 ms for C++ on one core (`benchmarks/gpu/kavach/frames.kav` with one `ps = ps.map(step)` before `t0`; 190–200 ms before the three changes below). Of the opening, about 160 ms is Vulkan finding the GPU and 40 ms making the device, the driver's own work (DirectX 12 takes longer, 280–490 ms); compiling the kernel takes 2 ms.
- **CUDA's frames take 17 ms, Kavach's about 70.** Both write each particle back where it was, so the 16 MB of particles can stay in the GPU's 24 MB cache from frame to frame: Kavach does it when nothing but the `map` holds the list, as in `ps = ps.map(step)` ([architecture/gpu.md](../../docs/architecture/gpu.md)). Before, it wrote each frame into a new buffer, which the GPU first fills with zeros (the same CUDA program, changed to do that, takes 75 ms), and then into the buffer of the frame before last (CUDA: 44 ms). Where the rest goes, so far: run alone through `wgpu`, 300 frames of Kavach's own kernel take 35–44 ms, as fast as one written by hand that reads each particle as four words, and one written by hand that reads it as one `vec4<f32>` takes 19–22 ms (in CUDA the two ways are equally fast). So a record of four 32-bit fields now goes to the GPU as one vector, `vec4`, in and out (2026-11-04): side by side with the build before, 300 frames took 112–134 ms against 128–145 ms, 5–20% less, on a machine running slower than for the numbers above. The other 30 ms of the 70 is the last line inside the timer, `ps[999999]`: it brings the whole list back, 16 MB over the bus and 4 million fields into the list's own memory, whose fresh pages the system fills with zeros first. Timed apart (2026-11-05): the 300 `map`s take 30–45 ms, as the kernel does alone, and the read 19–22 ms, 12–21 ms since the fields are copied item by item with their types looked up once. (GPU timestamps showed 205 µs a kernel, but only with a wait after each `map`, and the wait itself slows the GPU: the same run alone takes 66–92 ms with one.) Opening the GPU takes CUDA 90 ms.
- **Measured again side by side (2026-11-07, a busier machine):** after a first frame, 300 frames take Kavach 57–65 ms with `KAVACH_GPU=on`, CUDA 34–36 ms (plus 86–123 ms to open the GPU). A frame costs Kavach about 0.11 ms (1,200 frames: 157–186 ms), CUDA's about 0.08; the rest is the copy over and the read back, which CUDA does too. Reading the stop flags of the maps that didn't wait only at the end, instead of 32 at a time, changed nothing measurable, so that wait is not the cost. Without `KAVACH_GPU=on` the program takes about 330 ms: while the GPU opens (about 235 ms), frames run on the CPU at 3.4 ms each, by design (a light `map` doesn't wait for the GPU), and the list moves to the GPU once it is open.
- A `map` whose list stays on the GPU doesn't wait for it to finish ([decision 0061](../../docs/decisions/0061-gpu-maps-dont-wait.md)), so frames run back to back; a panic in the `gpu fn` shows at the next read of the list. Run side by side with the build before, this saved 30–110 ms of the 300 frames, counting the opening.
- **On the CPU, `ps = ps.map(step)` is about 2.5 times slower than C++**: each frame writes each particle back where it is, as C++ does, whether the step is in the `map`'s lambda (776 ms) or the named `gpu fn` with the GPU off (841 ms). Before, every frame made a new list: 3,900 and 4,070 ms.

Heavier work per particle (more steps, forces between neighbours) is where the GPU's 9× over all cores (above) shows.

## Running it

```
kavach build benchmarks/gpu/kavach/glow.kav -o glow
./glow                     # on the GPU
KAVACH_GPU=off ./glow      # on the CPU
g++ -O3 -march=native -fopenmp benchmarks/gpu/cpp/glow.cpp -o glow-cpp && ./glow-cpp
kavach build benchmarks/gpu/kavach/frames.kav -o frames && ./frames
g++ -O2 benchmarks/gpu/cpp/frames.cpp -o frames-cpp && ./frames-cpp
nvcc -O2 benchmarks/gpu/cpp/frames.cu -o frames-cuda && ./frames-cuda
nvcc -O2 benchmarks/gpu/cpp/stream.cu -o stream-cuda && ./stream-cuda data.bin 200 2
```
