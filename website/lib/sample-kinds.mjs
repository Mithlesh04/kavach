// How a sample's name says what is checked — one place, read by both
// scripts/check-samples.mjs (which runs samples) and lib/samples.ts
// (which shows them). A sample is a file `name.kav` or a project folder
// `name.project/` (kavach.toml + src/), with one optional kind before
// the ending: `name.<kind>.kav`, `name.<kind>.project`. A project with a
// `runs` file is a story across runs: one kavach command per line, in
// order, and its .expect reads like the terminal session. A file
// `name.stdin` next to a sample is what a person types: its standard
// input (see `inputOf`).

/** @typedef {"run" | "fails" | "panics" | "warns" | "dry" | "test" | "lib" | "native"} SampleKind */

/** What each kind means, for the checker and for writers. */
export const kinds = {
  run: "runs with `kavach run`; .expect is its output",
  fails: "must be REFUSED by the compiler; .expect is the first error line (warnings before it are left out)",
  panics: "must run, then panic; .expect is its output, then the `panic:` line",
  warns: "must run and print at least one warning; .expect is the `warning:` lines, then its output, then the `panic:` line if the run ends in one",
  dry: "runs with --dry-run: capability calls and ask<T> get simulated answers",
  test: "runs `kavach test --seed 7`; .expect is the test report",
  lib: "runs `kavach build --lib`; .expect is the generated C header",
  native: "runs with `kavach run --native` (needed for C calls)",
};

/**
 * What is typed into sample `file` (`name.kav` → `name.stdin`), or
 * undefined when nothing is: then its standard input is empty.
 * @param {string} file
 * @param {(p: string) => boolean} exists
 * @param {(p: string) => string} read
 */
export function inputOf(file, exists, read) {
  const input = file.replace(/\.kav$/, ".stdin");
  return input !== file && exists(input) ? read(input).replace(/\r\n/g, "\n") : undefined;
}

/**
 * The kind of a sample from its file or folder name.
 * @param {string} name
 * @returns {SampleKind}
 */
export function kindOf(name) {
  const m = name.match(/\.(fails|panics|warns|dry|test|lib|native)\.(kav|project)$/);
  return /** @type {SampleKind} */ (m ? m[1] : "run");
}

/**
 * The name a reader sees, and the name the checker runs it under (so
 * paths in output, like `stats.kav:14`, match the page):
 * `sub-stats.test.kav` → `stats.kav`.
 * @param {string} base
 */
export function shownName(base) {
  return base.replace(/^sub-/, "").replace(/\.(fails|panics|warns|dry|test|lib|native)\.kav$/, ".kav");
}
