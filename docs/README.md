# Kavach Documentation

## Reading order

| # | Document | Read it to learn |
|---|---|---|
| 1 | [vision/vision.md](vision/vision.md) | The problem, who Kavach is for, how it gets adopted |
| 2 | [vision/roadmap.md](vision/roadmap.md) | What gets built, in what order |
| 3 | [vision/platform.md](vision/platform.md) | The native platform: files, networking, GPU, UI, and when each arrives |
| 4 | [vision/todo.md](vision/todo.md) | Everything still to build, in the order it will be done |
| 5 | [vision/language-design-research.md](vision/language-design-research.md) | What new languages must get right (research, 2026), and how Kavach measures up |
| 6 | [vision/developer-needs.md](vision/developer-needs.md) | What developers want and need in a language (research, 2026), and what it means for Kavach |
| 7 | [vision/bug-classes.md](vision/bug-classes.md) | The critical bug classes of other languages (research, 2026): which Kavach rules out, which still need a rule |
| 8 | [vision/classic-mistakes.md](vision/classic-mistakes.md) | The everyday correctness traps of other languages (`0.1 + 0.2`, `NaN`, text, time): what Kavach does about each |
| 9 | [vision/startup-needs.md](vision/startup-needs.md) | What tech startups need (research, 2026), and which products and wedges Kavach should lead with |
| 10 | [spec/](spec/README.md) | How the language works (source of truth) |
| 11 | [architecture/overview.md](architecture/overview.md) | How the compiler and runtime are built |
| 12 | [architecture/repo-structure.md](architecture/repo-structure.md) | Where every file goes, and the rules for adding folders |
| 13 | [architecture/hash-maps.md](architecture/hash-maps.md) | How maps and sets are stored and hashed, and the research behind it |
| 14 | [architecture/gpu.md](architecture/gpu.md) | What makes GPU programming hard, how others ease it, and how Kavach runs `gpu fn` through `wgpu` |
| 15 | [architecture/flat-lists.md](architecture/flat-lists.md) | How lists of plain records and of text keep their contents inline, like C++'s `vector<Point>` and `std::string` |
| 16 | [architecture/windows-setup.md](architecture/windows-setup.md) | How to set up a Windows machine to run the tests and benchmarks |
| 17 | [architecture/tasks-on-all-cores.md](architecture/tasks-on-all-cores.md) | The proposal for running tasks on all cores: what stays in order, and how to build it |
| 18 | [architecture/crashes-and-restarts.md](architecture/crashes-and-restarts.md) | What happens when a program crashes or is killed, the machine restarts or sleeps, or the clock or time zone changes: every case and its status |
| 19 | [decisions/](decisions/README.md) | Why big decisions were made |

## Folders

```
docs/
├── vision/          why Kavach exists and where it is going
├── spec/            the language specification, one chapter per file
├── architecture/    how the implementation is organized
└── decisions/       decision records (one file per decision)
```

A user guide (`docs/guide/`) will be added once the Phase 1 compiler runs. Until then, the [examples](../examples/README.md) are the best introduction.
