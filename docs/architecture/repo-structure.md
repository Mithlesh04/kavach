# Repository Structure

A clear structure matters more as the project grows. This page shows where everything lives and the rules for adding new folders.

## The tree

```
kavach/
├── .cargo/                 cargo settings: compiler flags for C dependencies
├── .github/                CI workflow, issue forms, PR template
├── README.md               project front page
├── AGENTS.md               rules for every change (people and AI)
├── CLAUDE.md               loads AGENTS.md for Claude Code
├── CONTRIBUTING.md         how to contribute
├── llms.txt                the whole language for AI assistants, under 4,000 tokens
├── SECURITY.md             how to report vulnerabilities
├── Cargo.toml              Rust workspace: every compiler, runtime and tool crate
├── Cargo.lock              exact Rust dependency versions (generated)
├── LICENSE-APACHE          license (Apache-2.0)
├── LICENSE-MIT             license (MIT); users pick either
│
├── docs/                   everything to read
│   ├── vision/             why Kavach exists, roadmap
│   ├── spec/               language specification (01-…, 02-…)
│   ├── architecture/       how the implementation is organized
│   └── decisions/          decision records (0001-…, 0002-…)
│
├── compiler/               source code → machine code
│   ├── lexer/
│   ├── parser/
│   ├── ast/
│   ├── resolver/
│   ├── checker/
│   ├── interp/
│   ├── kir/
│   ├── backend/
│   │   ├── cranelift/
│   │   ├── llvm/
│   │   └── wgsl/           GPU kernels of gpu fn
│   └── driver/
│
├── runtime/                linked into every program
│   ├── core/               shared by the interpreter and native code
│   ├── net/                networking, TLS and the model call, shared like core/
│   ├── gpu/                GPU compute for gpu fn, through wgpu
│   ├── window/             windows and their input for std.window, through winit
│   ├── process/            other programs for std.process, as trees (unsafe allowed, decision 0105)
│   ├── native/             run-time library of native programs
│   ├── unpack/             packed executables: the format and the C starter
│   ├── memory/
│   ├── tasks/
│   ├── agents/
│   └── durable/
│
├── stdlib/                 standard library, written in Kavach
│   ├── core/  collections/  text/  math/  time/
│   └── io/  net/  json/  ai/  test/
│
├── tools/                  developer tools
│   ├── cli/  fmt/  lsp/  pkg/  ai/  playground/
│
├── interop/                C, Python, JavaScript
├── editors/                VS Code (syntax highlighting now) and future editors
├── website/                the official docs site, mithlesh04.github.io/kavach (decision 0033)
├── examples/               example programs, numbered in learning order
├── tests/                  conformance + integration tests
├── benchmarks/             performance comparisons
└── scripts/                helper scripts for developing Kavach
```

## Where does it go?

The placement table and the steps for creating new files and folders are in [AGENTS.md, section 2](../../AGENTS.md#2-where-things-go). There is one table, so people and AI tools follow the same one.

## Rules

1. **Top level stays small.** A new top-level folder needs a decision record.
2. **One purpose per folder.** If you can't describe it in one line, split it.
3. **Names are short lowercase nouns.** Use `kebab-case` for multiple words (`refund-bot`, not `RefundBot` or `refund_bot`).
4. **Every top-level folder has a README.md** listing its subfolders in a table.
5. **Nest only when needed.** Create a subfolder when a folder has more than ~10 files or two clearly separate areas. Code should stay within 4 levels of the root.
6. **Numbers only where order matters:** spec chapters (`01-`), decision records (`0001-`), examples (`01-`).
7. **Spec before code.** A language change is merged into `docs/spec/` before or together with its implementation, never after.
8. **Tests mirror the spec.** `tests/conformance/` subfolders use the same numbers and names as spec chapters.
9. **No file over 500 lines**, counting every line. Split by responsibility. Checked by `scripts/check-repo.sh` (see [AGENTS.md](../../AGENTS.md)).

## User projects

The layout of a project *written in* Kavach (not this repository) is defined in [spec chapter 10](../spec/10-modules-and-packages/03-project-layout.md).
