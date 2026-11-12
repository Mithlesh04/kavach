# 10.14 Workspaces

A **workspace** is several packages in one repository, built and tested together: a framework and its plugins, a service and the libraries it shares with its tools. The `kavach.toml` at the repository's root lists them ([decision 0091](../../decisions/0091-workspaces.md)):

```toml
[workspace]
members = ["packages/*", "tools/cli"]
```

```
rath/
  kavach.toml          # [workspace]: no [package] needed
  kavach.lock          # one for the whole workspace
  kavach_modules/      # one for the whole workspace
  packages/
    router/            # a member: its own kavach.toml and src/
    sessions/
  tools/cli/
```

- **Members.** Each entry is a folder relative to the root, and `*` stands for any one folder name: `packages/*` is every folder in `packages/` that has a `kavach.toml`. The root may be a package too, with a `[package]` of its own.
- **One lock, one `kavach_modules/`.** Every member's git dependencies are fetched once, into the root's `kavach_modules/`, and recorded in the root's `kavach.lock`, so all members use the same commit of each. A member's own `kavach.lock` or `kavach_modules/` is never written.
- **Members use each other** by `path`, as any package does (10.12): `router = { path = "../router" }`.
- **Commands.** `kavach check`, `kavach test` and `kavach build` at the root, with no file, run on every member in the order `members` lists them, and say which member each line is about; the command fails when any member fails. In a member's folder they run on that member alone.
- A member's settings (`[build]`, `[test]`, `[env]`) are its own; the root has only `[workspace]` and, when it is a package, its own.

```
$ kavach test
== packages/router
... 12 passed
== packages/sessions
... 7 passed
```
