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

Windows 11 laptop: Intel i7-13700HX (24 threads), NVIDIA GeForce RTX 4050 Laptop GPU (6 GB), through Vulkan ([the machine](../README.md#the-machine)). Kavach built by `kavach build` at commit ac2657be (`kavach --version`) and run on Windows; C++ built by g++ 13 under WSL Ubuntu 24.04 on the same CPU. Measured 2026-11-09 in three rounds of three runs one after another; each figure is the best of the three rounds, in ms.

| Way | First `map` | Second `map` |
|---|---|---|
| Kavach, CPU (`KAVACH_GPU=off`) | 7,017 | 7,059 |
| C++ `-O2`, one core | 6,744 | 6,631 |
| C++ `-O3 -march=native -fopenmp`, 24 threads | 296 | 302 |
| **Kavach, GPU** | **207** | **5** |

The best of each round, first and second `map`: Kavach on the GPU 224 and 5, 207 and 5, 232 and 7; on the CPU 7,057 and 7,287, 7,017 and 7,059, 7,027 and 7,189; C++ on one core 7,082 and 6,980, 6,744 and 6,631, 6,778 and 6,673; on 24 threads 347 and 348, 309 and 302, 296 and 309. All four give the same results to the digits printed.

- A `map` on the GPU doesn't wait for its kernel ([decision 0061](../../docs/decisions/0061-gpu-maps-dont-wait.md)), so each timer ends with a read of one item of the result, which waits for the kernel and brings the whole list back, 16 MB. Without the reads (until 2026-11-09) the second `map` timed only handing the work to the GPU: 0–1 ms.
- The second `map`'s 5–9 ms is the kernel and that copy back: about 1,300× one C++ core and 60× C++ on all 24 threads. The first `map` waits for the GPU to open (about 170 ms through Vulkan): a program with GPU kernels starts opening it, and compiling them, as it starts, so other work hides that time. Even so it is 33× one core and 1.4× all of them.
- A list of `Float32` keeps 4 bytes per item ([decision 0045](../../docs/decisions/0045-packed-lists-of-small-numbers.md)), so it goes to the GPU and comes back with no conversion; and `xs`, copied over for the first `map`, is still in GPU memory for the second (spec 15.2). Before these, the second `map` took 61 ms; on 2026-11-03, 33 ms.
- The C++ on 24 threads is far from the CPU's peak: each item's 1,000 steps are one chain of multiply-adds, each waiting for the last, so it reaches about 27 GFLOPS of the CPU's 1.3 TFLOPS. A program at both chips' peaks would see about 9× between them (about 12 TFLOPS of `Float32` on this GPU).

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

Measured again 2026-11-08, 5 runs in turns against the binaries of the day before, the median in ms, the same sum every time, with the file in the system's cache and the machine busy:

| Steps, on 2 GB | Before | Now | CUDA, 2 slots |
|---|---|---|---|
| 1 step | 1,949 | 1,746 | 946 |
| 200 steps | 4,363 | 2,740 | 1,064 |
| 800 steps | | 6,258 | 2,228 |

Two changes made that:

- A big chunk is read straight into a word-sized buffer, and `as_list` takes that buffer when nothing else holds the chunk (`runtime/native/src/rt_bytes.rs`), with nothing copied.
- A kernel divides through unsigned numbers (`div_k` and `rem_k`, `compiler/backend/wgsl/src/lib.rs`). On this GPU a signed `%` took about a third longer than an unsigned one, and a multiply in its place gained no more. A divisor the compiler knows has no guard either.

CUDA is still about 1.8× ahead on 1 step and 2.6× on 200, with the file in the system's cache; in the table before, the disk held CUDA back too. Time hardly changes with the steps up to about 200 for either, as the kernel runs while the next chunk is read and copied: there the difference is the CPU's work per chunk. The upload's copy is not it: on this GPU the copy into `wgpu`'s staging memory is the trip over the bus, 6 ms per 64 MB, as CUDA's is. But `write_buffer` made a new 64 MB staging buffer every few chunks, at 100–280 ms each, so the GPU runtime now copies through two buffers of its own, used in turn (`runtime/gpu/src/upload.rs`). Run 6 times in turns from one build, switching between the two ways on a busy machine, 32 chunks held in memory went from 869 to 764 ms (the uploads alone 410 → 304), and the 1-step stream, with the file in the system's cache, from about 1,250 to 1,040. After that, each chunk's `map` takes about 9 ms and the loop waits for the file, as CUDA's does; most of the rest is opening the GPU, which this program's time includes and CUDA's doesn't. Past that the kernel is the limit, and written by hand and run through the same loop at 800 steps it took 7.1 s with today's checks, 5.3 s without them, and 4.3 s with `wgpu`'s own counter in each loop off too, against CUDA's 1.7 s. That counter stops a kernel that never ends, so since [decision 0077](../../docs/decisions/0077-kernels-whose-loops-end.md) only a kernel whose every loop is a `for` over an exclusive range goes without it: over 5 runs in turns the same day, 200 steps went from 2,816 to 2,541 and 800 steps from 6,007 to 4,909, against CUDA's 1,062 and 2,233, with the same sums. Left of decision 0067: `as_list` as a view when the chunk is still held, and network chunks (its open question).

## Frames: records that stay on the GPU

A `gpu fn` takes and gives a record (spec 15.1), and each frame's list stays in GPU memory for the next (decision 0055); only the last frame comes back, read inside the timer. Measured 2026-11-09 at commit ac2657be on the same machine, three rounds of three runs; each row is the best of the three rounds, in ms. The CUDA program is the one built on 2026-11-06.

| Way | 300 frames |
|---|---|
| Kavach, CPU (`KAVACH_GPU=off`) | 687 |
| Kavach, CPU, the step in the `map`'s lambda | 734 |
| C++ `-O2`, one core | 364 |
| C++ `-O3 -march=native -fopenmp`, 24 threads | 552–1,691 (it varies from run to run under WSL) |
| **Kavach, GPU** | **239** |
| CUDA 12.4 (`nvcc -O2`), on Windows | 31, after 62–77 to open the GPU |

The best of each round: Kavach on the CPU 733, 749, 687, with the step in the lambda 734, 741, 766; C++ on one core 364, 382, 367; Kavach on the GPU 270, 242, 239 (the very first run, 293); CUDA 31 in every round (46 the first time). On 2026-11-04 the same table read 841, 776, 330, 206–1,179, 250 and 17 after 90.

All give the same last particle. Before records could be a `gpu fn`'s result, this loop couldn't run on the GPU at all; with results copied back every frame it took 6,400 ms.

Kavach's GPU now beats one C++ core, but most of its time is spent before the first frame, and CUDA's frames are faster:

- **On the GPU, most of the time is opening it.** The first `map` waits about 235 ms for the GPU to open and compile the kernel, once per program (opening starts with the program, on another thread). Timed from after a first frame, the 300 frames take about 70 ms, 0.23 ms each, against 1.1 ms for C++ on one core (`benchmarks/gpu/kavach/frames.kav` with one `ps = ps.map(step)` before `t0`; 190–200 ms before the three changes below). Of the opening, about 160 ms is Vulkan finding the GPU and 40 ms making the device, the driver's own work (DirectX 12 takes longer, 280–490 ms); compiling the kernel takes 2 ms.
- **CUDA's frames take 17 ms, Kavach's about 70.** Both write each particle back where it was, so the 16 MB of particles can stay in the GPU's 24 MB cache from frame to frame: Kavach does it when nothing but the `map` holds the list, as in `ps = ps.map(step)` ([architecture/gpu.md](../../docs/architecture/gpu.md)). Before, it wrote each frame into a new buffer, which the GPU first fills with zeros (the same CUDA program, changed to do that, takes 75 ms), and then into the buffer of the frame before last (CUDA: 44 ms). Where the rest goes, so far: run alone through `wgpu`, 300 frames of Kavach's own kernel take 35–44 ms, as fast as one written by hand that reads each particle as four words, and one written by hand that reads it as one `vec4<f32>` takes 19–22 ms (in CUDA the two ways are equally fast). So a record of four 32-bit fields now goes to the GPU as one vector, `vec4`, in and out (2026-11-04): side by side with the build before, 300 frames took 112–134 ms against 128–145 ms, 5–20% less, on a machine running slower than for the numbers above. The other 30 ms of the 70 is the last line inside the timer, `ps[999999]`: it brings the whole list back, 16 MB over the bus and 4 million fields into the list's own memory, whose fresh pages the system fills with zeros first. Timed apart (2026-11-05): the 300 `map`s take 30–45 ms, as the kernel does alone, and the read 19–22 ms, 12–21 ms since the fields are copied item by item with their types looked up once. (GPU timestamps showed 205 µs a kernel, but only with a wait after each `map`, and the wait itself slows the GPU: the same run alone takes 66–92 ms with one.) Opening the GPU takes CUDA 90 ms.
- **Measured again side by side (2026-11-07, a busier machine):** after a first frame, 300 frames take Kavach 57–65 ms with `KAVACH_GPU=on`, CUDA 34–36 ms (plus 86–123 ms to open the GPU). A frame costs Kavach about 0.11 ms (1,200 frames: 157–186 ms), CUDA's about 0.08; the rest is the copy over and the read back, which CUDA does too. Reading the stop flags of the maps that didn't wait only at the end, instead of 32 at a time, changed nothing measurable, so that wait is not the cost. Without `KAVACH_GPU=on` the program takes about 330 ms: while the GPU opens (about 235 ms), frames run on the CPU at 3.4 ms each, by design (a light `map` doesn't wait for the GPU), and the list moves to the GPU once it is open.
- **Tried 2026-11-10, none kept:** timed apart after a first frame, the 300 `map`s took 24–45 ms and the read 12–24 ms, CUDA's 300 frames 15–34 ms (median about 18), in turns on Windows. The `map`s go at the GPU's pace: past 64 stop flags not read, the oldest half is read, which waits for the GPU. So the CPU's work per frame is not the limit, and four ways to cut it measured flat side by side: reusing the buffers a stop flag is read back through (making the first one the CPU can map took 16–22 ms, once), keeping the buffer of a kernel's count and captures, sending `map`s to the GPU 16 at a time (`queue.submit` was about 65 µs a frame), and reading a four-word item once in the kernel (the driver already did). What is left is the kernel on the GPU, about twice CUDA's; finding why needs a GPU profiler (Nsight), not timers.
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
