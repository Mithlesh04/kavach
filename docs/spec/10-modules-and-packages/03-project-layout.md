# 10.3 Project layout

```
my-app/
├── kavach.toml          project settings and dependencies
├── kavach.lock          exact dependency versions (generated, commit it)
├── kavach_modules/      fetched dependencies (generated, never commit it; 10.12)
├── src/
│   ├── main.kav         entry point (for applications)
│   ├── orders.kav
│   ├── orders.test.kav  hand-written tests of `orders` (few; see 11.8)
│   └── billing/
│       ├── invoice.kav
│       └── invoice.test.kav
└── tests/               stories across many modules
    └── found/           failing inputs found by `kavach test` (generated, commit it)
```

A library has `src/lib.kav` instead of `src/main.kav`.

A file named `name.test.kav` is the tests of `name.kav` beside it, not a module of its own (11.8). So a glance at a folder shows which modules have hand-written tests.

Build output and caches go to `.kavach/`, which is never committed: `kavach build` writes the executable to `.kavach/bin/<name>` (`<name>.exe` on Windows), where `<name>` is the package name.
