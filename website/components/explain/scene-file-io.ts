import { TINT, checkHeap, freeAt, header, place } from "./heap-cells";
import type { HeapObject, Scene, SceneFrame } from "./types";

// Writing a file, reading it whole, then line by line and in pieces: the
// sample explain/file-io.native.kav (`kavach run --native`; it runs in
// the git-ignored .kavach/sample-runs folder, as every sample does).
// Checked in the source (2026-11-21):
// - write_text checks the task's folders, then writes a temporary file
//   beside the target and renames it over it, with no fsync; write_durable
//   forces the file and its folder to the disk (runtime/native/src/
//   rt_std.rs:180-195, 223-236; runtime/core/src/files.rs:19-74). The text
//   it writes is let go there (rt_std.rs `text`, 26-30).
// - read_text reads the whole file with std::fs::read_to_string and makes
//   one text value (rt_std.rs:196-202); text up to 30 bytes sits inside its
//   64-byte object (runtime/native/src/text.rs:5-17, From<String> 71-78).
// - fs.lines and fs.chunks open a reader over a Rust BufReader
//   (rt_files.rs open, 116-135); lines cuts at \n with read_until and
//   makes one text value per line (148-170); pieces under 1 MB are read
//   when asked (183-192), pieces of 1 MB or more one piece ahead on a
//   helper thread (READ_AHEAD, 19-22; decision 0067).
// - print takes the value and lets it go (rt_strings.rs kv_print_value).
// PLATFORM, not Kavach code: the OS page cache, when the OS writes it to
// the SSD, and BufReader's 8 KB buffer (Rust's default today).
// - Pieces of 1 MB or more are read ahead through a cloned file handle,
//   past the BufReader (rt_files.rs:76-108).
// Not drawn: when the readers are let go. A reader is a local of main, and
// native main leaves what its locals hold to the OS at exit, in Cranelift
// and LLVM alike (cranelift/src/codegen.rs:137-169, llvm/src/func.rs:
// 339-346); the interpreter frees everything when main ends.

const core = "cpu.core0";
const prog = "ram.chip1";
const cache = "ram.chip6";
const disk = "ssd.nand0";
const files = "kernel.files";

type State = "none" | "live" | "freed";
type View = { write: State; all: State; reader: State; buf?: boolean; line1: State; line2: State; chunks: State };

/** Every place the scene uses, in address order: 64-byte objects from 0x1000, the readers' buffers from 0x4000. */
function heap(v: View): HeapObject[] {
  const text = (title: string, base: number, state: State, body: string, tint: string) =>
    state === "none" ? freeAt(base, 4) : place(title, base, [...header(state === "freed" ? 0 : 1), { text: body, note: "the bytes, inside" }], { tint, freed: state === "freed" });
  const reader = (title: string, base: number, state: State) =>
    state === "none" ? freeAt(base, 4) : place(title, base, [...header(1), { text: "→ file", note: "the open file, its buffer" }], { tint: TINT.c });
  const buffer = (title: string, base: number, state: State, filled: boolean) =>
    state === "none"
      ? freeAt(base, 3)
      : place(title, base, filled ? [{ text: "tea\\ncake", note: "8 bytes" }, { text: "\\n", note: "1 byte" }, { text: "·", note: "8,183 bytes of room" }] : ["·", "·", { text: "·", note: "8 KB of room" }], { tint: TINT.c });
  return [
    text("the text to write", 0x1000, v.write, "tea\\ncake\\n", TINT.a),
    text("all: one text value", 0x1040, v.all, "tea\\ncake\\n", TINT.b),
    reader("the lines reader", 0x1080, v.reader),
    text("line: \"tea\"", 0x10c0, v.line1, "tea", TINT.d),
    text("line: \"cake\"", 0x1100, v.line2, "cake", TINT.d),
    reader("the chunks reader", 0x1140, v.chunks),
    buffer("the lines reader's buffer, 8 KB", 0x4000, v.reader, v.buf ?? false),
    buffer("the chunks reader's buffer, 8 KB", 0x6000, v.chunks, v.chunks !== "none"),
  ];
}

const NONE: View = { write: "none", all: "none", reader: "none", line1: "none", line2: "none", chunks: "none" };
const READ: View = { ...NONE, write: "freed", all: "live" };
const LINES: View = { ...READ, reader: "live", buf: true };

const frames: SceneFrame[] = [
  {
    line: 3,
    at: "fs.write_text",
    lit: [core, prog, cache, files],
    labels: { [prog]: "the text", [cache]: "page cache", [files]: "write" },
    packets: [
      { from: core, to: files, label: "create, write" },
      { from: prog, to: cache, label: "9 bytes" },
    ],
    heap: heap({ ...NONE, write: "live" }),
    caption:
      "The runtime, while running, checks that notes.txt is inside the folders this task may use. Then it asks the OS, through system calls, to make a temporary file beside it and write the 9 bytes. During that call, the OS copies them from the program's memory into its page cache: memory in RAM that the OS keeps for files.",
  },
  {
    line: 3,
    at: "fs.write_text",
    lit: [core, cache, files],
    labels: { [cache]: "page cache", [files]: "rename" },
    packets: [
      { from: core, to: files, label: "rename" },
      { from: cache, to: disk, label: "later" },
    ],
    heap: heap({ ...NONE, write: "freed" }),
    caption:
      "Then the runtime renames the temporary file over notes.txt, so a crash leaves the old file or the new one, never half of one. There is no wait for the disk: the OS decides, while the program goes on, when to copy the pages to the SSD, usually seconds later. fs.write_durable waits for the disk instead. The text was the write's to let go: freed.",
  },
  {
    line: 4,
    at: "fs.read_text",
    lit: [core, prog, cache, files],
    labels: { [prog]: "all", [cache]: "page cache", [files]: "read" },
    packets: [
      { from: core, to: files, label: "open, read" },
      { from: cache, to: prog, label: "9 bytes" },
    ],
    heap: heap(READ),
    caption:
      "read_text reads the whole file at once. Its pages are still in the page cache, so the OS copies them from there into the program's memory, most likely without touching the SSD; if they weren't, the OS would first read them from the SSD into the cache. The runtime makes one text value: up to 30 bytes fit inside its 64-byte object.",
  },
  {
    line: 5,
    at: "print(all.len())",
    lit: [core, files],
    labels: { [files]: "output" },
    heap: heap(READ),
    output: "9",
    caption: "9 characters: \"tea\", a line break, \"cake\", a line break.",
  },
  {
    line: 6,
    at: "fs.lines",
    lit: [core, prog, cache, files],
    labels: { [prog]: "reader's buffer", [cache]: "page cache", [files]: "read" },
    packets: [
      { from: core, to: files, label: "open, read" },
      { from: cache, to: prog, label: "up to 8 KB" },
    ],
    heap: heap(LINES),
    output: "9",
    caption:
      "fs.lines opens the file and gives a reader with an 8 KB buffer, Rust's default size. The first line asked for makes one read call for up to 8 KB: the whole 9-byte file lands in the buffer. A big file comes in 8 KB at a time, never all at once.",
  },
  {
    line: 7,
    at: "print(line)",
    lit: [core, prog, files],
    labels: { [prog]: "\"tea\"", [files]: "output" },
    heap: heap({ ...LINES, line1: "live" }),
    output: "9\ntea",
    caption: "The runtime cuts the buffer at the first line break and makes a new text value, \"tea\". print writes it.",
  },
  {
    line: 7,
    at: "print(line)",
    lit: [core, prog, files],
    labels: { [prog]: "\"cake\"", [files]: "output" },
    heap: heap({ ...LINES, line1: "freed", line2: "live" }),
    output: "9\ntea\ncake",
    caption:
      "\"cake\" comes from the same buffer: no system call this time. Each round's line is let go at its last use, the print: the compiler marked it while compiling, and print frees it. The next read finds the end of the file, and the loop stops.",
  },
  {
    line: 9,
    at: "fs.chunks",
    lit: [core, prog, cache, files],
    labels: { [prog]: "chunks' buffer", [cache]: "page cache", [files]: "read" },
    packets: [{ from: cache, to: prog, label: "up to 8 KB" }],
    heap: heap({ ...LINES, line1: "freed", line2: "freed", chunks: "live" }),
    output: "9\ntea\ncake",
    caption:
      "fs.chunks opens a reader of its own, also with an 8 KB buffer. Each piece is a new Bytes value of at most 4 bytes. Pieces this small are read through that buffer, when the loop asks. Pieces of 1 MB or more skip the buffer: a helper thread the runtime starts reads them through a second handle to the file, one piece ahead, so the disk and the loop work at the same time.",
  },
  {
    line: 10,
    at: "print(piece.len())",
    lit: [core, files],
    labels: { [files]: "output" },
    heap: heap({ ...LINES, line1: "freed", line2: "freed", chunks: "live" }),
    output: "9\ntea\ncake\n4\n4\n1",
    caption: "Three pieces: 4 bytes, 4 bytes, then the last 1.",
  },
  {
    line: 12,
    at: "fs.remove",
    lit: [core, files],
    labels: { [files]: "remove" },
    packets: [{ from: core, to: files, label: "remove" }],
    heap: heap({ ...LINES, line1: "freed", line2: "freed", chunks: "live" }),
    output: "9\ntea\ncake\n4\n4\n1",
    caption: "fs.remove asks the OS to remove notes.txt.",
  },
];

checkHeap("file-io", frames);

export const fileIoScene: Scene = {
  id: "file-io",
  title: "A file's bytes: program, page cache, SSD",
  caveman: "Write: bytes go to OS. OS keep them in RAM, write to disk later. Read: OS give bytes back from RAM, fast. Lines: small bucket, fill, take one line, fill again.",
  sample: "explain/file-io.native",
  code: "file",
  board: [
    { kind: "cpu", col: 0, row: 0, label: "CPU: core 0 runs it" },
    { kind: "ram", col: 1, row: 0, label: "RAM: program and page cache" },
    { kind: "ssd", col: 1, row: 1, label: "SSD: where files are kept" },
    { kind: "kernel", col: 0, row: 2 },
  ],
  panels: ["heap"],
  lastUse: { 5: ["all"], 7: ["line"], 10: ["piece"] },
  spec: "14.3",
  frames,
  note: "Drawn for kavach run --native. The page cache and when it reaches the SSD are the operating system's work, the same on Linux, macOS and Windows in outline. In native code, Cranelift or LLVM alike, main's locals keep all and the readers to the end, and the operating system takes that memory back when the program ends, so they are not drawn freed. kavach run, the interpreter, frees everything when main ends. Addresses are examples.",
};
