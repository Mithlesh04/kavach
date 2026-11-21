import type { LaneState, Scene, SceneFrame } from "./types";

// A ticker cancelled when its function returns, in native code:
// explain/ticker.native.kav, the code of tasks/sub-cancel (spec 9.2,
// 14.4). Facts, read in the compiler and runtime (2026-11-21):
// - A function that spawns is wrapped, while compiling, as
//   `mark; body; join(mark)` (compiler/kir/src/lower_tasks.rs:272-291);
//   a function's join (no second operand, so `b == 0`) cancels the
//   tasks it spawned that still run, then waits for them
//   (runtime/native/src/rt_tasks.rs:197-199, 427-444). An arena's join
//   passes 1 and only waits (compiler/kir/src/lower_arena.rs:42-43); this
//   scene has no arena.
// - Cancelling a waiting task ends its wait at once
//   (runtime/core/src/tasks.rs:369-381); it then returns from each
//   function up to its start (compiler/kir/src/cancel.rs:5-13). Its timer
//   is left and skipped (runtime/core/src/tasks/pick.rs:61-70).
// - every_ms: before the loop, next = now + ms, so the first tick comes
//   after one interval; each round waits until next, then next = the time
//   it woke + ms; ticks count from 0 (compiler/kir/src/lower_every.rs:
//   42-53, 76-84; runtime/native/src/rt_std.rs:76-84). A late round
//   doesn't catch up.
// - Sleeps are timers in the scheduler; with none ready, the thread
//   waits on a condition variable until the earliest
//   (runtime/native/src/rt_waits.rs:19-29; pick.rs:140-147).

const lanes = ["main", "ticker"];
const core = "cpu.core0";
// Far apart, so their labels never touch, even on a phone.
const stacks = { main: "ram.chip0", ticker: "ram.chip3" };

type Who = Partial<Record<"main" | "ticker", LaneState>>;

function step(line: number, at: string, time: string, who: Who, where: Record<string, string>, caption: string, output?: string): SceneFrame {
  const running = (Object.keys(who) as (keyof Who)[]).find((k) => who[k] === "run");
  const labels: Record<string, string> = { [stacks.main]: "main stack" };
  if (who.ticker) labels[stacks.ticker] = who.ticker === "done" ? "freed" : "ticker stack";
  return {
    line,
    at,
    time,
    caption,
    lit: running ? [core, stacks[running]] : ["kernel.scheduler"],
    labels: { ...labels, ...(running ? { [core]: running } : { "kernel.scheduler": "waiting" }) },
    lanes: who,
    where,
    output,
  };
}

const ticks = (n: number) => ["tick 0", "tick 1"].slice(0, n).join("\n") || undefined;

export const tickerScene: Scene = {
  id: "ticker",
  title: "A ticker, its timers, and its end",
  caveman: "Ticker wait one beat, then tick. Tick 0, tick 1. Sleep not hold core. Function end? Ticker stop. Nothing left running.",
  sample: "explain/ticker.native",
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
    step(12, "watch()", "0 ms", { main: "run" }, { main: "main's thread" },
      "main calls watch(): a plain call, in the same task. watch spawns, so the compiler, while compiling, wrapped it: when it returns, it cancels the tasks it spawned that still run, then waits for them."),
    step(3, "spawn {", "0 ms", { main: "run", ticker: "ready" }, { main: "in watch()", ticker: "ready queue" },
      "The runtime, while running, makes the ticker a coroutine on main's thread, with its own stack. It is ready; watch keeps the turn."),
    step(8, "time.sleep_ms(1000)", "0 ms", { main: "sleep", ticker: "run" }, { main: "timer at 1000 ms", ticker: "main's thread" },
      "watch sleeps 1000 ms: the scheduler notes a timer and parks it, so the ticker runs."),
    step(4, "time.every_ms(400)", "0 ms", { main: "sleep", ticker: "sleep" }, { main: "timer at 1000 ms", ticker: "timer at 400 ms" },
      "every_ms(400) doesn't tick at once: the compiler, while compiling, made each round a sleep until its start, and the first start is one interval from now. So the ticker parks with a timer at 400 ms."),
    step(4, "time.every_ms(400)", "0 → 400 ms", { main: "sleep", ticker: "sleep" }, { main: "timer at 1000 ms", ticker: "timer at 400 ms" },
      "Nothing is ready. The scheduler, while running, takes the earliest timer, 400 ms, and the thread waits on a condition variable until then. The OS takes it off the core: no core is held while tasks sleep."),
    step(5, "tick {tick}", "400 ms", { main: "sleep", ticker: "run" }, { main: "timer at 1000 ms", ticker: "main's thread" },
      "At 400 ms the OS wakes the thread, and the scheduler runs the ticker: its first round. Ticks count from 0. The next round starts 400 ms after this one did, at 800 ms. A late round doesn't catch up: the next one still waits 400 ms from when this one really started.", ticks(1)),
    step(5, "tick {tick}", "800 ms", { main: "sleep", ticker: "run" }, { main: "timer at 1000 ms", ticker: "main's thread" },
      "At 800 ms, tick 1. Its next round would start at 1200 ms, so it parks with that timer.", ticks(2)),
    step(8, "time.sleep_ms(1000)", "1000 ms", { main: "run", ticker: "sleep" }, { main: "in watch()", ticker: "timer at 1200 ms" },
      "At 1000 ms watch's timer comes first: the scheduler runs watch, and its sleep ends. The ticker still waits for 1200 ms.", ticks(2)),
    step(9, "}", "1000 ms", { main: "wait", ticker: "run" }, { main: "waits for the ticker", ticker: "cancelled: on its way out" },
      "watch returns, and the join the compiler put at the end of the function runs: first it cancels the ticker, then it waits for it to end. The runtime ends the ticker's wait at once, long before 1200 ms. Each function on the ticker's stack returns at once, letting go of what it holds. Its timer is left behind; the scheduler skips it.", ticks(2)),
    step(13, 'print("watching stopped")', "1000 ms", { main: "run", ticker: "done" }, { main: "main's thread", ticker: "ended" },
      "The ticker has ended, and its stack is freed. watch is back in main, which prints. Nothing is left running in the background.", "tick 0\ntick 1\nwatching stopped"),
  ],
  note: "Native code, one thread. With no task ever spawned, time.sleep_ms is a plain thread sleep, as there is no scheduler then. The interpreter (kavach run) gives each task an OS thread instead, with the same turns. The OS picks the core.",
  spec: "9.2",
};
