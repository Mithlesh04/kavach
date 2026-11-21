import type { LaneState, Scene, SceneFrame } from "./types";

// spawn, spawn cores and spawn thread in one program, drawn as an LLVM
// build (`kavach build`) runs it on 4 cores: explain/spawn-kinds.native.kav,
// checked with `kavach run --native`, which prints the same. Read in the
// compiler and runtime (2026-11-21):
// - A file with a `spawn cores` anywhere gets, while compiling, a call at
//   the top of the program that starts the workers
//   (compiler/kir/src/lower_drops.rs:47-53, 140-152); the runtime starts a
//   thread per worker but the first, which is main's own
//   (runtime/native/src/rt_tasks.rs:141-146; rt_workers.rs:93-107;
//   runtime/core/src/tasks.rs:203-204). Workers: one per core only when
//   the program was built by LLVM, otherwise one (rt_workers.rs:19-27).
// - spawn (rt_tasks.rs:246-337): on a host thread (main's, or a worker's)
//   a plain spawn is a coroutine there (250, 325-326). spawn cores goes to
//   the next worker in turn, counting from 0 (rt_workers.rs:55-60); aimed
//   at its own worker it is a coroutine there, with no copy (253-255).
//   On another worker it gets a copy of its values (260-263) and waits in
//   that worker's inbox (rt_workers.rs:62-77). spawn thread is always a
//   new OS thread with a 512 MB stack (250, 327-331); in an LLVM build it
//   runs ahead on a copy and waits for its turn only before an effect
//   others could see, ending included (rt_tasks.rs:16-22, 98-106, 133-135,
//   259, 341-345). Off a host thread, even a plain spawn is an OS thread.
// - On main's worker, waiting tasks give the turn to the first ready one,
//   in the order they became ready (runtime/core/src/tasks/pick.rs:58).

const lanes = ["main", "a", "b", "c", "d"];

type Who = Partial<Record<"main" | "a" | "b" | "c" | "d", LaneState>>;

const W0 = "main's thread, worker 0";
const ALONG = "worker 1's thread, alongside";
const AHEAD = "its own thread, running ahead";

function step(line: number, at: string, who: Who, where: Record<string, string>, cores: Record<number, string>, caption: string, extra: Partial<SceneFrame> = {}): SceneFrame {
  const labels: Record<string, string> = {};
  for (const [i, word] of Object.entries(cores)) labels[`cpu.core${i}`] = word;
  return { line, at, caption, lit: Object.keys(cores).map((i) => `cpu.core${i}`), labels, lanes: who, where, ...extra };
}

export const spawnKindsScene: Scene = {
  id: "spawn-kinds",
  title: "spawn, spawn cores, spawn thread: which thread runs each task",
  caveman: "Plain spawn: same thread, take turn. Spawn cores: next worker, maybe other core. Spawn thread: new thread, always. Same answer every way.",
  sample: "explain/spawn-kinds.native",
  code: "file",
  board: [
    { kind: "cpu", col: 0, row: 0, cores: 4, label: "CPU: 4 cores, a worker each" },
    { kind: "ram", col: 1, row: 0, label: "RAM: copies for other threads" },
    { kind: "kernel", col: 0, row: 1 },
  ],
  lanes,
  lanesAre: "tasks",
  panels: ["timeline"],
  frames: [
    step(8, "fn main()", { main: "run" }, { main: W0 }, { 0: "main" },
      "The file has a spawn cores, so the compiler, while compiling, put a call at the very top that starts the workers, one per core. The runtime, while running, starts threads for workers 1 to 3; worker 0 is main's own thread. A worker with nothing to do sleeps, on no core.",
      { lit: ["cpu.core0", "kernel.scheduler"], labels: { "cpu.core0": "main", "kernel.scheduler": "3 threads" } }),
    step(9, "spawn sum_to(10)", { main: "run", a: "ready" }, { main: W0, a: "main's thread: a coroutine" }, { 0: "main" },
      "A plain spawn on main's thread is a coroutine there (the runtime, while running): its own stack, the same thread, and a place in worker 0's ready queue. main keeps the turn."),
    step(10, "spawn cores sum_to(20)", { main: "run", a: "ready", b: "ready" }, { main: W0, a: "main's thread", b: "main's thread: worker 0" }, { 0: "main" },
      "spawn cores sends its task to the next worker in turn, counting from worker 0. So this first one lands on worker 0, main's own: it becomes a coroutine here, just like a, with nothing copied."),
    step(11, "spawn cores sum_to(30)", { main: "run", a: "ready", b: "ready", c: "run" }, { main: W0, a: "main's thread", b: "main's thread", c: ALONG }, { 0: "main", 1: "c" },
      "The next one goes to worker 1. It gets a copy of its values, here one number, so no object is shared between threads, and waits in worker 1's inbox. Worker 1's thread wakes and runs it as a coroutine there, while main goes on: each worker has its own turns. The OS picks the core.",
      { packets: [{ from: "cpu.core0", to: "ram.chip3", label: "copy" }] }),
    step(12, "spawn thread sum_to(40)", { main: "run", a: "ready", b: "ready", c: "run", d: "run" }, { main: W0, a: "main's thread", b: "main's thread", c: ALONG, d: AHEAD }, { 0: "main", 1: "c", 2: "d" },
      "spawn thread always makes a new OS thread, with a 512 MB stack. Built by LLVM, it runs ahead: on a copy of its values, it computes at once, and waits for its turn only before something others could see, such as ending. A plain spawn inside d would make a thread too: d's thread hosts no coroutines.",
      { packets: [{ from: "cpu.core0", to: "ram.chip4", label: "copy" }] }),
    step(13, "try a.wait()", { main: "wait", a: "run", b: "ready", c: "run", d: "run" }, { main: "waits for a", a: "main's thread", b: "main's thread", c: ALONG, d: AHEAD }, { 0: "a", 1: "c", 2: "d" },
      "main waits for a. On worker 0 the scheduler, while running, gives the turn to the first ready task, a: a switch of stacks on the same thread."),
    step(3, "total += i", { main: "ready", a: "done", b: "run", c: "run", d: "run" }, { main: "ready: a ended", a: "ended: 45", b: "main's thread", c: ALONG, d: AHEAD }, { 0: "b", 1: "c", 2: "d" },
      "a adds up to 45 and ends, so main is ready, behind b and d. b, next in worker 0's ready queue, runs its loop."),
    step(5, "total", { main: "ready", a: "done", b: "done", c: "run", d: "run" }, { main: "ready", a: "ended: 45", b: "ended: 190", c: ALONG, d: "its turn: it ends" }, { 2: "d" },
      "b ends with 190. Now d's turn comes: it has worked out 780 on its own thread, or finishes first, and only now does it end, in turn. Its thread ends with it."),
    step(13, "try c.wait()", { main: "run", a: "done", b: "done", c: "done", d: "done" }, { main: W0, a: "ended: 45", b: "ended: 190", c: "ended: 435", d: "ended: 780" }, { 0: "main" },
      "main runs and reads each result. c ran on worker 1 alongside all this: it has ended, or main waits for it here. Three ways to run, one output, the same on every run.",
      { output: "45 190 435 780" }),
  ],
  note: "Drawn for a program built with kavach build (LLVM) on 4 cores; the output is checked with kavach run --native, which prints the same. There, and in a program Cranelift built, spawn cores has one worker: b and c are both coroutines on main's thread, and d's thread waits for its turn before it starts. The interpreter (kavach run) gives every task an OS thread, taking the same turns. Which core runs a thread is the OS's choice.",
  spec: "9.8",
};
