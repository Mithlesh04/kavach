import { hex, stackAddr } from "./panels";
import type { CallFrame, LaneState, Scene, SceneFrame, Slot } from "./types";

// days_to_save(100, 30): one call, from the arguments going in to the
// result coming back, on one CPU core with the stack in RAM. The sample
// is explain/days-to-save.native.kav (`kavach run --native`). Checked on
// Windows x86-64 (2026-11-21), in Cranelift's IR (`KAVACH_CLIF=1`) and in
// a program built with `KAVACH_BACKEND=cranelift kavach build`,
// disassembled with llvm-objdump:
// - The call is real. Small functions are copied into their callers
//   (compiler/kir/src/inline.rs, `inlinable`), but not one that still
//   returns early after `lift_returns`, as a `return` inside a loop does
//   (`has_return`). main does `mov ecx, 100; mov edx, 30; call`.
// - Before the call, main compares the stack pointer with the limit
//   (cranelift codegen.rs `check_stack`): `cmp rsi, [limit]; jb` to a
//   cold path that panics.
// - Arguments and result follow the platform's calling convention
//   (cranelift abi.rs, `make_signature`): rcx and rdx on Windows (rdi and
//   rsi on Linux and macOS), the result in rax.
// - days_to_save's frame: `call` pushes the return address; its prologue
//   is `push rbp; mov rbp, rsp; sub rsp, 0x40` and saves rsi and rdi, which
//   Windows' convention has a function put back. goal, per_day, saved and
//   day stay in registers the whole call; each `+=` is followed by `seto`,
//   the overflow check. The epilogue puts rsi, rdi and rbp back, then `ret`.
// - main passes the result from rax straight to print (`mov rcx, rax`):
//   `days` is never in memory.
// - Seen again on Windows x86-64 at commit 54d0181e (2026-11-21): every
//   instruction above, in `KAVACH_BACKEND=cranelift kavach build`'s program.
// - `kavach build` (LLVM) at 54d0181e worked the call out while compiling:
//   the program does `mov ecx, 4` and calls print. That is what LLVM did in
//   this build, not a rule; a stack compare was still there before it.

const ARG1 = "rdi / rcx";
const ARG2 = "rsi / rdx";
const lanes = ["main", "days_to_save(100, 30)"];

const main: CallFrame = {
  fn: "main()",
  slots: [{ name: "return address", value: "→ runtime", note: "main goes back to Kavach's runtime when it ends" }],
};

/** days_to_save's frame: the return address, then (after its first instructions) the saved registers. */
function callee(saved: boolean): CallFrame {
  const back: Slot = { name: "return address", value: "→ line 13", note: "where main goes on: written by the call" };
  if (!saved) return { fn: "days_to_save(100, 30)", slots: [back] };
  const keep = (name: string): Slot => ({ name, value: "main's", note: "main's value, put back on return" });
  return { fn: "days_to_save(100, 30)", slots: [back, keep("saved rbp"), keep("saved rsi"), keep("saved rdi")] };
}

/** The registers that matter: the two arguments, the two locals, the result and the stack pointer. */
function regs(s: CallFrame[], saved = "—", day = "—", result = "?"): Slot[] {
  const slots = s.reduce((n, f) => n + f.slots.length, 0);
  return [
    { name: ARG1, value: "100", note: "goal, the 1st argument: rdi on Linux and macOS, rcx on Windows" },
    { name: ARG2, value: "30", note: "per_day, the 2nd argument" },
    { name: "saved", value: saved, note: "a local: in a register, never in memory" },
    { name: "day", value: day, note: "the loop's counter: in a register too" },
    { name: "rax", value: result, note: "a function's result comes back here" },
    { name: "rsp", value: `…${hex(stackAddr(slots - 1)).slice(-4)}`, note: "stack pointer: the newest slot" },
  ];
}

const core = "cpu.core0";
const stackChip = "ram.chip1";
const inMain: Record<string, LaneState> = { main: "run" };
const inCall: Record<string, LaneState> = { main: "wait", "days_to_save(100, 30)": "run" };

/** A step on core 0, with the stack and registers as they are then. */
function at(line: number, text: string, word: string, stack: CallFrame[], r: Slot[], extra: Partial<SceneFrame> & { caption: string }): SceneFrame {
  return {
    line,
    at: text,
    lit: [core, stackChip],
    labels: { [core]: word, [stackChip]: "stack" },
    stack,
    regs: r,
    lanes: stack.length > 1 ? inCall : inMain,
    ...extra,
  };
}

const before = [main];
const called = [main, callee(false)];
const framed = [main, callee(true)];

export const callScene: Scene = {
  id: "call",
  title: "days_to_save(100, 30): one call, step by step",
  caveman:
    "Main call function. 100 and 30 go in register. Call push way-back on stack. Function count in register only. Answer 4 go in rax. Frame pop. Main take 4.",
  sample: "explain/days-to-save.native",
  code: "file",
  board: [
    { kind: "cpu", col: 0, row: 0, cores: 4, label: "CPU: core 0 runs it" },
    { kind: "ram", col: 1, row: 0, label: "RAM: the stack lives here" },
    { kind: "kernel", col: 0, row: 1 },
  ],
  lanes,
  lanesAre: "calls on core 0",
  panels: ["regs", "stack", "heap", "timeline"],
  heapEmpty: "Nothing: whole numbers travel in registers and on the stack, never on the heap.",
  spec: "3.1",
  frames: [
    at(12, "days_to_save(100, 30)", "main", before, regs(before), {
      caption:
        "main calls days_to_save(100, 30). The OS picked a core to run it (here core 0), while running. The compiler, while compiling, put 100 and 30 straight into the first two argument registers, as the platform's calling convention says. It also kept this a real call: a function that returns from inside a loop is not copied into its caller.",
    }),
    at(12, "days_to_save(100, 30)", "room?", before, regs(before), {
      caption:
        "Before the call, a check the compiler put there runs: the core compares the stack pointer with the stack's limit. There is room, so the call goes on. Past the limit, the program would stop with a clear panic.",
    }),
    at(0, "fn days_to_save", "call", called, regs(called), {
      packets: [{ from: core, to: stackChip, label: "return address" }],
      caption:
        "The call writes the return address, where main goes on, onto the stack in RAM, and jumps to days_to_save. A new frame starts below main's.",
    }),
    at(0, "fn days_to_save", "days_to_save", framed, regs(framed), {
      packets: [{ from: core, to: stackChip, label: "saved registers" }],
      caption:
        "The function's first instructions, which the compiler wrote, save three of main's registers into the frame: main's frame pointer, and two registers this function will use. The convention says it must put them back, so main finds them as it left them.",
    }),
    at(1, "let saved = 0", "saved = 0", framed, regs(framed, "0"), {
      caption:
        "saved starts at 0, in a register. goal, per_day, saved and day all stay in registers for the whole call: the frame holds none of them.",
    }),
    at(3, "saved += per_day", "0 + 30", framed, regs(framed, "30", "1"), {
      caption: "Round 1: day is 1, and saved becomes 30. After each add, the core checks the overflow flag: a sum too big for an Int would stop the program.",
    }),
    at(4, "if saved >= goal", "30 >= 100?", framed, regs(framed, "30", "1"), {
      caption: "Is 30 at least 100? No, so the loop goes round again.",
    }),
    at(3, "saved += per_day", "+ 30", framed, regs(framed, "90", "3"), {
      caption: "Rounds 2 and 3: saved becomes 60, then 90. Each round only changes registers inside the core; nothing touches memory.",
    }),
    at(4, "if saved >= goal", "120 >= 100?", framed, regs(framed, "120", "4"), {
      caption: "Round 4: saved becomes 120, and 120 >= 100 is true.",
    }),
    at(5, "return day", "return 4", framed, regs(framed, "120", "4", "4"), {
      packets: [{ from: stackChip, to: core, label: "saved registers" }],
      caption:
        "return day puts 4 in rax, where a result goes back. The last instructions, which the compiler wrote, put main's three registers back from the frame.",
    }),
    at(5, "return day", "ret", before, regs(before, "—", "—", "4"), {
      packets: [{ from: stackChip, to: core, label: "return address" }],
      caption:
        "ret reads the return address from the stack and jumps back into main. The frame is popped: the stack pointer moves back up, and those bytes are free at once. Code the compiler wrote does it: nothing is counted or searched.",
    }),
    at(12, "const days", "days = 4", before, regs(before, "—", "—", "4"), {
      caption: "main takes the result from rax: days is 4. It never goes to memory either: it goes straight on to print.",
    }),
    {
      ...at(13, "print(days)", "print", before, regs(before, "—", "—", "4"), {
        caption: "print asks the operating system's kernel to write 4 to the program's output. The stack holds only main's frame again.",
      }),
      lit: [core, "kernel.files"],
      labels: { [core]: "print", "kernel.files": "output" },
      packets: [{ from: core, to: "kernel.files", label: "\"4\"" }],
      output: "4",
    },
  ],
  note: "Seen on Windows x86-64 at commit 54d0181e, in a program Cranelift built (the code kavach run --native runs), read with llvm-objdump: the arguments in rcx and rdx, the result in rax, and a frame with the return address and three saved registers. Real frames also keep some room the Windows convention asks for. On Linux and macOS the arguments come in rdi and rsi, and the frame may save other registers (not checked there). Built with kavach build instead, LLVM worked days_to_save(100, 30) out while compiling: the program put 4 in rcx and called print. That is what LLVM did there, not a rule, and the stack check before it was still there. Addresses are examples: they differ on every run.",
};
