# components/explain/ — the explainer kit

Step-by-step animations of how code runs on the machine: the CPU, RAM, the
stack, the heap, the GPU, disks, the network and the OS kernel. A **scene**
is a checked sample and one frame per step; each frame names the line it
runs, and the build fails when that line is not in the sample.

Every scene has, written into the page's HTML: a **"Caveman says"** line
(the idea in very short, very simple words), the animation, and **"Every
step, as text"** (closed until opened). `/explainers.txt` (linked from
`/llms.txt`) and the page's schema.org `LearningResource` data carry the same
text. A page shows a scene with `<Explainer id="…" />`, puts its `<SrcRef>`
links under it, and nothing else: the text versions find it in the page.

## Making a scene

1. Write the sample under `content/samples/` and run
   `node scripts/check-samples.mjs --write`; read the `.expect`. A program
   that opens a window is a `.shown` sample: the compiler checks it, and
   nothing runs it.
2. Write the scene as data (`scene-<name>.ts`, typed by `types.ts`): the
   board's resources, and frames with `line`, `at`, `caption`, and what is
   lit, moving, on the stack, on the heap, in the timeline.
3. Add it to `KIT_SCENES` in `scenes.ts`, and put `<Explainer id>` on its page.
4. Mark any claim about the machine you could not check with `// UNVERIFIED:`.

## Who decides, and when

When a step frees, moves, copies, waits, sleeps, or picks a core or the
GPU, its caption says **who** decided it and **when**, in a few words: "the
compiler, while compiling" (a last use, a move, a register for an
argument, the stack check before a call) or "the runtime, while running"
(the count, the scheduler, the GPU cost check, the OS picking a core). The
caption is also the step's text, so "Every step, as text" says it too. A
step-through of names also marks each name's last use in its code panel
(`CodeLines`' `lastUse`: "a: last use, marked by the compiler while
compiling"); a kit scene gives them as `Scene.lastUse`. A kit scene's heap
uses `heap-cells.ts`, whose `checkHeap` holds it to the rule below.

## Places and addresses stay put

Memory is drawn in address order, and every object, buffer, stack slot and
stack frame keeps one place and one address for the whole scene. An object
not made yet is free cells at its place; a freed one goes grey where it is
(marked "freed"), then is free cells again; a popped frame fades where it
was. Nothing moves up or down to fill a gap. Only a real move by the
runtime changes an address: never invent one, never hide one, and when it
isn't verified (a growing buffer), keep the address and say "may move".
A pointer always shows the fixed address of what it holds. The memory
step-through checks this at build time (`../memory-places.ts`,
`checkPlaces`); the call stack panel works out each slot's address from
its position, so it can't move.

## Files

| File | What it is |
|---|---|
| `types.ts` | The shapes: `SceneInfo`, `Scene`, `SceneFrame`, packets, slots, views |
| `scenes.ts` | Every scene by id: the kit's, and the memory step-throughs' titles and caveman lines |
| `scene-factorial.ts` | The first kit scene: `factorial(10)`, the two calls native code really makes (Functions › Recursion) |
| `scene-list-buffer.ts` | A list in memory: index check, push in place, growth, a slice copied, two items swapped (Collections › Lists) |
| `scene-dict-lookup.ts` | A dict lookup by stored hashes, then through the index past 8 entries (Dicts › Reading a value) |
| `scene-sorting.ts` | Which sort runs for which items, and when sorting copies (Collections › Sorting) |
| `scene-chain.ts` | map → filter → sum as written (Cranelift) and fused into one loop (LLVM) (Lists › map, filter, sum) |
| `scene-file-io.ts` | A file's bytes through the page cache; read whole, by lines, in pieces (Standard Library › Files) |
| `heap-cells.ts` | The heap for kit scenes: places at fixed example addresses, and `checkHeap`, which fails the build when a place moves |
| `scene-gpu-map.ts` | A `map` of a million pixels on the GPU: the cost check, the copy over PCIe, the kernel, the result left in VRAM and read back late (Speed › GPU) |
| `scene-gpu-draw.ts` | One frame through a drawing pipeline: WGSL ahead, new buffers each draw, the index check, vsync (Standard Library › GPU Screens) |
| `scene-spawn.ts` | A plain `spawn`: a coroutine on the same thread, the ready queue, `.wait()` (Concurrency › Tasks) |
| `scene-turns.ts` | Two tasks sleeping: timers, the thread waiting in the OS, the turn order (Concurrency › Taking Turns) |
| `scene-ticker.ts` | `every_ms` ticks, sleeps, and the ticker cancelled when its function returns (Concurrency › Tasks Never Leak) |
| `scene-channel.ts` | Producer → channel → consumer: a full channel makes the sender wait, an empty one the receiver (Concurrency › Channels) |
| `scene-cores-loop.ts` | `on cores for`: small ranges taken from a shared counter by each core, in an LLVM build on 4 cores (Speed › Loops on All Cores) |
| `scene-spawn-kinds.ts` | `spawn`, `spawn cores`, `spawn thread`: which thread runs each, in an LLVM build on 4 cores (Concurrency › How Tasks Run) |
| `scene-call.ts` | One call: arguments in registers, the return address and frame, the result in `rax` (Functions › Returning a value) |
| `scene-compiler.ts` | From a `.kav` file to a running program: `kavach run`, `run --native` and `build` (Projects & Tools › The kavach Command) |
| `scene-print.ts` | `print`: a line built whole, then written in one go through the OS to the terminal (Basics › Output › print) |
| `explainer.tsx` | `Explainer` and `SceneShell`: caveman line, the animation, every step as text; checks the sample |
| `scene-text.ts` | A scene as text: the step list, Markdown for `/explainers.txt`, schema.org data; `sceneCode` checks each step's line |
| `scene-pages.ts` | Which scenes a page shows, read from its content |
| `scene-player.tsx` | The animation: code, board, panels, views and step controls (`../step-player.tsx`) |
| `code-track.tsx` | The code beside a scene, grammar-colored, the running line marked; what the program printed |
| `board.tsx` | The machine: resources in a grid, the kernel band, links, packets; one column on phones |
| `packet.tsx` | A labelled packet moving along a link; still for readers who prefer less motion |
| `picture-parts.ts` | What the pictures share: their box, fixed hardware colors, the `Picture` shape |
| `pictures-compute.tsx` | CPU (cores with registers and L1, shared L3) and GPU (core groups, VRAM) |
| `pictures-storage.tsx` | RAM stick (also used by the memory step-through), SSD, hard disk |
| `pictures-network.tsx` | Network card, internet and remote server; the OS kernel band |
| `pictures-io.tsx` | Screen with a terminal, keyboard |
| `cells.tsx` | 8-byte memory cells on a dark panel, shared with `../memory-blocks.tsx` |
| `panels.tsx` | Registers, call stack (frames pushed and popped), heap, and the GPU's memory (VRAM) |
| `timeline.tsx` | Who runs when: tasks, threads, cores or calls over the steps |
| `strips.tsx` | A pipeline strip (stages in a row) and a ring buffer |
| `bits.tsx` | A value as bits with the CPU's flags, and a latency ladder (figures come from a cited data file, never typed) |
