# 0084. Environment variables as secrets, `.env` files, and capabilities granted per dependency

- **Status:** Accepted
- **Date:** 2026-11-12

## Context

A maintainer asked (2026-11-12) for packages from outside the repository and environment variables to be "handled carefully", as npm and pnpm handle packages. Three gaps stood out:

- A dependency could use every capability: a package fetched to format dates could read files and send them over the network. npm has the same hole, and it is how most supply-chain attacks work.
- `env("NAME")` gave plain text, though API keys and passwords usually come from the environment, so nothing stopped one from being printed or logged. Native code had no `env` at all.
- There was no way to keep a project's settings in a file that is not committed (a `.env` file), as Node, Deno, Bun, Docker and most frameworks allow.

The maintainer chose each answer below from the options put to them.

## Decision

- **Capabilities are granted per dependency** (spec 10.12): `money = { git = "…", can = ["Net"] }`. A dependency may use only what its entry grants; one without `can` is pure. The checker enforces it, since a `pub` function already lists what it uses (8.1). A dependency's own dependencies get at most what it was granted. `kavach add` lists what the package uses and writes the grant, so the person adding it sees it.
- **`env(name)` gives a secret** (spec 8.7, 10.13): it can't be printed, logged or put in a prompt unless revealed with `reveal(…)`. Native code gets `env` too.
- **`.env` files, in development and in builds alike** (spec 10.13). The maintainer asked that a built program behave as `kavach run` does, so there is one rule to learn:
  - `kavach run` and `kavach test` read the project's `.env`. `kavach build` copies it beside the executable, and the executable reads the `.env` beside it the first time it reads a variable (so hello world doesn't grow: 48 KB on Linux when it was read at start).
  - The values are never compiled into the executable: an executable is shared and copied, and secrets inside it would go wherever it goes.
  - Another file is chosen with `--env-file <path>` or `[env] file` in `kavach.toml`; a built program takes the variable `KAVACH_ENV_FILE`, so `docker run -e KAVACH_ENV_FILE=…` works.
  - Single variables, `--env NAME=value` and `[env.vars]` in `kavach.toml`, override the file.
  - Which value wins, strongest first: `--env` on `kavach run` or `kavach test`, the real environment (the shell, `docker run -e`), `[env.vars]`, the file. For a build, `--env` and `[env.vars]` are written into the copied file, so the place the program runs has the last word.

## Consequences

- A package can't quietly reach the network, the disk or the environment: what it may do is written in the project's `kavach.toml` and seen in review.
- Keys from the environment are tracked like any other secret. Values that aren't secret (a port, a log level) need `reveal(…)`, which is more to write but visible in review.
- `kavach new` puts `.env` in `.gitignore`. A copied `.env` lives in `.kavach/bin/`, which is never committed.

## Alternatives considered

- **Only listing what each dependency uses** (`kavach audit`): it informs but stops nothing.
- **`env` plain, with `env.secret` for secrets:** forgetting it is the mistake this decision prevents.
- **`.env` only with a flag** (Node, Deno): the maintainer preferred one behaviour for `run`, `test` and built programs.
- **Compiling `.env` into the executable:** the secrets would travel with every copy of the file.
