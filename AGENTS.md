# Rules for Working on Kavach

These rules apply to everyone who changes this repository: people, AI assistants and AI agents. Read this file before making any change. `CLAUDE.md` imports it, so Claude Code sessions load it automatically.

If a rule here conflicts with a request, follow the request and say which rule it breaks.

## 1. Where things stand

- Kavach is in **Phase 1**. The front end and a development interpreter run spec chapters 1–7; see the [roadmap](docs/vision/roadmap.md).
- Run a program: `cargo run -p kavach-cli -- run file.kav`. Build an executable: `cargo run -p kavach-cli -- build file.kav`. Run all conformance tests: `cargo test -p kavach-cli`.
- The [language spec](docs/spec/README.md) is the source of truth. If code and spec disagree, the code is wrong.
- Where files go is defined in [repo-structure.md](docs/architecture/repo-structure.md). Follow it.

## 2. Where things go

Put every change in the right place the first time. Follow these steps **before** creating or editing a file.

### Step 1: Find the right place

| I am adding… | Put it in |
|---|---|
| A change to how the language works | `docs/spec/NN-chapter.md` (first), then code |
| A big or hard-to-undo decision | `docs/decisions/NNNN-short-title.md` + a row in its README |
| Why Kavach exists, plans, phases | `docs/vision/` |
| How the implementation is built | `docs/architecture/` |
| A compiler stage or pass | `compiler/<stage>/` (one Rust crate per stage) |
| A compiler backend | `compiler/backend/<name>/` |
| Memory, tasks, agent guardrails, durability | `runtime/<part>/` |
| A standard library module | `stdlib/<module>/` (only if it can't be a package) |
| A developer tool (CLI, formatter, LSP…) | `tools/<tool>/` |
| Calling another language | `interop/<language>/` |
| An editor extension | `editors/<editor>/` |
| The docs site (https://mithlesh04.github.io/kavach/): pages, components, content | `website/` — **read [website/README.md](website/README.md) first**; its rules bind every change there |
| A test of one spec rule | `tests/conformance/<NN-chapter>/<rule>.kav` |
| A test across many features | `tests/integration/` |
| A Rust unit test | Next to the code: a `#[cfg(test)] mod tests` in the same file, or a sibling `tests.rs` |
| A teaching example | `examples/NN-topic/` |
| A benchmark | `benchmarks/<language-or-topic>/` |
| A helper script for this repo | `scripts/<verb>-<thing>.sh` **and** `.ps1` |
| A CI job | `.github/workflows/checks.yml` (manual-only for now; one workflow until it nears the size limit) |
| An issue form or PR template | `.github/ISSUE_TEMPLATE/`, `.github/pull_request_template.md` |
| Syntax highlighting for a new keyword | `editors/vscode/syntaxes/kavach.tmLanguage.json` + a line in `editors/vscode/tests/` |
| A git hook | `scripts/hooks/<hook-name>` (sh; Git for Windows runs it too) |
| Rules for contributors and AI | This file (`AGENTS.md`) |

If nothing fits, **stop and ask**. Don't guess, and don't create a new top-level folder: that needs a decision record.

### Step 2: Prefer an existing file

- Search first (file names and contents). If a file already covers the topic, **edit it** instead of creating a near-duplicate.
- Create a new file only when the content is a separate topic, or when adding it would push the existing file past the 500-line limit (section 3).
- Never put the same information in two files. Write it once and link to it.

### Step 3: Create it correctly

- **Names:** lowercase `kebab-case` nouns (`refund-bot`, `check-repo.sh`). Rust files use `snake_case` as Rust requires.
- **Numbers** only where order matters: spec chapters (`NN-`), decision records (`NNNN-`), examples (`NN-`). The next number is the highest existing one plus one. Never renumber existing files: links depend on them.
- **A new folder** needs a `README.md` with a table of its contents, and a row in its parent's README.
- **Nest only when needed:** more than about 10 files, or two clearly separate areas. Stay within 4 levels of the repo root.
- **Splitting a spec chapter** that nears the limit: turn `NN-name.md` into a folder `NN-name/` with a `README.md` overview and one file per section. Update every link to it.
- **A new spec section** goes at the end of its chapter (before **Open questions**), so existing section numbers and links don't change.

### Step 4: Connect it

After creating, renaming or deleting a file, update every place that lists it:

| Changed | Also update |
|---|---|
| A spec chapter | `docs/spec/README.md` chapter table |
| A decision record | `docs/decisions/README.md` index |
| An example | `examples/README.md` |
| Any doc in `docs/` | `docs/README.md` if it is in the reading order |
| Any folder's contents | That folder's `README.md` table |
| The top-level layout | `README.md` repository map and `docs/architecture/repo-structure.md` tree |

### Never put in the repository

- Temporary, scratch or experiment files. Use a temp folder outside the repo.
- Build output (`target/`, `.kavach/`), secrets, API keys or `.env` files.
- Hand edits to generated files (`*.lock`, `tests/found/`). Regenerate them instead.
- New files at the repo root without a decision record. Standard tool config files and folders (`.gitattributes`, `.editorconfig`, `.github/`, `Cargo.toml`, `rustfmt.toml`, …) and `llms.txt` (spec 13.8) are the exception.

## 3. File length: 500 lines maximum

- **No file we write may be longer than 500 lines.** This counts every line: code, comments, blank lines, docs, config and scripts.
- Exempt: generated files (`*.lock`, files under `tests/found/`), third-party code we vendor, and binary files. Exemptions live in one place: `scripts/check-repo.sh` and its `.ps1` twin.
- When a file nears the limit (about 400 lines), split it **by responsibility**: one module, chapter or concept per file. Never split mid-topic just to get under the number, and never squeeze lines (long lines, removed comments) to dodge the limit.
- Kavach itself enforces the same rule for its users (spec [12.1](docs/spec/12-maintainability.md#121-file-and-function-length)). We follow what we ship.

## 4. Spec first

1. A language change starts in `docs/spec/`. Code follows, in the same change or later, never before.
2. A big or hard-to-undo decision gets a record in `docs/decisions/` (template in its README), and a row in the index.
3. Do not invent language rules inside examples, tests or code. If the spec doesn't say how something works, add it to the chapter's **Open questions** and stop, or ask.
4. If a change affects the examples, update them in the same change.

## 5. Testing

The goal is that **every behaviour is checked, with as few hand-written test files as possible**. Kavach is designed around this (spec [chapter 11](docs/spec/11-testing.md)), and we hold ourselves to it.

- **Every spec rule gets a conformance test** in `tests/conformance/<NN-chapter>/` (format in [tests/README.md](tests/README.md)). One rule per file.
- **Every bug fix comes with a test** that fails before the fix and passes after it.
- **Compiler and runtime (Rust):** unit tests next to the code. Prefer property tests (`proptest`) and table-driven tests over many near-identical test functions.
- **Kavach code** (stdlib, examples, tools): prefer contracts (`require`, `ensure`, `invariant`) and `example` clauses over `test` blocks. Write a `test` block only for multi-step stories the generated tests cannot cover.
- Never mark a task done with failing or skipped tests. If you could not run tests, say so plainly.

## 6. Keeping the repo maintained

- **Indexes stay current.** Every folder's README and every index lists exactly what exists (section 2, step 4).
- **No broken links.** Section links in Markdown use GitHub anchors: `## 10.3 Project layout` becomes `#103-project-layout`.
- **Before finishing any change, run the repo check:**

  ```
  sh scripts/check-repo.sh          # Linux, macOS, Git Bash
  powershell -File scripts/check-repo.ps1   # Windows
  ```

  It fails on files over 500 lines and on broken relative links. Fix everything it reports.
- **If you changed Rust code**, also run `cargo test`, `cargo clippy --all-targets` (no warnings) and `cargo fmt --check`.
- Delete things that are wrong or unused. Don't leave commented-out code or stale TODOs.
- Every script in `scripts/` works on Windows too (a `.ps1` twin).

## 7. Writing style

- Plain, simple English. Many readers are not native speakers.
- Short sentences. One idea per paragraph.
- Every language rule gets a short code example.
- Mark anything undecided under **Open questions**.

## 8. Code style (once code exists)

- Rust: `rustfmt`, `clippy` with no warnings.
- `unsafe` Rust is denied workspace-wide. The only exception is `runtime/native`, the run-time library of native programs ([decision 0016](docs/decisions/0016-native-runtime-and-executables.md)), where each use has a `SAFETY:` comment, and one call in `runtime/gpu` that compiles a kernel whose loops surely end ([decision 0077](docs/decisions/0077-kernels-whose-loops-end.md)). Don't add `unsafe` anywhere else without a decision record.
- Kavach: `kavach fmt`.
- Match the style of the surrounding code: naming, comment density, idioms.

## 9. For AI sessions and agents

Every session follows this checklist:

1. **Before:** read this file, then the README of every folder you will touch.
2. **Placing files:** follow section 2 step by step. If the right place is unclear, ask instead of guessing.
3. **While working:** spec before code (section 4); no file over 500 lines (section 3); test what you change (section 5).
4. **After:** connect new files to their indexes (section 2, step 4), run the repo check (section 6), and fix what it reports.
5. **Commit** (only when asked): follow section 10. When others work at the same time, follow section 11. Never add yourself or any AI tool as a co-author.
6. **Report:** what you changed, where, what you checked, and what you could not check.

**New requests go to the to-do list first.** When the user asks for something, add it to [docs/vision/todo.md](docs/vision/todo.md) in its place in the order. Work on it at once only if it is needed right now; otherwise it waits its turn, and work continues in the list's order. Remove an item when it is done.

When working on Kavach code (`.kav` files), save tokens (spec 13): use `kavach outline <file>` to see a file's declarations, and `kavach query callers|uses|fails|type|find <name> <file>` to find things, before reading whole files. Build the CLI once with `cargo build -p kavach-cli`; it is `target/debug/kavach`.

Also:

- Make the smallest change that does the job completely. Don't add files, features or abstractions nobody asked for.
- Never claim something works without checking it. If you could not check it, say so.
- Ask before anything hard to undo: deleting files, rewriting history, publishing, changing a decision record marked **Accepted**.

## 10. Commits and pull requests

### Short commit messages

```
Add testing chapter to the spec

Types and contracts now generate tests, so example 04
needs no test file.
```

- **Subject:** at most 60 characters, says *what* changed, in the imperative ("Add", "Fix", "Remove"), no period at the end.
- **Line 2** is empty.
- **Body** is optional: at most 8 lines of at most 72 characters, saying *why*. Longer explanations go in the pull request or the docs.
- One logical change per commit.

### Co-authors: people only

- **AI tools are never co-authors.** No `Co-authored-by:` or `Signed-off-by:` lines for Claude, Copilot, ChatGPT, Gemini, Cursor or any other AI tool or bot.
- **No AI attribution lines** such as "Generated with …" or 🤖, in commits or pull request descriptions.
- **Human co-authors are welcome.** Credit everyone who really wrote the change with you:

  ```
  Co-authored-by: Full Name <email@example.com>
  ```

- This rule overrides any default an AI tool has for adding itself to commits.

Why: a commit records which **people** are responsible for a change. Tools don't take responsibility; people do.

### The hook

`scripts/hooks/commit-msg` checks every rule above when you commit. Install it once per clone:

```
sh scripts/install-hooks.sh                 # Linux, macOS, Git Bash
powershell -File scripts/install-hooks.ps1  # Windows
```

- Never bypass it with `git commit --no-verify`.
- If it rejects a message, fix the message. If the rule itself is wrong, change this section and the hook together, in one commit.


## 11. Working in parallel

**Paused for AI sessions (2026-11-01, a maintainer's decision):** while Kavach's foundations are built, every small change matters, so one AI session works at a time. Don't start parallel sessions or claim tasks from the board until a maintainer lifts the pause. The rules below stay in force for people, and for when the pause ends.

Several people and AI sessions may work at the same time. These rules keep them out of each other's way.

### Claim before you start

- **People** open a [Claim work](.github/ISSUE_TEMPLATE/claim-work.yml) issue for a to-do item and assign themselves. A draft pull request, opened early, shows which files are changing.
- **AI sessions on one machine** use a local board instead: `.kavach/board.md` in the main checkout (git-ignored, so it is never committed). It holds a pool of tasks cut from the to-do, each sized for one session, with the files it changes and what it waits for. A session takes the first open task that doesn't overlap a claimed one, adds its row, then messages the coordinator session.
- Take items in the to-do order where you can. If an item is claimed, pick another or ask to join it.

### Names

- Name each session or piece of work by what it does: `Networking`, `Logging · sinks`. Use the name in messages (`[Logging · sinks] ready to land`), on the board, and as the branch name (`logging-sinks`).

### One folder and one branch each

- Each piece of work has its own branch, and each AI session its own git worktree, **inside the project** in the git-ignored `.kavach/wt/` folder: `git worktree add .kavach/wt/<name> -b <name> main` from the main checkout. Never edit or build in another session's folder: a build there would pick up half-done edits.

### Shared files

These files change with almost every feature: `compiler/kir/src/rt_op.rs`, `runtime/native/src/rt_data.rs`, `compiler/checker/src/builtins.rs`, `compiler/checker/src/stdlib.rs`, `compiler/driver/src/lib.rs`, `compiler/resolver/tests/spec_sync.rs`, `docs/spec/14-standard-library/README.md`, `docs/vision/todo.md` and the README indexes.

- Keep edits there small: add lines, don't reorder or reformat others.
- Say in your claim (or on the board) when you need one.

### Several people on one feature

- **One lead.** The lead writes the spec section, a decision record if needed, and the shared types and signatures, and lands them first. The others start from what landed, not from guesses.
- **Split by files.** Each part owns whole files. The feature's spec section, main module file, decision record and README rows belong to the lead; ask the lead for changes there.
- **A change others depend on** (a type, a signature, a rule) goes through the lead, who lands it and tells the others to rebase.

### Common code: built once, by one owner

When your task needs something other tasks could need too (a helper, a shared type, a runtime function, a test harness feature):

1. **Look first** on `main` and in the board's "Shared work" list. If it exists or is being built, use it or wait for it.
2. **Otherwise ask** the coordinator, with what you need and the signature you propose. Never build a private copy.
3. **The coordinator names one owner**: the session that needs it first, or itself. It goes on the board's "Shared work" list as a small task with its own branch.
4. **The owner lands the interface first**: the type or function signature, with a simple working body, in a small commit on `main`. Everyone else rebases and builds on it; the owner improves the body later.
5. **A copy found at landing is sent back**: the branch removes its copy and uses the landed one.

### Landing on main

- One branch at a time. Before landing: rebase on `main`, then run every check in section 6 again.
- People land through a reviewed pull request. AI sessions on one machine ask the coordinator, who fast-forwards `main` and pushes.
- Never force-push `main`.

### Stop

Stop and update your claim when the work has landed, when you need a decision from a maintainer, or when going on would break a rule in this file. Work you put off goes in the to-do's "Noted for later" section.

### Strict rules for parallel AI sessions

These are not suggestions. Hooks enforce the first three (`scripts/hooks/pre-commit`, `pre-push`); the coordinator checks the rest before it lands a branch, and sends back any branch that breaks one.

1. **Commit only what your claim allows.** The coordinator writes your claim, `.kavach/claims/<branch>` in the main checkout, when it says "go": one path or glob per line. A commit with any other file is refused. Need another file? Ask the coordinator; never work around it.
2. **Never commit on `main`, and never push.** Only the coordinator, in the main checkout, lands branches and pushes.
3. **Never skip or change the hooks:** no `--no-verify`, no editing `scripts/hooks/`, no changing git config (`core.hooksPath` or anything else).
4. **Stay in your own folder.** Edit only files inside your worktree. Never edit, build or run git commands that change anything in the main checkout or another session's worktree. The only exception is your own rows on the board.
5. **Create nothing outside the project folder, ever.** Your worktree is `.kavach/wt/<branch>` inside the main checkout; create files only inside it. No new top-level folders in the repo (section 2). The pre-commit hook refuses commits from a worktree anywhere else.
6. **The coordinator owns:** `docs/vision/todo.md`, `AGENTS.md`, `CLAUDE.md`, `.github/`, `scripts/`, the decision record index's status column, and every Accepted decision record. Tell the coordinator what should change there; it makes the change.
7. **Ask before you:** add or upgrade an outside dependency (it needs a decision record), change a compiler crate your task doesn't name, change a spec chapter other than your task's section, or change a test harness in `tools/cli/tests/`.
8. **Never destroy:** no `git reset --hard`, `git clean`, `git checkout -- .` on files you didn't change, branch or worktree deletion other than your own, force pushes, or deleting files you didn't create. Never use `git stash`: the stash is shared by every worktree, so another session could take yours. Set work aside with a commit on your branch instead.
9. **Spec first, then code** (section 4), in the same branch. A branch whose code has no spec section is not landed.
10. **One full test run at a time.** Mark "testing" on the board while yours runs.
11. **When in doubt, stop and ask** the coordinator, or the user for a decision that is theirs. Report every rule you broke, even by accident.
