# tools/

Everything a developer touches besides the language itself. Good tools matter as much as good syntax.

| Folder | Tool | Job |
|---|---|---|
| `cli/` | `kavach` (alias `kv`) | `new`, `run`, `build`, `test`, `fmt`, `add`, `audit` |
| [fmt/](fmt/README.md) | formatter (`kavach-fmt`) | One official code style, no options to argue about (spec 12.7) |
| [lsp/](lsp/README.md) | language server (`kavach-lsp`) | Errors as you type in any editor now; hover and go-to-definition next |
| `pkg/` | package manager | Dependencies, `kavach.lock`, registry client |
| [ai/](ai/README.md) | AI tooling | Token-saving tools (spec 13): `outline`, `query`, JSON messages now; MCP server and `llms.txt` next |
| [playground/](playground/README.md) | playground engine | Checks a program for the docs site's playground, in the browser as WASI ([decision 0078](../docs/decisions/0078-browser-playground.md)) |

**Status:** `cli/` works: `new`, `run`, `build`, `test`, `check` and `fmt` (see [cli/README.md](cli/README.md)). `fmt/` and `ai/` work; `lsp/` reports errors as you type (`kavach lsp`); `pkg/` comes in Phase 3.
