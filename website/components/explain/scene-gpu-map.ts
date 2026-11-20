import { hex } from "./panels";
import type { HeapObject, LaneState, Scene, SceneFrame } from "./types";

// A million pixels through a gpu fn: the sample explain/gpu-map.native.kav
// (`kavach run --native`), on a machine with a GPU. The facts, checked in
// the source (2026-11-21):
// - The compiler writes each gpu fn a `map` uses as a WGSL kernel while
//   compiling, with `// work <n>` (its operations per item, a loop counted
//   once per round) on its first line (compiler/backend/wgsl/src/lib.rs:161,
//   cost.rs:13-100): one unit per node, a loop's body + 1 per round. glow
//   is about 7 a round × 1000 = about 7,000 (by reading; not printed).
// - The program opens the GPU and compiles its kernels on a background
//   thread as it starts, unless the GPU is off (runtime/native/src/
//   rt_gpu.rs:127-143, `warm`; wgsl lib.rs:267-292 puts GpuWarm first).
//   An in-place map compiles its own variant at first use (runtime/gpu/
//   src/pipeline.rs:25-35); this one isn't in place.
// - The cost check, while running: n × work / 10 ns of CPU time; the GPU
//   when that is 2 ms or more (300 ms while the GPU is still opening).
//   1M × 7,000 / 10 = 700 ms: over both, so the map waits for the GPU to
//   open (devices.rs:201-208 `ready`). KAVACH_GPU=on and check skip it;
//   off or cpu keep every map on the CPU (rt_gpu.rs:24-48 worth_it,
//   :233-236 map; core lib.rs:391-402). KAVACH_GPU and `[run] gpu` can
//   also name a maker or a GPU number (decision 0139).
// - A list packed at 4 bytes goes up as it is; others are turned into
//   4-byte words on the CPU first (rt_gpu.rs:200-228 with_kernel_bytes).
//   The captured values go in `info`, after the item count (rt_gpu.rs
//   captures_of; runtime/gpu/src/lib.rs:5-10, submit).
// - 256 items per workgroup (runtime/gpu/src/lib.rs:28-29): a million
//   items are 3,907 groups.
// - A local a GPU map sets keeps its result on the GPU (`GpuMapStay`,
//   compiler/kir/src/gpu_stay.rs:77-96), and that map doesn't wait
//   (decision 0061; rt_gpu.rs `if !stay { finish_late() }`; runtime/gpu/
//   src/lib.rs `run`, late::keep). Its RAM list is made zero-filled until
//   fetched. The runtime drops the stay under KAVACH_GPU=check, for a
//   one-word result not packed at 4 bytes ([Bool]), and for a list bigger
//   than one dispatch (rt_gpu.rs `let stay = …`; map.rs:118-137).
// - The stop flag is 20 bytes, 5 words: the flag, what happened, two
//   operands, the line (runtime/gpu/src/lib.rs `read_back`). It is read
//   at the next CPU read of the list (kept.rs bring_back → finish_late),
//   another GPU wait, or the program's end (kept.rs at_exit(finish_late);
//   runtime/gpu/src/late.rs:61-84).
// - Up to 4 lists, 512 MB, stay in GPU memory: the map's own list, when it
//   went up from RAM in one dispatch, and its result (rt_gpu/kept.rs:19-20;
//   runtime/gpu/src/map.rs:150-159).

const core = "cpu.core0";
const pixChip = "ram.chip2";
const litChip = "ram.chip5";
const vram = "gpu.vram";
const groups = ["gpu.group0", "gpu.group1", "gpu.group2", "gpu.group3"];

const AMBER = "#e6a23c";
const GREEN = "#5fbf7f";
const PURPLE = "#c586c0";

/** Example addresses: 4-byte items side by side. They differ on every run. */
const PIX = 0x7f3a_1000_0000;
const LIT = 0x7f3a_2000_0000;
const at = (base: number, i: number) => hex(base + 4 * i);

const free = (n: number, base?: number) => Array.from({ length: n }, (_, i) => ({ text: "free", addr: base ? at(base, i) : undefined }));
const items = (xs: string[], base?: number) => [...xs.map((text, i) => ({ text, addr: base ? at(base, i) : undefined })), { text: "…", note: "999,997 more" }];

const PIXELS = ["0/255", "1/255", "2/255"];
const GLOWED = ["≈ 25.0", "≈ 25.0", "≈ 25.0"];

/** RAM's lists, each at its one place: not made yet, made, written. */
function ram(pixels: boolean, lit: "none" | "made" | "written"): HeapObject[] {
  return [
    { title: "pixels: 1,000,000 Float32s × 4 bytes = 4 MB", tint: pixels ? AMBER : undefined, cells: pixels ? items(PIXELS, PIX) : free(4, PIX) },
    {
      title: "lit: 4 MB",
      tint: lit === "none" ? undefined : GREEN,
      cells: lit === "none" ? free(4, LIT) : lit === "made" ? [...free(3, LIT).map((c) => ({ ...c, text: "0" })), { text: "…", note: "zeros until fetched" }] : items(GLOWED, LIT),
    },
  ];
}

const STOP = ["flag", "what", "a", "b", "line"];

/** VRAM's buffers, each at its one place. */
function gpuMem(up: boolean, lit: boolean): HeapObject[] {
  return [
    { title: "pixels, copied over: 4 MB", tint: up ? AMBER : undefined, cells: up ? items(PIXELS) : free(4) },
    { title: "info: the count, then gain", tint: up ? PURPLE : undefined, cells: up ? [{ text: "1000000", note: "items" }, { text: "0.25", note: "gain" }] : free(2) },
    { title: "lit's items: 4 MB", tint: lit ? GREEN : undefined, cells: lit ? items(GLOWED) : free(4) },
    { title: "stop flag: 5 words, 20 bytes", tint: up ? PURPLE : undefined, cells: STOP.map((note) => ({ text: up ? "0" : "free", note })) },
  ];
}

const lanes = (cpu?: LaneState, gpu?: LaneState): Record<string, LaneState> => ({
  ...(cpu ? { "CPU: main thread": cpu } : {}),
  ...(gpu ? { GPU: gpu } : {}),
});

function step(line: number, text: string, f: Omit<SceneFrame, "line" | "at">): SceneFrame {
  return { line, at: text, ...f };
}

export const gpuMapScene: Scene = {
  id: "gpu-map",
  title: "A map on the GPU: copied over, run at once, left there",
  caveman: "Big list, heavy work? Runtime send to GPU. List go over. Many small cores work at once. Answer stay on GPU. CPU no wait. CPU read answer? Then answer come back. Once.",
  sample: "explain/gpu-map.native",
  code: "file",
  board: [
    { kind: "cpu", col: 0, row: 0, cores: 4, label: "CPU" },
    { kind: "ram", col: 1, row: 0, label: "RAM" },
    { kind: "gpu", col: 1, row: 1, label: "GPU, with its own memory" },
    { kind: "kernel", col: 0, row: 1 },
  ],
  lanes: ["CPU: main thread", "GPU"],
  lanesAre: "the CPU and the GPU",
  panels: ["heap", "vram", "timeline"],
  frames: [
    step(0, "gpu fn glow", {
      heap: ram(false, "none"),
      vram: gpuMem(false, false),
      caption:
        "While compiling, the compiler checks that glow uses only what a GPU can do, and writes it, with the lambda that calls it, as a WGSL kernel: a small GPU program kept inside this one. It also counts glow's work per item: 1000 rounds of a multiply and an add. As the program starts, the runtime opens the GPU and compiles that kernel for it, on a background thread, unless the GPU is set off.",
    }),
    step(11, "pixels.push", {
      lit: [core, pixChip],
      labels: { [core]: "push", [pixChip]: "pixels" },
      heap: ram(true, "none"),
      vram: gpuMem(false, false),
      lanes: lanes("run"),
      caption:
        "The CPU fills pixels in RAM: a million Float32s, 4 bytes each, side by side: 4 MB. A list of Float32 is packed, so its items are already in the form a GPU reads.",
    }),
    step(14, "pixels.map", {
      lit: [core],
      labels: { [core]: "worth it?" },
      heap: ram(true, "none"),
      vram: gpuMem(false, false),
      lanes: lanes("run"),
      caption:
        "CPU or GPU? The runtime decides while running. The compiler counted about 7,000 steps per item; a million items make about 700 ms of CPU time. That is over the 2 ms that makes the GPU worth it, and over the 300 ms worth waiting for a GPU still opening, so if the GPU isn't open yet, the map waits for it. Less work stays on the CPU, as does every map on a machine with no usable GPU: the same output, more slowly. KAVACH_GPU, or [run] gpu in kavach.toml, changes this: on and check skip the check, cpu or off keep every map on the CPU, and a maker's name or a GPU number picks the GPU.",
    }),
    step(14, "pixels.map", {
      lit: [pixChip, vram],
      labels: { [pixChip]: "pixels", [vram]: "copying in" },
      packets: [
        { from: pixChip, to: vram, label: "pixels · 4 MB" },
        { from: "ram.chip6", to: vram, label: "count, gain" },
      ],
      heap: ram(true, "none"),
      vram: gpuMem(true, false),
      lanes: lanes("run"),
      caption:
        "The runtime copies pixels over the PCIe link into the GPU's own memory, VRAM, as it is. Only lists already packed at 4 bytes (Int32, UInt32, Float32) go as they are: a [Bool] is widened to 4-byte words, and a record's fields narrowed from 8 to 4 bytes, on the CPU first. Every field of every item goes up. So do the captured values: gain, after the item count.",
    }),
    step(3, "v = v * 0.99 + gain", {
      lit: [...groups, vram],
      labels: { [vram]: "3,907 groups × 256" },
      heap: ram(true, "made"),
      vram: gpuMem(true, true),
      lanes: lanes("run", "run"),
      caption:
        "The GPU runs the kernel: one thread per item, in 3,907 groups of 256, on its many small cores at once. Each thread runs glow's 1000 rounds on its own pixel and writes the result into lit's buffer in VRAM. An item that overflowed a whole number, or divided by zero, would set the stop flag: what happened, its two numbers and the line.",
    }),
    step(15, "print(pixels.len())", {
      lit: [core, "kernel.files"],
      labels: { [core]: "print", "kernel.files": "output" },
      packets: [{ from: core, to: "kernel.files", label: "1000000" }],
      heap: ram(true, "made"),
      vram: gpuMem(true, true),
      lanes: lanes("run", "run"),
      output: "1000000",
      caption:
        "The map didn't wait. lit is kept in a local, so the compiler, while compiling, marked this map to leave its result on the GPU (decision 0061): the runtime queued the work and went on. lit's list in RAM is made, filled with zeros until its items come back. The CPU prints 1000000 while the GPU may still be working. (With KAVACH_GPU=check, a result like a [Bool] that isn't packed at 4 bytes, or a list too big for one pass, the map waits and copies back instead.)",
    }),
    step(16, "lit.all", {
      lit: [core, vram],
      labels: { [core]: "waits", [vram]: "stop flag" },
      packets: [{ from: vram, to: "ram.chip6", label: "stop flag · 20 B" }],
      heap: ram(true, "made"),
      vram: gpuMem(true, true),
      lanes: lanes("wait", "run"),
      output: "1000000",
      caption:
        "The first CPU read of lit. The compiler put a fetch before it, while compiling. The runtime waits for the GPU and reads the stop flag: 20 bytes back over PCIe. Had an item failed, its panic would show here, with glow's line, after line 16 had run. Another GPU wait, or the program's end, reads the flag too.",
    }),
    step(16, "lit.all", {
      lit: [vram, litChip],
      labels: { [litChip]: "lit", [vram]: "copying back" },
      packets: [{ from: vram, to: litChip, label: "lit · 4 MB" }],
      heap: ram(true, "written"),
      vram: gpuMem(true, true),
      lanes: lanes("run", "done"),
      output: "1000000",
      caption:
        "The flag is clear, so the runtime copies lit's 4 MB back over PCIe into its list in RAM, once. A map whose result is read at once, not kept in a local, waits for the GPU and copies it back by itself.",
    }),
    step(16, "print(lit.all", {
      lit: [core, "kernel.files"],
      labels: { [core]: "all", "kernel.files": "output" },
      packets: [{ from: core, to: "kernel.files", label: "true" }],
      heap: ram(true, "written"),
      vram: gpuMem(true, true),
      lanes: lanes("run", "done"),
      output: "1000000\ntrue",
      caption:
        "all runs on the CPU over lit's items: each is near 25, so it prints true. pixels and lit stay in GPU memory too: pixels went up in one piece, so it is kept. The runtime keeps up to 4 lists a map read or made, 512 MB at most, so a later map of either copies nothing over.",
    }),
  ],
  note: "A picture of the steps, not of the times. Simplified: the GPU's memory also holds a few small buffers for copying, and a list too big for one pass goes up in pieces and isn't kept there. Drawn on a graphics card: a GPU built into the CPU uses part of RAM as its memory, so its copies stay in RAM. Addresses are examples: they differ on every run. On a machine with no usable GPU, or with KAVACH_GPU set to cpu or off, the same program runs the map on the CPU and prints the same lines. Plain kavach run, the interpreter, always runs a map on the CPU.",
  spec: "15.2",
};
