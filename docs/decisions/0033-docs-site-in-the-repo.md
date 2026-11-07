# 0033. The docs site lives in this repository

- **Status:** Accepted
- **Date:** 2026-11-02

## Context

kavach.io needs official documentation for beginners, experts,
researchers and decision makers. Its core promise mirrors the
language's: every claim links to the line of code and the measurement
behind it, every sample runs, and readers verify rather than trust.
That promise is only checkable if docs, code and benchmarks move
together.

## Decision

**The site lives in this repository, under a new top-level `website/`
folder** (this record is the folder's required decision), built with
Next.js and Tailwind as a fully static export, deployed to Vercel or
GitHub Pages. The Node toolchain stays scoped to `website/`; the
language itself keeps needing only Rust.

1. **One tree, sections → chapters.** A chapter page is tabs —
   Introduction, Code, Understand, Benchmark, Quiz — so each audience
   opens only its depth. The approved design (canvas, 2026-11-02) is
   the contract: fixed sidebar, sticky tabs, animated explainers, the
   same-speed race chart, themed scrollbars, hover notes on new syntax.
2. **Docs are versioned with the language.** Each release tag builds
   its own static tree (`/docs/…` latest, `/vX.Y/docs/…` frozen),
   listed in `versions.json`. Source references are shortcodes
   (`file:line#symbol`); the build resolves them against **that
   version's tag** and fails if the file or symbol moved. Deleting a
   version removes one entry and one folder; a stub redirects old
   links to latest with a notice.
3. **Nothing hand-made in code blocks.** Samples are extracted,
   compiled and run by the real `kavach` at build time, and shown
   output is compared exactly. Highlighting comes from
   `editors/vscode/syntaxes/kavach.tmLanguage.json` through Shiki —
   the editor and the site share one grammar.
4. **One place for every look.** All colors, spacing and the chart
   palettes (light and dark validated separately for colorblind
   safety) live in one token file Tailwind consumes; benchmark numbers
   live in one JSON generated from `benchmarks/` runs; the sidebar
   tree, quiz banks, hover notes and cheer/nudge pools are data files.
5. **Search is static.** Keyword search from a build-time index;
   semantic ranking from build-time embeddings with a lazily loaded
   in-browser query model. No backend anywhere.

## Consequences

- `website/` joins the repo map; `scripts/check-repo` skips its
  generated folders (`node_modules/`, `.next/`, `out/`) and lockfile,
  and the 500-line rule applies to the files we write there.
- CI for deploys stays manual-only, like everything else, until the maintainers
  want more.
- A docs change that lies about the language cannot ship: the sample
  runner and the reference checker fail the build first.

## Alternatives considered

- **A separate docs repository:** deploy independence, but file:line
  references rot silently, samples drift from the language, and
  benchmark numbers get retyped. Rejected — it breaks the site's core
  promise.
- **An off-the-shelf docs framework (Docusaurus, Starlight):** faster
  start, but the chapter-tab model, verified references, race charts
  and quiz engine are all custom anyway; the framework would mostly be
  in the way.
