# 0078. A browser playground through a WASI build of the checker

- **Status:** Accepted
- **Date:** 2026-11-09 (Mithlesh asked for a way outside developers can try Kavach)

## Context

Outside developers won't install a pre-alpha compiler, but almost everyone
clicks "Run" on a web page. The docs site is fully static (decision 0033),
so the playground must run in the reader's browser — there is no server to
check or run programs on.

Compiling the whole toolchain for the browser is not one step. The
interpreter runs each Kavach task on an operating-system thread
(`compiler/interp/src/tasks.rs`), and the browser has none without
cross-origin isolation headers GitHub Pages can't set. `runtime/core`
also pulls in the clipboard (`arboard`), serial ports (`serial2`) and
hidden input (`rpassword`), which don't build for WebAssembly, and
`runtime/net` needs `mio`, which doesn't either.

The checker alone, though, is the pitch: a visitor writes an agent that
calls a capability outside `can`, and the compiler refuses it — live, in
the page. The checker's crates (lexer → parser → resolver → checker →
driver) are pure Rust.

## Decision

**The playground is built in milestones. Milestone 1 ships the checker as
a WASI program the docs site runs in the browser.**

1. A new tool, `tools/playground/`, is a small binary: it reads a program
   on standard input, checks it with `kavach-driver` (the same pipeline
   every tool uses), and writes the messages as JSON on standard output,
   written with `runtime/core`'s own JSON writer. No new Rust dependency
   and no `unsafe`: the WebAssembly boundary is standard input and output,
   not hand-made memory passing, so the same architecture later carries
   the interpreter's `print` output to the page.
2. It is built for `wasm32-wasip1`. The docs site instantiates it with a
   small hand-written WASI shim (standard input from the editor's text,
   standard output into the result pane), one fresh instance per check.
3. `runtime/core` gets one cargo feature, `devices` (on by default):
   the clipboard, serial ports and hidden input. `kavach-driver` turns it
   off, so the checker's whole dependency tree builds for WebAssembly.
   Every other crate keeps the default, so nothing changes natively.
4. The engine is **not** committed: like the rest of the site's build
   output, `website/scripts/` builds it from source when the site builds.

### Later milestones

- **Milestone 2:** run programs whose tasks never `spawn`: the
  interpreter behind cargo features, with `runtime/net` stubbed and
  `ask<T>` always in its dry-run form.
- **Milestone 3 (maybe never):** `spawn` in the browser needs threads —
  cross-origin isolation via a service worker, or a cooperative
  scheduler. Decided only if milestone 2 shows real demand.

## Consequences

- A visitor tries Kavach's capability errors with nothing installed.
- Milestone 1 checks programs but doesn't run them; the page must say so
  honestly ("checking runs here; running needs the real compiler").
- Building the site needs the Rust toolchain with the `wasm32-wasip1`
  target — it already needs cargo, since every docs sample runs against
  the real `kavach`.
- `runtime/core` has its first cargo feature; a crate that needs the
  clipboard, serial ports or hidden input must keep the default features.
