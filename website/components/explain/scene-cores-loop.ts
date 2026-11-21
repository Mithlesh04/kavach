import type { HeapObject, LaneState, Scene, SceneFrame } from "./types";

// A loop `on cores` (spec 4.9, decision 0031), drawn as a program built
// with `kavach build` (LLVM) runs it on 4 cores: explain/cores-loop.native.kav
// (the code of speed/sub-elementwise), checked with `kavach run --native`,
// whose Cranelift code runs the plain loop and prints the same. Read in
// the compiler and runtime (2026-11-21), confirmed by the maintainer's
// checker:
// - While compiling, an eligible loop (arithmetic, list reads, writes to
//   its own slot of a list the function made) is moved into a range
//   function (compiler/kir/src/par.rs:5-15). Only LLVM spreads it;
//   Cranelift always runs the plain loop (cranelift codegen.rs:449-455).
// - While running (runtime/native/src/rt_par.rs, fan_out 254-335): fewer
//   than 65,536 rounds (× a row's width), more than 6 values from outside
//   the loop, or one core → the plain loop (23-29, 266-268). Otherwise a
//   range is max(n / (workers × 4), 4096 / width, n / 256 + 1) rounds
//   (288-290): here max(62,500, 4,096, 3,907) = 62,500, so 16 ranges. The
//   loop's thread and up to workers − 1 kept threads (started once, asleep
//   between loops: 75-96) take ranges from a shared counter (fetch_add,
//   156-176) until none are left; which takes which changes per run. The
//   loop's thread then waits for the others and checks the failed flag
//   (325-334): a failed check means the plain loop runs, so a panic has
//   its exact round. A loop that finds the kept threads busy runs alone.
// - Fewer than 2 workers after an `on cores(n)` cap → the plain loop; the
//   pool is sized by the first loop that spreads, so one under `on cores(n)`
//   may keep n threads for the run (checker's notes, not run).

const ROUNDS = 1_000_000;
const STEP = 62_500;
const N = ROUNDS / STEP;

const lanes = ["loop's thread", "kept 1", "kept 2", "kept 3"];
const LOOP = lanes[0];

/** Which ranges are done, and which thread (lane index) is in which range now. */
function ys(done: number, now: Record<number, number>): HeapObject[] {
  return [
    {
      title: `ys's items, one box per range of ${STEP.toLocaleString("en-US")} rounds; each round writes its own item`,
      cells: Array.from({ length: N }, (_, r) => ({
        text: now[r] !== undefined ? `${r} …` : r < done ? `${r} ✓` : `${r}`,
      })),
    },
  ];
}

function step(line: number, at: string, who: Record<string, LaneState>, where: Record<string, string>, heap: HeapObject[], caption: string, extra: Partial<SceneFrame> = {}): SceneFrame {
  const lit = lanes.map((l, i) => (who[l] === "run" ? `cpu.core${i}` : "")).filter(Boolean);
  const labels: Record<string, string> = {};
  lanes.forEach((l, i) => {
    if (who[l] === "run") labels[`cpu.core${i}`] = where[l]?.split(":")[0] ?? l;
  });
  return { line, at, caption, lit: [...lit, "ram.chip4"], labels: { ...labels, "ram.chip4": "ys" }, lanes: who, where, heap, ...extra };
}

const all = (s: LaneState): Record<string, LaneState> => Object.fromEntries(lanes.map((l) => [l, s]));
const rangeOf = (r: number) => `range ${r}: rounds ${(r * STEP).toLocaleString("en-US")}–${((r + 1) * STEP - 1).toLocaleString("en-US")}`;

export const coresLoopScene: Scene = {
  id: "cores-loop",
  title: "on cores: small ranges, taken in turn by every core",
  caveman: "Many rounds. Cut in small piles. Each core take next pile, again and again. Each round write own box. Answer same as one core.",
  sample: "explain/cores-loop.native",
  code: "file",
  board: [
    { kind: "cpu", col: 0, row: 0, cores: 4, label: "CPU: 4 cores" },
    { kind: "ram", col: 1, row: 0, label: "RAM: xs and ys" },
  ],
  lanes,
  lanesAre: "threads",
  panels: ["heap", "timeline"],
  heapEmpty: "Nothing yet.",
  frames: [
    step(3, "for i in 0..1_000_000", { [LOOP]: "run" }, { [LOOP]: "main: the plain loop" }, ys(0, {}),
      "A plain for fills xs and ys: one core, in order, round after round."),
    step(7, "on cores for", { [LOOP]: "run", "kept 1": "ready", "kept 2": "ready", "kept 3": "ready" }, { [LOOP]: "main", "kept 1": "woken", "kept 2": "woken", "kept 3": "woken" }, ys(0, {}),
      "The compiler, while compiling, proved each round writes only its own ys[i], and moved the body into a function that runs a range of rounds. The runtime, while running, checks: 1,000,000 rounds (65,536 at least), few values from outside the loop (6 at most), 4 cores. So it spreads the loop: a range is the largest of 1,000,000 / (4 × 4), 4,096 and 1,000,000 / 256 + 1, so 62,500 rounds, 16 ranges. It wakes the threads it keeps for loops, and 3 of them join this one."),
    step(8, "ys[i] = xs[i]", all("run"), { [LOOP]: rangeOf(0), "kept 1": rangeOf(1), "kept 2": rangeOf(2), "kept 3": rangeOf(3) }, ys(0, { 0: 0, 1: 1, 2: 2, 3: 3 }),
      "Each thread takes the next free range from a shared counter, one atomic add, and runs it. The OS puts the threads on cores. Each round writes only its own slot, so no two threads touch the same item."),
    step(8, "ys[i] = xs[i]", all("run"), { [LOOP]: rangeOf(5), "kept 1": rangeOf(7), "kept 2": rangeOf(4), "kept 3": rangeOf(6) }, ys(4, { 4: 2, 5: 0, 6: 3, 7: 1 }),
      "A thread that finishes takes the next free range at once, so none waits for a slow one. Which thread takes which range changes from run to run: this is one run."),
    step(8, "ys[i] = xs[i]", all("run"), { [LOOP]: rangeOf(13), "kept 1": rangeOf(12), "kept 2": rangeOf(15), "kept 3": rangeOf(14) }, ys(12, { 12: 1, 13: 0, 14: 3, 15: 2 }),
      "The last four ranges. When the counter passes 1,000,000, a thread finds no range left and stops."),
    step(9, "}", { [LOOP]: "run", "kept 1": "wait", "kept 2": "wait", "kept 3": "wait" }, { [LOOP]: "main: checks", "kept 1": "asleep till the next loop", "kept 2": "asleep till the next loop", "kept 3": "asleep till the next loop" }, ys(N, {}),
      "The loop's thread waits until the others have left, then checks: did any range fail a check, such as an overflow? None did, so the loop is over. Had one failed, the runtime would throw the work away and run the plain loop on one core, so the panic comes at the same round and line."),
    step(10, "print(ys[999_999])", { [LOOP]: "run", "kept 1": "wait", "kept 2": "wait", "kept 3": "wait" }, { [LOOP]: "main" }, ys(N, {}),
      "Every round wrote its own slot, so ys is exactly what the plain loop makes, in round order.", { output: "313.1875" }),
  ],
  note: "Drawn as a program built with kavach build (LLVM) runs it, on 4 cores. The output is checked with kavach run --native, where Cranelift runs the plain loop on one core: the same answer. The plain loop also runs for fewer than 65,536 rounds, for more than 6 values from outside the loop, on one core, when on cores(n) leaves fewer than 2 threads, and in the interpreter. The threads are started by the first loop that spreads: if that loop is under on cores(n), the program may keep n threads for the rest of the run. A loop that finds the kept threads busy with another loop runs alone on its own thread.",
  spec: "4.9",
};
