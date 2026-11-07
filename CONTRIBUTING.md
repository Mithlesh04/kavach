# Contributing to Kavach

Kavach is in **Phase 1** (see the [roadmap](docs/vision/roadmap.md)). Each language rule is written in [docs/spec](docs/spec/README.md) first, and the code follows it.

**Read [AGENTS.md](AGENTS.md) first.** It has the rules every change must follow (people and AI alike): 500-line file limit, spec first, testing, and the repo check to run before finishing.

On Windows, [windows-setup.md](docs/architecture/windows-setup.md) lists what to install to run the tests and benchmarks.

After cloning, install the commit hook once: `sh scripts/install-hooks.sh` (or `powershell -File scripts/install-hooks.ps1`). It keeps commit messages short and makes sure only people are listed as co-authors.

## How changes happen

1. **Spec first.** Any change to the language starts as a change to `docs/spec/`. Code follows the spec, never the other way around.
2. **Big decisions get a decision record.** If a change affects many users or is hard to undo (syntax, memory model, compatibility), add a record in [docs/decisions](docs/decisions/README.md).
3. **Every language feature gets tests.** Add conformance tests in [tests/conformance](tests/README.md) under the matching spec chapter.
4. **No file over 500 lines.** Run `sh scripts/check-repo.sh` (or `scripts/check-repo.ps1` on Windows) before sending a change.
5. **Examples stay working.** If a change affects the examples, update them in the same change.

## What helps most right now

- Reading the spec and pointing out anything confusing, inconsistent or missing.
- Real use cases: "I would use Kavach for X, but Y blocks me."
- Writing a small real program in Kavach and reporting what felt hard.

## Writing style for docs

- Simple, plain English. Many readers are not native English speakers.
- Short sentences. One idea per paragraph.
- Every rule gets a short code example.
- Mark undecided things clearly under **Open questions**.

## Code style

- Compiler and runtime: Rust, formatted with `rustfmt`, no warnings.
- Kavach code: formatted with `kavach fmt`.

## License of contributions

When you send a contribution to Kavach, you agree that it can be used under both licenses, Apache-2.0 and MIT, with no extra terms. If you don't want that, say so when you send it. "Contribution" means what it means in the Apache-2.0 license.
