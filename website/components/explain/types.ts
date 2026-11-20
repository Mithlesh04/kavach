// The shape of an explainer scene: a checked sample, and one frame per
// step. Each frame names the line it runs; components/explain/explainer.tsx
// fails the build when that line is not in the sample. Every frame is
// plain data, so the server can write it out as text (scene-text.ts).

/** One step of any scene: the line it runs, a piece of that line, and what happens. */
export type Step = {
  /** The line this step runs, from 0, in the code the scene shows. */
  line: number;
  /** A piece of that line's text: a changed sample fails the build. */
  at: string;
  /**
   * What happens at this step, in plain words. When the step frees, moves,
   * copies, waits, sleeps, or picks a core or the GPU, it says who decided
   * and when: "the compiler, while compiling" or "the runtime, while running".
   */
  caption: string;
};

/** What every scene has, whatever draws it. */
export type SceneInfo = {
  /** Unique on the site; also the anchor on its page (`#scene-<id>`). */
  id: string;
  /** A short title: the question the scene answers. */
  title: string;
  /** The idea in very short, very simple words. */
  caveman: string;
  /** The sample under content/samples/, without `.kav`. */
  sample: string;
  /** Show the whole file, or only the body of `fn main()`. */
  code: "file" | "main";
  steps: Step[];
};

/** A resource on the board, and the grid cell it sits in. */
export type Placed = {
  kind: "cpu" | "gpu" | "ram" | "ssd" | "hdd" | "net" | "screen" | "keyboard" | "kernel";
  col: number;
  row: number;
  /** A CPU's cores (default 4). */
  cores?: number;
  /** The name under the picture. */
  label?: string;
};

/**
 * A place on the board: a resource, and maybe one of its parts, as
 * `kind` or `kind.part`: `cpu.core2`, `ram.chip3`, `gpu.vram`, `ssd.nand1`,
 * `net.server`, `kernel.files`. See each picture for its parts.
 */
export type Spot = string;

/** Something moving along a link this step: bytes, a value, a request. */
export type Packet = { from: Spot; to: Spot; label: string };

/** One 8-byte slot: a register or a cell in a stack frame. */
export type Slot = { name: string; value: string; note?: string };

/** One call on the call stack: the function, and its slots. */
export type CallFrame = { fn: string; slots: Slot[] };

/** One object on the heap, drawn as a row of 8-byte cells. */
export type HeapObject = { title: string; cells: { text: string; addr?: string }[]; tint?: string };

/** What a task, thread, core or call is doing at one step. */
export type LaneState = "run" | "ready" | "wait" | "sleep" | "done";

/** Stages in a row with items moving through them: a compiler, a CPU's, a GPU's or a data pipeline. */
export type PipelineView = {
  title: string;
  stages: string[];
  /** Each item, and the stage it is in now (from 0); past the last stage, it is out. */
  items: { label: string; stage: number }[];
  /** A note per stage this step: "stalled", "flushed". */
  notes?: Record<number, string>;
};

/** A fixed ring of cells: a channel's buffer, an output buffer, audio. */
export type RingView = {
  title: string;
  /** One entry per cell; null is empty. */
  cells: (string | null)[];
  /** The next cell to take from, and the next to put into. */
  head: number;
  tail: number;
  closed?: boolean;
  /** Who waits, and for what: a sender for room, a receiver for an item. */
  waiting?: { who: string; for: "room" | "item" }[];
};

/** A value as bits, most significant first, in groups: sign, exponent, fraction. */
export type BitsView = {
  title: string;
  bits: string;
  groups?: { label: string; from: number; to: number; tone: "sign" | "exp" | "frac" | "plain" }[];
  /** The CPU's flags this step: overflow, carry, zero. */
  flags?: { name: string; on: boolean }[];
  /** What the bits mean, in words. */
  value?: string;
};

/** One frame of a scene the kit draws: a step, and what the machine shows. */
export type SceneFrame = Step & {
  /** Spots that are busy this step. */
  lit?: Spot[];
  /** A word shown under a spot this step: `{ "cpu.core0": "factorial(2)" }`. */
  labels?: Record<Spot, string>;
  packets?: Packet[];
  /** The registers that matter this step. */
  regs?: Slot[];
  /** The call stack, oldest call first. */
  stack?: CallFrame[];
  heap?: HeapObject[];
  /** What each lane of the timeline does this step; a lane left out is idle. */
  lanes?: Record<string, LaneState>;
  /** Where each lane runs this step: `{ main: "thread 1 on core 0" }`. */
  where?: Record<string, string>;
  /** The clock at this step: "0 ms", "120 ms". */
  time?: string;
  pipeline?: PipelineView;
  ring?: RingView;
  bits?: BitsView[];
  /** What the program has printed so far. */
  output?: string;
};

/** A scene the kit draws: the resources on its board, and its frames. */
export type Scene = Omit<SceneInfo, "steps"> & {
  board: Placed[];
  /** The timeline's rows, top to bottom: threads, cores or calls. */
  lanes?: string[];
  /** What the timeline's rows are, such as "calls" or "threads". */
  lanesAre?: string;
  frames: SceneFrame[];
  /** The panels under the board, in this order of choice. */
  panels?: ("regs" | "stack" | "heap" | "timeline")[];
  /** What the heap panel says while it is empty. */
  heapEmpty?: string;
  /** A note under the picture: what is an example, what is simplified. */
  note?: string;
  /** The spec section the scene shows, such as "3.8". */
  spec?: string;
};
