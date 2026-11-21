import type { HeapObject, LaneState, Scene, SceneFrame } from "./types";

// spawn, step by step, in native code: the sample explain/spawn.native.kav
// (`kavach run --native`; the code of tasks/sub-spawn). Facts, read in the
// runtime (2026-11-21):
// - main's code runs on a thread the runtime starts with a 512 MB stack
//   (runtime/native/src/lib.rs:280-283), and that thread hosts coroutines
//   (lib.rs:167, make_host).
// - A plain spawn on a host thread is a coroutine on that thread
//   (runtime/native/src/rt_tasks.rs:250 and 325-326): no new OS thread.
//   Its stack is corosensei's DefaultStack of 512 MB
//   (rt_coroutines.rs:196); that it takes memory only as it is used is
//   the OS's doing (PLATFORM).
// - The Task object is a heap object holding the result
//   (rt_tasks.rs:44-46, 279-289). The new task goes to the back of the
//   ready queue (runtime/core/src/tasks.rs:286, ready is a VecDeque); a
//   waiting task gives the turn to the first ready one
//   (runtime/core/src/tasks/pick.rs:58).
// - Passing the turn between coroutines switches stacks with no system
//   call (rt_coroutines.rs:5-11). A finished coroutine is dropped with its
//   stack (rt_coroutines.rs:253-257).

const MAIN = "main";
const TASK = "slow_double(21)";
const lanes = [MAIN, TASK];

const core = "cpu.core0";
// Far apart, so their labels never touch, even on a phone.
const chips = { main: "ram.chip0", task: "ram.chip3", heap: "ram.chip6" };

/** The Task object, at one fixed (example) address the whole scene. */
const taskObj = (result: string): HeapObject[] => [
  { title: "Task<Int>, held by t", cells: [{ text: `result: ${result}`, addr: "0x6000_0000_1a40" }] },
];

function step(
  line: number,
  at: string,
  running: string,
  lane: Record<string, LaneState>,
  where: Record<string, string>,
  result: string,
  caption: string,
  extra: Partial<SceneFrame> = {},
): SceneFrame {
  const taskStack = lane[TASK] === "done" ? "freed" : "task stack";
  return {
    line,
    at,
    caption,
    lit: [core, chips.main, ...(lane[TASK] && lane[TASK] !== "done" ? [chips.task] : []), chips.heap],
    labels: { [core]: running, [chips.main]: "main stack", ...(lane[TASK] ? { [chips.task]: taskStack } : {}), [chips.heap]: "heap" },
    lanes: lane,
    where,
    heap: taskObj(result),
    ...extra,
  };
}

const onMain = "main's thread";

export const spawnScene: Scene = {
  id: "spawn",
  title: "spawn: a task on the same thread",
  caveman: "Spawn make small task. Same thread, own stack. Task wait in line. Main wait? Task run. Task done? Main go on.",
  sample: "explain/spawn.native",
  code: "file",
  board: [
    { kind: "cpu", col: 0, row: 0, cores: 4, label: "CPU: one thread on one core" },
    { kind: "ram", col: 1, row: 0, label: "RAM: two stacks and the heap" },
    { kind: "kernel", col: 0, row: 1 },
  ],
  lanes,
  lanesAre: "tasks on main's thread",
  panels: ["heap", "timeline"],
  heapEmpty: "Nothing yet.",
  frames: [
    step(5, "spawn slow_double(21)", "main", { [MAIN]: "run", [TASK]: "ready" }, { [MAIN]: onMain, [TASK]: "ready queue, 1st" }, "none yet",
      "main works out the argument, 21, then spawns. main's thread hosts coroutines, so the runtime, while running, makes the task a coroutine on that same thread: no new OS thread. It gets a stack of its own (512 MB of address space; the OS gives it memory only as it is used) and a Task object on the heap. The scheduler puts it at the back of the ready queue, and main keeps the turn."),
    step(6, "working meanwhile", "main", { [MAIN]: "run", [TASK]: "ready" }, { [MAIN]: onMain, [TASK]: "ready queue, 1st" }, "none yet",
      "main still has the turn, so it goes on. Its line goes out through the OS. The task waits in the ready queue: nothing runs it until main waits.",
      { packets: [{ from: core, to: "kernel.files", label: "line" }], output: "working meanwhile" }),
    step(7, "t.wait()", "slow_double", { [MAIN]: "wait", [TASK]: "run" }, { [MAIN]: "waits for t", [TASK]: `${onMain}, own stack` }, "none yet",
      "t.wait(): the task hasn't ended, so main waits and gives up the turn. The scheduler, while running, picks the first ready task. Passing the turn switches stacks on the same thread, with no system call: the OS sees nothing.",
      { output: "working meanwhile" }),
    step(1, "n * 2", "21 × 2", { [MAIN]: "wait", [TASK]: "run" }, { [MAIN]: "waits for t", [TASK]: `${onMain}, own stack` }, "none yet",
      "slow_double runs on the same core and the same thread, with its own stack: 21 * 2 = 42.",
      { output: "working meanwhile" }),
    step(2, "}", "return 42", { [MAIN]: "ready", [TASK]: "done" }, { [MAIN]: "ready: what it waited for ended", [TASK]: "ended" }, "42",
      "It returns: 42 goes into the Task object on the heap, and the task ends. The runtime drops its coroutine, and the task's stack with it, at once. main's wait is over, so the scheduler, while running, makes main ready.",
      { packets: [{ from: core, to: chips.heap, label: "42" }], output: "working meanwhile" }),
    step(7, "print(try t.wait())", "main", { [MAIN]: "run", [TASK]: "done" }, { [MAIN]: onMain, [TASK]: "ended" }, "42",
      "main runs again: wait() reads 42 from the Task object, and print writes it out. One thread did all of it, on one core, taking turns.",
      { packets: [{ from: chips.heap, to: core, label: "42" }], output: "working meanwhile\n42" }),
  ],
  note: "Native code (kavach run --native or kavach build). The interpreter (kavach run) gives each task an OS thread of its own instead, and the same turns. The OS picks the core; addresses are examples.",
  spec: "9.1",
};
