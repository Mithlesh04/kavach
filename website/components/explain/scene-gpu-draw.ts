import type { HeapObject, PipelineView, Scene, SceneFrame } from "./types";

// One frame of a window program drawing two squares through a pipeline:
// the sample explain/gpu-draw.shown.kav, checked by the compiler and never
// run (it opens a window). The facts, checked in the source (2026-11-21):
// - The compiler writes the vertex and fragment fns into one WGSL module
//   while compiling, its first line the buffers' layout
//   (compiler/backend/wgsl/src/render.rs:1-15; kir/src/lower_draw.rs).
// - A screen gets an sRGB color format, present mode Fifo (vsync) and at
//   most 2 frames waiting (runtime/gpu/src/draw.rs:78-92). Every pipeline
//   is compiled for each open screen's color format when the screen opens
//   and when the pipeline is made: never in the middle of a frame
//   (runtime/gpu/src/render.rs:1-12, `ready_for`, `made`).
// - Each draw makes new vertex, instance, index and shared buffers, except
//   a list a GPU map left in GPU memory, drawn where it is; an index past
//   the last vertex stops the draw before anything is drawn
//   (runtime/gpu/src/render.rs `draw_into`). A draw is recorded into the
//   frame; `show` sends it all to the GPU and presents the picture
//   (runtime/gpu/src/draw.rs `draw`, `show`).
// - In a vertex or fragment fn, an overflow gives a wrong value, not a
//   panic (spec 15.3; render.rs header: `halt` does nothing).
// - Where a compiled pipeline lives is the driver's choice. Kavach keeps
//   wgpu handles (render pipelines by pipeline and color format), with no
//   PipelineCache, so nothing is saved between runs (render.rs:81-128,
//   243): the scene says "held by the driver" and draws it nowhere.
// - The format is sRGB when the surface offers one, else its first
//   (draw.rs:80-83). Drawing stays on the program's own GPU: the one the
//   settings name, or else the best one (decision 0139).
// PLATFORM: that `frame()` waits while 2 pictures wait for the screen is
// wgpu's Fifo with desired_maximum_frame_latency 2.

const core = "cpu.core0";
const chip = "ram.chip3";
const vram = "gpu.vram";
const groups = ["gpu.group0", "gpu.group1", "gpu.group2", "gpu.group3"];

const AMBER = "#e6a23c";
const GREEN = "#5fbf7f";

const free = (n: number) => Array.from({ length: n }, () => ({ text: "free" }));

/** How far one frame has got: each buffer keeps its place in VRAM throughout. */
type Stage = 0 | 1 | 2 | 3 | 4 | 5;

function gpuMem(s: Stage): HeapObject[] {
  const made = s >= 3;
  return [
    {
      title: "this frame's picture: 640 × 480 pixels",
      tint: s >= 2 ? AMBER : undefined,
      cells: s >= 5 ? [{ text: "black" }, { text: "2 squares", note: "drawn" }] : s >= 2 ? [{ text: "black", note: "cleared" }, { text: "…" }] : free(2),
    },
    { title: "vertices: square, 4 × 8 bytes", tint: made ? GREEN : undefined, cells: made ? ["-.1, -.1", ".1, -.1", ".1, .1", "-.1, .1"].map((text, i) => ({ text, note: `corner ${i}` })) : free(4) },
    { title: "instances: swarm, 2 × 12 bytes", tint: made ? GREEN : undefined, cells: made ? [{ text: "-.5, 0, .2", note: "dot 0" }, { text: ".5, .25, .9", note: "dot 1" }] : free(2) },
    { title: "indices: corners, 6 × 4 bytes", tint: made ? GREEN : undefined, cells: made ? ["0", "1", "2", "0", "2", "3"].map((text) => ({ text })) : free(6) },
    { title: "shared: Camera, 4 bytes", tint: made ? GREEN : undefined, cells: made ? [{ text: "1.0", note: "zoom" }] : free(1) },
  ];
}

const STAGES = ["place: each corner", "corners to triangles", "pixels inside", "glow: each pixel", "picture", "screen"];
const strip = (stage: number, notes?: Record<number, string>): PipelineView => ({
  title: "The GPU's drawing pipeline, for this frame's draw",
  stages: STAGES,
  items: [
    { label: "dot 0", stage },
    { label: "dot 1", stage },
  ],
  notes,
});

function step(line: number, text: string, s: Stage, f: Omit<SceneFrame, "line" | "at" | "vram">): SceneFrame {
  return { line, at: text, vram: gpuMem(s), ...f };
}

export const gpuDrawScene: Scene = {
  id: "gpu-draw",
  title: "One frame through a drawing pipeline",
  caveman: "Two small fn. One say where corner go. One say what color pixel is. Compiler turn both into GPU program. Runtime ready it before any frame. Each draw: send corners and dots to GPU. GPU paint. Screen show at its beat.",
  sample: "explain/gpu-draw.shown",
  code: "file",
  board: [
    { kind: "cpu", col: 0, row: 0, cores: 4, label: "CPU" },
    { kind: "ram", col: 1, row: 0, label: "RAM" },
    { kind: "screen", col: 0, row: 1, label: "Screen: the window" },
    { kind: "gpu", col: 1, row: 1, label: "GPU, with its own memory" },
  ],
  panels: ["vram"],
  frames: [
    step(23, "gpu vertex fn place", 0, {
      caption:
        "While compiling, the compiler checks place and glow, and that glow takes what place gives. It writes both into one WGSL module, a GPU program. Its first line tells the runtime how the buffers are laid out: a corner is 2 Float32s, a dot 3, the shared Camera 1.",
    }),
    step(33, "gpu.screen(win)", 0, {
      lit: [core, "screen"],
      labels: { [core]: "screen" },
      caption:
        "The runtime, while running, readies the window for the program's own GPU, the one the settings name or else the best one: an sRGB color format when the screen offers one, else its first format, and Fifo, which shows each picture at the screen's next refresh (vsync), with at most 2 pictures waiting. A pipeline made before this would be compiled for its color format now.",
    }),
    step(34, "gpu.pipeline(place, glow)", 1, {
      lit: [core],
      labels: { [core]: "compile" },
      caption:
        "A pipeline made while a screen is open is compiled at once, for each open screen's color format: ahead, never in the middle of a frame. The GPU's driver turns the WGSL into the GPU's own code and holds it, ready for every frame; where it keeps it is the driver's choice. Kavach keeps only a handle to it, and saves nothing between runs.",
    }),
    step(39, "screen.frame()", 2, {
      lit: [core, vram],
      labels: { [core]: "frame", [vram]: "picture" },
      caption:
        "The next picture to draw into, as big as the window is now. When 2 pictures already wait for the screen, this waits for its next refresh, so the loop runs at the screen's rate. clear then fills it with black.",
    }),
    step(41, "frame.draw", 2, {
      lit: [core],
      labels: { [core]: "check" },
      caption:
        "Before anything is drawn, the runtime checks, while running, that each of the 6 indices names one of square's 4 corners. An index of 4 or more would stop the program here with a panic, and nothing would be drawn.",
    }),
    step(41, "frame.draw", 3, {
      lit: [chip, vram],
      labels: { [chip]: "square, swarm", [vram]: "new buffers" },
      packets: [
        { from: chip, to: vram, label: "square · 32 B" },
        { from: "ram.chip4", to: vram, label: "swarm, corners, Camera" },
      ],
      caption:
        "Each draw makes new buffers in VRAM and copies them over: the vertices, the instances, the indices and the shared value. Only a list a GPU map left in GPU memory is drawn where it is, with nothing copied. The draw is noted in the frame; the GPU hasn't drawn yet.",
    }),
    step(42, "frame.show()", 3, {
      lit: [core, vram],
      labels: { [core]: "show", [vram]: "work sent" },
      pipeline: strip(0),
      caption:
        "show sends the frame's work, the clear and the draw, to the GPU in one go, and asks for the picture to be shown. The CPU goes on to the next round of the loop while the GPU draws.",
    }),
    step(24, "Lit(position", 4, {
      lit: [...groups, vram],
      labels: { [vram]: "place × 8" },
      pipeline: strip(0),
      caption:
        "The GPU runs place on its many small cores, for each corner of each dot: 4 corners × 2 dots. Corner (-0.1, -0.1) of dot 0 lands at (-0.6, -0.1). heat goes on to glow. An overflow here gives a wrong value, not a panic: the GPU can't stop in the middle of a frame.",
    }),
    step(28, "(l.heat, 0.2", 5, {
      lit: [...groups, vram],
      labels: { [vram]: "glow × each pixel" },
      pipeline: strip(3, { 1: "6 indices: 2 triangles a square", 2: "heat blended between corners" }),
      caption:
        "The GPU joins the corners into triangles, finds the pixels inside each, and runs glow once for each of them: dot 0's pixels get (0.2, 0.2, 0.8), blue; dot 1's (0.9, 0.2, about 0.1), red. Each lands in the picture.",
    }),
    step(42, "frame.show()", 5, {
      lit: [vram, "screen.text"],
      labels: { [vram]: "picture" },
      packets: [{ from: vram, to: "screen", label: "the picture" }],
      pipeline: strip(5),
      caption:
        "At the screen's next refresh the picture is shown: Fifo, vsync. Then the loop draws the next frame, 600 in all, with new buffers each time.",
    }),
  ],
  note: "A window program: the compiler checked this code, and the site doesn't run it. One frame of the loop is shown; each frame makes its buffers anew and lets the last ones go. Simplified: the GPU's memory also holds buffers for copying, and the GPU may run place once for each index rather than each corner.",
  spec: "14.24",
};
