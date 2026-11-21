import type { LaneState, RingView, Scene, SceneFrame } from "./types";

// A producer and a consumer joined by a channel of capacity 2, in native
// code: explain/channel.native.kav. Read in the runtime (2026-11-21),
// runtime/native/src/rt_channels.rs:
// - One channel object, both ends point to it (270-286): a queue of
//   values, a Rust VecDeque (138), behind a spin lock held only while a
//   value or a waiter goes in or out (71-112), and the tasks waiting to
//   take and to put, first come first (140-143).
// - put: room (fewer than `capacity` values, 339) → the value goes in and
//   the first waiting taker is made ready (340-346, 352); no room → the
//   task notes itself as waiting to put and parks (347-349, 356), a wait
//   in the scheduler while other tasks run (rt_waits.rs:84-90).
// - take: a value → out, and a waiting putter is made ready (401-402,
//   408-409); empty and open → it notes itself as a taker and parks (403-
//   405, 414); empty and closed → the loop ends (411-412).
// - close: the flag is set and every waiter is woken (239-248).
// - Making a task ready doesn't switch to it: the one running keeps the
//   turn until it waits (runtime/core/src/tasks.rs:389-397, 463-471).
// The turn order drawn was also seen in a run with a print around each
// put and take.
// The picture shows the channel's room, 2 values, as a ring. In memory
// the VecDeque is itself a ring buffer, which may have more slots than
// the capacity; values never move inside it (PLATFORM, Rust's std).

const lanes = ["main", "producer"];
const core = "cpu.core0";
// Far apart, so their labels never touch, even on a phone.
const chips = { main: "ram.chip0", producer: "ram.chip3", heap: "ram.chip6" };

type Who = Partial<Record<"main" | "producer", LaneState>>;
type Ring = Pick<RingView, "cells" | "head" | "tail" | "closed" | "waiting">;

const TITLE = "The channel's queue, on the heap: room for 2";

function step(line: number, at: string, who: Who, where: Record<string, string>, ring: Ring, caption: string, output?: string): SceneFrame {
  const running = (Object.keys(who) as (keyof Who)[]).find((k) => who[k] === "run");
  const labels: Record<string, string> = { [chips.main]: "main stack", [chips.heap]: "channel" };
  if (who.producer) labels[chips.producer] = who.producer === "done" ? "freed" : "producer";
  return {
    line,
    at,
    caption,
    lit: [core, chips.heap, ...(running ? [chips[running]] : [])],
    labels: { ...labels, [core]: running ?? "" },
    lanes: who,
    where,
    ring: { title: TITLE, ...ring },
    output,
  };
}

const empty = (head: number, more: Partial<Ring> = {}): Ring => ({ cells: [null, null], head, tail: head, ...more });
const out = (n: number) => ["10", "20", "30", "40"].slice(0, n).join("\n") || undefined;
const takes = [{ who: "main", for: "item" as const }];
const puts = [{ who: "producer", for: "room" as const }];

export const channelScene: Scene = {
  id: "channel",
  title: "A channel between two tasks: full makes the sender wait, empty the receiver",
  caveman: "Channel is small line of box. Line full? Putter wait. Line empty? Taker wait. Waiting task not hold core. Close? Taker stop when line empty.",
  sample: "explain/channel.native",
  code: "file",
  board: [
    { kind: "cpu", col: 0, row: 0, cores: 4, label: "CPU: both tasks, one thread" },
    { kind: "ram", col: 1, row: 0, label: "RAM: stacks and the channel" },
  ],
  lanes,
  lanesAre: "tasks on main's thread",
  panels: ["timeline"],
  frames: [
    step(1, "capacity: 2", { main: "run" }, { main: "main's thread" }, empty(0),
      "Channel<Int>(capacity: 2) makes one channel object on the heap; send and receive both point to it. It holds a queue with room for 2 values, two lists of tasks waiting (to take, and to put), and a lock the runtime holds only while a value or a waiter goes in or out."),
    step(2, "spawn {", { main: "run", producer: "ready" }, { main: "main's thread", producer: "ready queue" }, empty(0),
      "The producer becomes a coroutine on main's thread (the runtime, while running), ready. main keeps the turn."),
    step(8, "for n in receive", { main: "wait", producer: "run" }, { main: "waits for a value", producer: "main's thread" }, empty(0, { waiting: takes }),
      "main takes: the queue is empty and open, so the runtime, while running, notes main as a taker and parks it. The scheduler gives the turn to the producer."),
    step(4, "send.put(i * 10)", { main: "ready", producer: "run" }, { main: "ready: a value came", producer: "main's thread" }, { cells: ["10", null], head: 0, tail: 1 },
      "put(10): there is room, so 10 goes in, and the runtime makes the first waiting taker, main, ready. Ready doesn't mean running: the producer keeps the turn until it waits."),
    step(4, "send.put(i * 10)", { main: "ready", producer: "run" }, { main: "ready", producer: "main's thread" }, { cells: ["10", "20"], head: 0, tail: 0 },
      "put(20): room for one more. Now the queue holds 2 values: it is full."),
    step(4, "send.put(i * 10)", { main: "run", producer: "wait" }, { main: "main's thread", producer: "waits for room" }, { cells: ["10", "20"], head: 0, tail: 0, waiting: puts },
      "put(30): full. The runtime, while running, notes the producer as a putter and parks it, so a fast producer can't fill memory. The scheduler runs main, ready since put(10)."),
    step(9, "print(n)", { main: "run", producer: "ready" }, { main: "main's thread", producer: "ready: room came" }, { cells: [null, "20"], head: 1, tail: 0 },
      "main takes 10, the oldest value. That makes room, so the runtime makes the waiting producer ready. main keeps the turn and prints 10.", out(1)),
    step(9, "print(n)", { main: "run", producer: "ready" }, { main: "main's thread", producer: "ready" }, empty(0),
      "main takes 20 and prints it. The queue is empty again.", out(2)),
    step(8, "for n in receive", { main: "wait", producer: "run" }, { main: "waits for a value", producer: "main's thread" }, empty(0, { waiting: takes }),
      "main takes again: empty, so the runtime parks it. The producer runs: its put(30) tries again.", out(2)),
    step(4, "send.put(i * 10)", { main: "ready", producer: "run" }, { main: "ready: a value came", producer: "main's thread" }, { cells: ["30", null], head: 0, tail: 1 },
      "30 goes in, and the runtime makes main ready.", out(2)),
    step(4, "send.put(i * 10)", { main: "ready", producer: "run" }, { main: "ready", producer: "main's thread" }, { cells: ["30", "40"], head: 0, tail: 0 },
      "40 goes in: full again, but the loop is over.", out(2)),
    step(6, "send.close()", { main: "ready", producer: "run" }, { main: "ready", producer: "main's thread" }, { cells: ["30", "40"], head: 0, tail: 0, closed: true },
      "close() marks the channel closed and wakes every task waiting on it; none is. The values in it stay to be taken. Then the producer ends, and its stack is freed.", out(2)),
    step(9, "print(n)", { main: "run", producer: "done" }, { main: "main's thread", producer: "ended" }, { cells: [null, "40"], head: 1, tail: 0, closed: true },
      "main runs: it takes 30 and prints it.", out(3)),
    step(9, "print(n)", { main: "run", producer: "done" }, { main: "main's thread", producer: "ended" }, empty(0, { closed: true }),
      "Then 40.", out(4)),
    step(8, "for n in receive", { main: "run", producer: "done" }, { main: "main's thread", producer: "ended" }, empty(0, { closed: true }),
      "main takes once more: empty and closed, so the loop ends, without waiting. The values came out in the order they went in.", out(4)),
  ],
  note: "Native code, one thread. With tasks on several cores (spawn cores in an LLVM build), each side may run on its own worker, and a value made of objects is copied as it goes in, so no two threads share it. The ring stands for the queue's room: in memory it is a Rust VecDeque, which may keep more slots than the capacity, and the channel lets at most 2 in.",
  spec: "9.4",
};
