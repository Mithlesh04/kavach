import type { PipelineView, Scene, SceneFrame } from "./types";

// From a .kav file to a running program, three ways: `kavach run` (the
// interpreter), `kavach run --native` (Cranelift, in memory) and `kavach
// build` (LLVM, an executable). The sample is explain/compiler.kav. Facts,
// checked in the code (2026-11-21):
// - The first stages are the same for every command: lexer, parser,
//   resolver, checker (compiler/driver/src/lib.rs, `compile`). `kavach
//   check` stops after them (tools/cli/src/main.rs, "check").
// - `kavach run` hands the checked program to the interpreter
//   (tools/cli/src/main.rs → kavach_interp::run_live_with_args), which
//   runs every `map` on the CPU (compiler/interp/src/gpu_choice.rs:5-6).
// - `--native` and `build` lower to KIR, then write the WGSL kernel of a
//   `map` that may run on the GPU (tools/cli/src/native.rs, `lowered` →
//   kavach_backend_wgsl::attach). The program's start opens the GPU and
//   compiles the kernels on a background thread (wgsl lib.rs, GpuWarm;
//   runtime/native/src/rt_gpu.rs, `warm`). For this sample, main's first
//   call (KAVACH_CLIF=1) makes the 4,909-byte kernel text.
// - A map worth less than 2 ms of the CPU's work stays on the CPU
//   (rt_gpu.rs, WORTH_NS). Checked: `KAVACH_GPU=off kavach run --native`
//   prints the same list as `kavach run` (the GPU itself was not run).
// - While the GPU is still opening, the bar is 300 ms (rt_gpu.rs,
//   OPENING_NS); KAVACH_GPU=on or check skips the cost check (`map`). The
//   interpreter touches no device unless KAVACH_GPU names one, by number
//   or maker, or can't be read (gpu_choice.rs; the checker's verdict).
// - `--native` compiles with Cranelift into memory and runs at once
//   (native.rs, `native`). `build` uses LLVM when it supports the program
//   and finds opt and llc, otherwise Cranelift (native.rs,
//   `build_with_best_backend`): LLVM IR text → `opt -O3` → `llc -O3` →
//   object file (llvm/src/lib.rs), linked with the runtime library by
//   link.exe on Windows or cc elsewhere (cranelift/src/link.rs). This
//   sample was built both ways; the default build used LLVM.

const FRONT = ["read the file", "lexer", "parser", "resolver", "checker"];
const RUN = [...FRONT, "interpreter"];
const NATIVE = [...FRONT, "KIR", "Cranelift", "runs in memory"];
const BUILD = [...FRONT, "KIR", "LLVM: opt, llc", "linker", "executable"];

const front = (label: string, stage: number): PipelineView => ({ title: "The first stages: the same for every command", stages: FRONT, items: [{ label, stage }] });
const run = (label: string, stage: number): PipelineView => ({ title: "kavach run: the interpreter", stages: RUN, items: [{ label, stage }] });
const native = (label: string, stage: number, notes?: Record<number, string>): PipelineView => ({
  title: "kavach run --native: Cranelift, in memory",
  stages: NATIVE,
  items: [{ label, stage }],
  notes,
});
const build = (label: string, stage: number): PipelineView => ({ title: "kavach build: LLVM, then the linker", stages: BUILD, items: [{ label, stage }] });

const core = "cpu.core0";
const OUT = "[2.0, 4.0, 6.0]";

/** A step where kavach works on the CPU and keeps what it made in RAM chip `chip`. */
function stage(line: number, at: string, chip: number, made: string, pipeline: PipelineView, caption: string, extra: Partial<SceneFrame> = {}): SceneFrame {
  return { line, at, caption, pipeline, lit: [core, `ram.chip${chip}`], labels: { [core]: "kavach", [`ram.chip${chip}`]: made }, ...extra };
}

export const compilerScene: Scene = {
  id: "compiler",
  title: "From a .kav file to a running program, three ways",
  caveman:
    "Read file. Cut in words. Make tree. Find names. Check types. Then three road. run: walk tree. run --native: make machine code in memory, go. build: make machine code, glue runtime, write file. GPU part get own code too.",
  sample: "explain/compiler",
  code: "file",
  board: [
    { kind: "ssd", col: 0, row: 0, label: "SSD: the .kav file, the executable" },
    { kind: "cpu", col: 1, row: 0, cores: 4, label: "CPU: runs kavach, then the program" },
    { kind: "ram", col: 0, row: 1, label: "RAM: what each stage makes" },
    { kind: "gpu", col: 1, row: 1, label: "GPU: only for the map's kernel" },
    { kind: "kernel", col: 0, row: 2 },
  ],
  spec: "10.5",
  frames: [
    {
      ...stage(0, "gpu fn double", 0, "text", front("text", 0), "You type kavach run compiler.kav. kavach is a program running on the CPU: it asks the OS kernel to read the file, and the file's bytes come from the SSD into RAM. The first five stages are the same for every command."),
      lit: [core, "ssd.nand0", "kernel.files", "ram.chip0"],
      labels: { [core]: "kavach", "ssd.nand0": "compiler.kav", "kernel.files": "read", "ram.chip0": "text" },
      packets: [
        { from: "ssd.nand0", to: "kernel.files", label: "compiler.kav" },
        { from: "kernel.files", to: "ram.chip0", label: "bytes" },
      ],
    },
    stage(6, "print(xs.map", 1, "tokens", front("tokens", 1), "The lexer cuts the text into tokens, the words of the language: print, (, xs, ., map, (, x, =>, double, and so on."),
    stage(6, "xs.map(x => double(x))", 2, "tree", front("tree", 2), "The parser arranges the tokens into a syntax tree: a call of print, holding a call of map on xs, holding a lambda that calls double."),
    stage(6, "double(x)", 3, "names", front("names", 3), "The resolver links each name to what it means: double to the gpu fn on line 1, xs to the const on line 6, print to the built-in print."),
    stage(5, "[Float32]", 4, "typed tree", front("typed tree", 4), "The checker works out every type and checks every rule: xs is a [Float32], and double takes and gives a Float32, so the call fits. kavach check stops here and says: no errors. A mistake stops every command here, before anything runs."),
    stage(6, "print(xs.map", 4, "typed tree", run("typed tree", 5), "kavach run makes no machine code. The interpreter, part of kavach, walks the checked tree and does what each part says. It runs every map on the CPU, one item after another, and prints the list. It doesn't touch the GPU at all, unless KAVACH_GPU names a device, by number or by maker (or can't be read): then it looks for that device, to check it is there.", {
      output: OUT,
    }),
    stage(1, "x * 2.0", 5, "KIR", native("KIR", 5), "kavach run --native goes on from the checked tree to KIR, a simpler form of the program made for code generators. Its passes, while compiling, mark each name's last use and copy small functions into their callers."),
    stage(0, "gpu fn double", 6, "WGSL", native("KIR", 5, { 5: "+ WGSL kernel for the GPU" }), "The map might be worth running on the GPU, so the compiler, while compiling, also writes its kernel in WGSL, the GPU's own language, and keeps that text in the program. The GPU is not touched yet."),
    stage(6, "print(xs.map", 7, "machine code", native("machine code", 6), "Cranelift turns the KIR into machine code for this CPU, straight into RAM. Nothing is written to the SSD."),
    {
      ...stage(6, "print(xs.map", 7, "machine code", native("machine code", 7), "The CPU runs that code at once. At its start, the runtime begins opening the GPU on a background thread, where the GPU's driver compiles the WGSL. A map of 3 items is far less work than using the GPU costs, so the runtime's cost check, while running, keeps it on the CPU. Once the GPU is open, a map goes there when it would take the CPU at least 2 ms; while the GPU is still opening, only one that would take 300 ms. KAVACH_GPU=on or check skips the cost check. The same list is printed.", {
        output: OUT,
      }),
      lit: [core, "ram.chip7", "gpu.group0"],
      labels: { [core]: "the program", "ram.chip7": "machine code", "gpu.group0": "opening" },
    },
    stage(6, "print(xs.map", 7, "object file", build("object file", 6), "kavach build makes the same KIR, then decides, while building: LLVM when it supports the program and its tools are found, else Cranelift. LLVM gets the KIR as LLVM IR text; opt -O3 makes it faster, and llc -O3 turns it into an object file of machine code."),
    {
      ...stage(4, "fn main()", 7, "object file", build("executable", 8), "The platform's linker (link.exe on Windows, cc elsewhere) joins the object file with Kavach's runtime library into one executable, and the OS writes it to the SSD. Nothing has run yet: you run that file later, as often as you like, without kavach."),
      lit: [core, "ram.chip7", "kernel.files", "ssd.nand1"],
      labels: { [core]: "linker", "ram.chip7": "object file", "kernel.files": "write", "ssd.nand1": "executable" },
      packets: [
        { from: "ram.chip7", to: "kernel.files", label: "machine code" },
        { from: "kernel.files", to: "ssd.nand1", label: "executable" },
      ],
    },
  ],
  note: "Each stage lives in its own folder of the compiler: compiler/lexer, parser, resolver, checker, interp, kir, and backend/cranelift, llvm and wgsl. The RAM chips are only places to show what each stage made; real memory isn't split up this way.",
};
