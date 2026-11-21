import type { HeapObject, Scene, SceneFrame } from "./types";

// print, line by line: each line is built whole in memory, then written
// in one go, through the OS kernel to the terminal. The sample is
// explain/print-line.kav (`kavach run`). One picture is true in every
// mode (the maintainer's verdict, 2026-11-21); the page's text gives the
// detail for native code on one core. Checked in the code:
// - The interpreter builds the value's text, adds "\n" to the task's
//   output. When the output is live, as in `kavach run`, it then writes
//   it to stdout and flushes, every print; otherwise (the interpreter run
//   from Rust, as tests do) the lines are gathered and handed back at the
//   end (compiler/interp/src/builtins.rs, "print"; tasks.rs, `flush`;
//   lib.rs, `run_live_with_args`).
// - Native code on several workers keeps a line's pieces in a line of its
//   own and writes the whole line at its end (runtime/native/src/rt.rs,
//   LINE, `kv_print_end`). On one core, each piece goes to Rust's
//   stdout, which holds bytes until a newline (rt.rs, `piece`, `out`).
// - The compiler works out the values of a print that can wait before
//   writing any of it, so a line never stops half way
//   (compiler/kir/src/whole_lines.rs). Spec 9.8: a task's lines come out
//   whole.
// The cells are 8 bytes each, as memory is drawn everywhere in the kit.

const core = "cpu.core0";
const buffer = "ram.chip2";
const kernel = "kernel.files";
const text = "screen.text";

/** The line being built, in 8-byte cells. */
const line = (...cells: string[]): HeapObject[] => [{ title: "the line being built (8 bytes a cell)", cells: cells.map((c) => ({ text: c })) }];
const empty = line("empty");

/** A step on core 0 with the line buffer as it is. */
function at(n: number, code: string, word: string, heap: HeapObject[], caption: string, extra: Partial<SceneFrame> = {}): SceneFrame {
  // The terminal keeps showing the last line written.
  const last = extra.output?.split("\n").pop();
  const labels: Record<string, string> = { [core]: word, [buffer]: "line", ...(last ? { [text]: last } : {}) };
  return { line: n, at: code, caption, heap, lit: [core, buffer], labels, ...extra };
}

/** The whole line goes to the kernel, then to the terminal, which shows `shown`. */
function written(n: number, code: string, bytes: string, shown: string, output: string, caption: string): SceneFrame {
  return {
    line: n,
    at: code,
    caption,
    heap: empty,
    lit: [buffer, kernel, text],
    labels: { [buffer]: "line", [kernel]: "stdout", [text]: shown },
    packets: [
      { from: buffer, to: kernel, label: bytes },
      { from: kernel, to: text, label: shown },
    ],
    output,
  };
}

export const printScene: Scene = {
  id: "print",
  title: "print: a line is built, then written in one go",
  caveman: "Print make whole line first, in memory. Number turn into digit. Line end with newline. Then whole line go to OS. OS give to screen. No half line.",
  sample: "explain/print-line",
  code: "file",
  board: [
    { kind: "cpu", col: 0, row: 0, cores: 4, label: "CPU: runs print" },
    { kind: "ram", col: 1, row: 0, label: "RAM: the line being built" },
    { kind: "screen", col: 1, row: 1, label: "Screen: the terminal" },
    { kind: "kernel", col: 0, row: 1 },
  ],
  panels: ["heap"],
  spec: "1.10",
  frames: [
    at(2, 'print("Hello")', "print", line("Hello"), "print starts a line in a buffer in RAM. \"Hello\" is text, so it is bytes already: its 5 bytes are copied in.", {
      packets: [{ from: core, to: buffer, label: "Hello" }],
    }),
    at(2, 'print("Hello")', "print", line("Hello\\n"), "print adds the end of the line, one newline byte (\\n). The line is whole: 6 bytes."),
    written(
      2,
      'print("Hello")',
      "Hello\\n",
      "Hello",
      "Hello",
      "Now the line is written in one go: a system call hands its 6 bytes to the OS kernel, which passes them to the terminal, and the terminal draws them. The runtime decides when, while running: once the line is whole. The next line starts in an empty buffer.",
    ),
    at(3, "price * 3", "40 × 3", empty, "Before print can start, price * 3 is worked out in the core: 120, a whole number. A number is held in binary, not as text.", { output: "Hello" }),
    at(3, "print(price * 3)", "120 → text", line("120\\n"), "The runtime, while running, turns 120 into the digits 1, 2 and 0: three bytes of text. Then the newline: the line is whole, 4 bytes.", {
      packets: [{ from: core, to: buffer, label: "120" }],
      output: "Hello",
    }),
    written(3, "print(price * 3)", "120\\n", "120", "Hello\n120", "Written in one go, like the first: the terminal shows 120 on a line of its own."),
    at(4, "print([1, 2, 3])", "[1, 2, 3]", line("[1, 2, 3", "]\\n"), "The list is turned into text piece by piece: [, then 1, then a comma and a space, and so on to ], then the newline. 10 bytes: more than one 8-byte cell. Nothing is written while the line is half made.", {
      packets: [{ from: core, to: buffer, label: "[1, 2, 3]" }],
      output: "Hello\n120",
    }),
    written(
      4,
      "print([1, 2, 3])",
      "[1, 2, 3]\\n",
      "[1, 2, 3]",
      "Hello\n120\n[1, 2, 3]",
      "The whole line goes to the kernel in one go. So another task's print can never land in the middle of it: each line comes out whole.",
    ),
  ],
  note: "The same picture holds for kavach run, kavach run --native and kavach build. The output can also go to a file or a pipe instead of a terminal: the program writes it the same way. kavach run writes and flushes each line as it is printed, because its output is shown live. The buffer is drawn without addresses.",
};
