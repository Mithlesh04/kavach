# scripts/

Helper scripts for developing Kavach itself: building everything, running all tests, release packaging.

| Script | Does |
|---|---|
| `run-benchmarks.sh` / `run-benchmarks.ps1` | Builds and times [benchmarks/loops](../benchmarks/loops/README.md) in Kavach and C++ (`g++ -O2`). Linux or WSL. |
| `check-repo.sh` / `check-repo.ps1` | Fails on files over 500 lines and broken relative Markdown links, in the files git keeps (what `.gitignore` leaves out is skipped). Run before finishing any change. |
| `package-vscode.sh` / `package-vscode.ps1` | Packages the VS Code extension as `.kavach/kavach-vscode-<version>.vsix`, without npm ([editors/vscode](../editors/vscode/README.md#install)). |
| `package-kavach.sh` / `package-kavach.ps1` | Packages an installable Kavach for this machine as `.kavach/dist/kavach-<version>-<system>-<arch>.tar.gz` (`.zip` on Windows) with a `.sha256`: `bin/` (`kavach`, `kv`), `lib/` (the runtime library), `llvm/` (`opt`, `llc`) ([decision 0080](../docs/decisions/0080-installing-kavach.md)). Needs rustup's `llvm-tools`. |
| `install-kavach.sh` / `install-kavach.ps1` | Installs Kavach for users: downloads the newest release archive (or `--from <file>` / `-From <file>`), unpacks it to `~/.kavach` (`%LOCALAPPDATA%\Kavach` on Windows), puts `kavach` on the path, and says what to install when the linker is missing. |
| `install-hooks.sh` / `install-hooks.ps1` | Makes git use the hooks in `hooks/`. Run once per clone. |
| `hooks/commit-msg` | Rejects long commit messages and AI co-authors ([AGENTS.md, section 10](../AGENTS.md#10-commits-and-pull-requests)). |
| `hooks/pre-commit` | In a parallel session's worktree: refuses commits on `main` and files outside the session's claim ([AGENTS.md, section 11](../AGENTS.md#11-working-in-parallel)). |
| `hooks/pre-push` | Refuses pushes from a worktree: only the main checkout pushes ([AGENTS.md, section 11](../AGENTS.md#11-working-in-parallel)). |

Keep each script small and name it after what it does (`build-all.sh`, `release.sh`). Every script must also work on Windows (provide a `.ps1` twin, or write it in Kavach once the CLI works). Git hooks in `hooks/` are the exception: they are sh scripts, and Git for Windows runs them with its own sh.
