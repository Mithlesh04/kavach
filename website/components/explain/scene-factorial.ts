import { hex, stackAddr } from "./panels";
import type { CallFrame, LaneState, Scene, SceneFrame, Slot } from "./types";

// factorial(10), as the machine really runs it: on one CPU core, with the
// stack in RAM. The sample is explain/factorial.native.kav (`kavach run
// --native`). Checked on Windows x86-64 (2026-11-21, compiler e3256594)
// in Cranelift's IR (`KAVACH_CLIF=1`) and in programs built with
// `KAVACH_BACKEND=cranelift kavach build` and with LLVM (`kavach build`),
// disassembled with llvm-objdump:
// - Before Cranelift or LLVM sees it, KIR copies factorial into its
//   callers and into itself (compiler/kir/src/inline.rs: a small function
//   is inlined, `inlinable`; a recursive one gets RECURSION_LEVELS = 2
//   levels, `unroll`), and puts factorial's quick exit, `n <= 1`, in front
//   of each call left (inline/guard.rs). So main does the first level
//   itself, checks 9 <= 1, then calls factorial(9); factorial's own code
//   does four levels (n, n - 1, n - 2, n - 3), checks n - 4 <= 1, and
//   only then calls factorial(n - 4). factorial(10) makes two real calls:
//   factorial(9) and factorial(5). Both builds make the same two calls.
// - `n * f(n - 1)` is not turned into a loop (unrecurse.rs takes only
//   `e + f(step)` or `f(step)`), LLVM calls are `notail`, and Cranelift
//   makes no tail calls.
// - Before each real call, the code compares the stack pointer with a
//   limit and panics past it (cranelift codegen.rs, `check_stack`; LLVM
//   func.rs, `stack_check`). The limit sits 8 MB above the stack's bottom
//   (runtime/native/src/rt.rs, STACK_RESERVE), so it fires first; the
//   guard page below is only a backstop.
// - The platform's calling convention (cranelift abi.rs, make_signature):
//   the argument in rcx on Windows (rdi on Linux and macOS), the result in
//   rax. Cranelift's factorial keeps n in rbx and n - 1 in rsi across its
//   call; the callee saves rbx, rsi, rdi and rbp in its own frame and puts
//   them back before it returns. Every multiply is followed by `seto`.
// UNVERIFIED: frames on Linux and macOS (other registers are saved there).

const ARG = "rdi / rcx";
const lanes = ["main", "factorial(9)", "factorial(5)"];

const main: CallFrame = {
  fn: "main()",
  slots: [{ name: "return address", value: "→ runtime", note: "main goes back to Kavach's runtime when it ends" }],
};

/** A real call's frame: where to go back, and the caller's two registers it saved. */
function call(arg: number, back: string, rbx: string, rsi: string, whose: string): CallFrame {
  return {
    fn: `factorial(${arg})`,
    slots: [
      { name: "return address", value: back, note: "where the caller goes on: written by the call" },
      { name: "saved rbx", value: rbx, note: `${whose}, put back on return` },
      { name: "saved rsi", value: rsi, note: `${whose}, put back on return` },
    ],
  };
}

const f9 = call(9, "→ line 9", "main's", "main's", "main's value");
const f5 = call(5, "→ line 5", "9", "8", "factorial(9)'s n and n - 1");

/** The registers: the argument, n and n - 1 kept across a call, the result, and the stack pointer. */
function regs(s: CallFrame[], arg: string, n: string, n1: string, result: string): Slot[] {
  const slots = s.reduce((k, f) => k + f.slots.length, 0);
  return [
    { name: ARG, value: arg, note: "the argument: rdi on Linux and macOS, rcx on Windows" },
    { name: "rbx", value: n, note: "n, kept here across the inner call" },
    { name: "rsi", value: n1, note: "n - 1, kept here too" },
    { name: "rax", value: result, note: "a function's result comes back here" },
    { name: "rsp", value: `…${hex(stackAddr(slots - 1)).slice(-4)}`, note: "stack pointer: the newest slot" },
  ];
}

/** Who runs: the deepest real call runs, every call above it waits. */
function who(depth: number): Record<string, LaneState> {
  const out: Record<string, LaneState> = {};
  for (let i = 0; i <= depth; i++) out[lanes[i]] = i === depth ? "run" : "wait";
  return out;
}

const core = "cpu.core0";
const stackChip = "ram.chip1";

/** A step on core 0 at real-call depth `depth`, with this stack and these registers. */
function at(line: number, text: string, depth: number, word: string, stack: CallFrame[], r: Slot[], extra: Partial<SceneFrame> & { caption: string }): SceneFrame {
  return { line, at: text, lit: [core, stackChip], labels: { [core]: word, [stackChip]: "stack" }, stack, regs: r, lanes: who(depth), ...extra };
}

const s0 = [main];
const s1 = [main, f9];
const s2 = [main, f9, f5];

export const factorialScene: Scene = {
  id: "factorial",
  title: "factorial(10): the calls that really happen",
  caveman:
    "Compiler copy factorial into itself. One real call do four step. main do 10, call 9. factorial(9) do 9, 8, 7, 6, call 5. factorial(5) do 5, 4, 3, 2. 1? Stop, no call. Answers multiply back. Two box on stack, not ten.",
  sample: "explain/factorial.native",
  code: "file",
  board: [
    { kind: "cpu", col: 0, row: 0, cores: 4, label: "CPU: core 0 runs it" },
    { kind: "ram", col: 1, row: 0, label: "RAM: the stack lives here" },
    { kind: "kernel", col: 0, row: 1 },
  ],
  lanes,
  lanesAre: "real calls on core 0",
  panels: ["regs", "stack", "heap", "timeline"],
  heapEmpty: "Nothing of factorial's: whole numbers live in registers and on the stack, never on the heap.",
  frames: [
    at(8, "factorial(10)", 0, "10 <= 1?", s0, regs(s0, "—", "—", "—", "?"), {
      caption:
        "The compiler, while compiling, copied factorial into main once, so main does the first level itself: 10 is not <= 1. It also copied factorial's quick exit in front of the call: 9 <= 1? No. The OS picked a core to run it (here core 0), while running.",
    }),
    at(8, "factorial(10)", 0, "call 9", s1, regs(s1, "9", "—", "—", "?"), {
      packets: [{ from: core, to: stackChip, label: "return address" }],
      caption:
        "A real call, factorial(9), with 9 in the argument register. First a check the compiler put there: is there room on the stack? The limit sits 8 MB above the stack's bottom, so this check fires long before the guard page below, which is only a backstop. Then the call writes the return address and jumps.",
    }),
    at(0, "fn factorial", 1, "factorial(9)", s1, regs(s1, "9", "9", "8", "?"), {
      packets: [{ from: core, to: stackChip, label: "saved registers" }],
      caption:
        "factorial(9) begins. Its first instructions save registers of main's it will use, into its own frame. Then it keeps n = 9 in rbx and n - 1 = 8 in rsi.",
    }),
    at(1, "if n <= 1", 1, "9, 8, 7, 6", s1, regs(s1, "9", "9", "8", "?"), {
      caption:
        "The compiler copied factorial into itself, so this one call checks four levels at once, all in registers: 9, 8, 7 and 6 are not <= 1. No call is made for 8, 7 or 6.",
    }),
    at(4, "n * factorial(n - 1)", 1, "call 5", s2, regs(s2, "5", "9", "8", "?"), {
      packets: [{ from: core, to: stackChip, label: "return address" }],
      caption:
        "The quick exit for the next level: 5 <= 1? No. So, after the same stack check, a second real call: factorial(5). 9 and 8 wait in rbx and rsi.",
    }),
    at(0, "fn factorial", 2, "factorial(5)", s2, regs(s2, "5", "5", "4", "?"), {
      packets: [{ from: core, to: stackChip, label: "9, 8" }],
      caption:
        "factorial(5) begins, with the same code. It needs rbx and rsi itself, so it first saves factorial(9)'s 9 and 8 into its own frame. That is where the waiting values really are.",
    }),
    at(1, "if n <= 1", 2, "1 <= 1", s2, regs(s2, "5", "5", "4", "?"), {
      caption: "5, 4, 3 and 2 are not <= 1. Then the quick exit: 1 <= 1 is true. factorial(1) is just 1, and no call is made.",
    }),
    at(4, "n * factorial", 2, "5×4×3×2×1", s2, regs(s2, "5", "5", "4", "120"), {
      caption: "On the way back, the multiplies run in registers: 2 × 1 = 2, 3 × 2 = 6, 4 × 6 = 24, 5 × 24 = 120. Every multiply is checked for overflow. 120 goes in rax.",
    }),
    at(4, "n * factorial", 1, "return 120", s1, regs(s1, "—", "9", "8", "120"), {
      packets: [{ from: stackChip, to: core, label: "9, 8" }],
      caption:
        "factorial(5) puts 9 and 8 back into rbx and rsi, reads its return address and jumps back. Its frame is popped by code the compiler wrote: no count, no search, those bytes are free again at once.",
    }),
    at(4, "n * factorial", 0, "6×7×8×9", s0, regs(s0, "—", "—", "—", "362880"), {
      packets: [{ from: stackChip, to: core, label: "return address" }],
      caption:
        "Back in factorial(9): 6 × 120 = 720, 7 × 720 = 5040, 8 × 5040 = 40320, 9 × 40320 = 362880. It puts main's registers back and returns. Only main's frame is left.",
    }),
    at(8, "const x = factorial(10)", 0, "10 × 362880", s0, regs(s0, "—", "—", "—", "3628800"), {
      caption: "main does its own level: 10 × 362880 = 3628800, so x is 3628800. Two real calls did the work of ten.",
    }),
    {
      ...at(9, "print(x)", 0, "print", s0, regs(s0, "—", "—", "—", "3628800"), {
        caption: "print asks the operating system's kernel to write 3628800 to the program's output. Each real call's frame was freed the moment it returned.",
      }),
      lit: [core, "kernel.files"],
      labels: { [core]: "print", "kernel.files": "output" },
      packets: [{ from: core, to: "kernel.files", label: "\"3628800\"" }],
      output: "3628800",
    },
  ],
  note: "Who decides how many calls: the compiler, while compiling. kavach run (the interpreter) makes every call as written: ten for factorial(10). kavach run --native makes the calls left after inlining: two, as drawn. A kavach build with LLVM, when checked, made the same two. Simplified, as Cranelift built it on Windows x86-64: frames also save rbp and rdi, which aren't drawn, and other registers on Linux and macOS. Addresses are examples: they differ on every run.",
};
