import type { LaneState, Scene, SceneFrame } from "./types";

// Two tasks sleeping, in native code: explain/turns.native.kav, the code
// of tasks/sub-turns (spec 9.7). It replaces the old TaskTurns widget and
// keeps its six steps, with what the machine does between them. Facts,
// read in the runtime (2026-11-21):
// - Plain spawns on main's thread are coroutines there
//   (runtime/native/src/rt_tasks.rs:250, 325-326), ready in spawn order
//   (runtime/core/src/tasks.rs:286-300, 470).
// - time.sleep_ms parks the task in the scheduler with a timer
//   (runtime/native/src/rt_waits.rs:19-23;
//   runtime/core/src/tasks/wait.rs:33-39), counted from when the task
//   became ready (wait.rs:41-47, spec 9.7).
// - With no task ready, the thread waits on a condition variable until
//   the earliest timer (runtime/core/src/tasks/pick.rs:140-147), or on
//   the OS's readiness queue when a task waits on a socket (pick.rs:122-
//   138, mio, decision 0046). A blocked thread uses no core (PLATFORM).
// - Timers come due in time order (a min-heap, pick.rs:47-57); a woken
//   sleeper joins the back of the ready queue (tasks.rs:470).

const lanes = ["main", "slow", "fast"];
const core = "cpu.core0";
// Far apart, so their labels never touch, even on a phone.
const stacks = { main: "ram.chip0", slow: "ram.chip3", fast: "ram.chip6" };
const thread = "main's thread";

type Who = Partial<Record<"main" | "slow" | "fast", LaneState>>;

function step(line: number, at: string, time: string, who: Who, where: Record<string, string>, caption: string, extra: Partial<SceneFrame> = {}): SceneFrame {
  const running = (Object.keys(who) as (keyof Who)[]).find((k) => who[k] === "run");
  const labels: Record<string, string> = { [stacks.main]: "main stack" };
  for (const t of ["slow", "fast"] as const) if (who[t]) labels[stacks[t]] = who[t] === "done" ? "freed" : `${t} stack`;
  return {
    line,
    at,
    time,
    caption,
    lit: running ? [core, stacks[running]] : ["kernel.scheduler"],
    labels: { ...labels, ...(running ? { [core]: running } : { "kernel.scheduler": "waiting" }) },
    lanes: who,
    where,
    ...extra,
  };
}

const out = (n: number) => ["both started", "fast done", "slow done", "slow and fast are back"].slice(0, n).join("\n");

export const turnsScene: Scene = {
  id: "turns",
  title: "Two tasks sleep: who wakes them, and when",
  caveman: "Task sleep? Not hold core. Scheduler write down wake time. Nothing to do? Thread sleep in OS. Time come? OS wake thread. Short sleep wake first.",
  sample: "explain/turns.native",
  code: "file",
  board: [
    { kind: "cpu", col: 0, row: 0, cores: 4, label: "CPU: one thread, when it runs" },
    { kind: "ram", col: 1, row: 0, label: "RAM: a stack per task" },
    { kind: "kernel", col: 0, row: 1 },
  ],
  lanes,
  lanesAre: "tasks on main's thread",
  panels: ["timeline"],
  frames: [
    step(9, 'report("slow", 400)', "0 ms", { main: "run", slow: "ready" }, { main: thread, slow: "ready queue, 1st" },
      "main spawns slow. The runtime, while running, makes it a coroutine on main's thread, with its own stack, and puts it in the ready queue. The task that spawned it keeps running."),
    step(10, 'report("fast", 100)', "0 ms", { main: "run", slow: "ready", fast: "ready" }, { main: thread, slow: "ready queue, 1st", fast: "ready queue, 2nd" },
      "fast too, behind slow in the ready queue. Both are ready, but main still has the turn."),
    step(11, 'print("both started")', "0 ms", { main: "run", slow: "ready", fast: "ready" }, { main: thread, slow: "ready queue, 1st", fast: "ready queue, 2nd" },
      "So main prints first.", { output: out(1) }),
    step(12, "slow.wait()", "0 ms", { main: "wait", slow: "run", fast: "ready" }, { main: "waits for slow", slow: thread, fast: "ready queue, 1st" },
      "main waits for slow and gives up its turn. The scheduler, while running, runs the task that became ready first: slow. The switch is a change of stacks on the same thread, with no system call.", { output: out(1) }),
    step(3, "time.sleep_ms(ms)", "0 ms", { main: "wait", slow: "sleep", fast: "run" }, { main: "waits for slow", slow: "timer at 400 ms", fast: thread },
      "slow asks to sleep 400 ms. The scheduler notes a timer, counted from when slow became ready, and parks slow: no thread sleeps for it, and no core is held. fast runs.", { output: out(1) }),
    step(3, "time.sleep_ms(ms)", "0 ms", { main: "wait", slow: "sleep", fast: "sleep" }, { main: "waits for slow", slow: "timer at 400 ms", fast: "timer at 100 ms" },
      "fast parks too, with a timer at 100 ms. Now nothing is ready. The scheduler, while running, takes the earliest timer, 100 ms, and the thread waits on a condition variable until then: the OS takes it off the core, which is free for other programs. (A task waiting on a socket would make it wait on the OS's readiness queue instead: epoll, kqueue or IOCP.)", { output: out(1) }),
    step(4, "done", "100 ms", { main: "wait", slow: "sleep", fast: "run" }, { main: "waits for slow", slow: "timer at 400 ms", fast: thread },
      "At 100 ms the OS wakes the thread. The scheduler finds fast's time has come and makes it ready, and it is the only one: fast prints.", { output: out(2) }),
    step(5, "name", "100 ms", { main: "wait", slow: "sleep", fast: "done" }, { main: "waits for slow", slow: "timer at 400 ms", fast: "ended" },
      "fast returns its name and ends; its stack is freed. Nothing is ready, so the thread waits again, now until 400 ms.", { output: out(2) }),
    step(4, "done", "400 ms", { main: "wait", slow: "run", fast: "done" }, { main: "waits for slow", slow: thread, fast: "ended" },
      "At 400 ms slow wakes and prints. The sleeps overlapped: slow's 400 ms began at the same moment as fast's 100.", { output: out(3) }),
    step(12, "are back", "400 ms", { main: "run", slow: "done", fast: "done" }, { main: thread, slow: "ended", fast: "ended" },
      "slow ended, which main waited for, so main runs. fast ended long ago, so fast.wait() doesn't wait. About 400 ms in all, not 500, and the same order on every run: the scheduler decides it from the program, not from the machine.", { output: out(4) }),
  ],
  note: "Native code, one thread. With no task ever spawned, time.sleep_ms is a plain thread sleep: there is no scheduler then. The interpreter (kavach run) gives each task an OS thread instead, with the same turns. The OS picks the core.",
  spec: "9.7",
};
