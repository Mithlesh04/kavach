# tools/ai (`kavach-ai`)

Tools that let AI assistants **ask the compiler instead of reading files** ([spec chapter 13](../../docs/spec/13-ai-tooling.md), [decision 0015](../../docs/decisions/0015-token-efficient-ai-tooling.md)). The goal: 90% fewer tokens for the same task.

```
kavach outline shop.kav                     # declarations without function bodies
kavach query callers total_paid shop.kav    # every place a name is used
kavach query uses Payments shop.kav         # functions that can touch a capability
kavach query fails ShopError shop.kav       # functions that can fail with an error type
kavach query type Order shop.kav            # one type's fields, variants and methods
kavach query find refund shop.kav           # declarations whose name contains it
kavach check --format json shop.kav         # one compact JSON line per message
kavach context pay shop.kav --budget 2000   # pay's code plus the outlines of what it uses
kavach context orders.refund                # in a project: no file, answers from .kavach/index
kavach mcp                                  # all of the above, for AI assistants over MCP
```

## Connect an AI assistant (MCP)

Install the CLI (`cargo install --path tools/cli`), then register the server with your assistant. For Claude Code:

```
claude mcp add kavach -- kavach mcp
```

The server offers six tools: `kavach_outline`, `kavach_query`, `kavach_context`, `kavach_check`, `kavach_run` and `kavach_test`. It runs locally and reads files only when a tool is called.

## llms.txt

[llms.txt](../../llms.txt) at the repository root is the whole language in about 1,350 tokens, against about 19,300 for the full spec (93% fewer). A test keeps it under 4,000 tokens and checks that every code block in it parses.

`outline` keeps doc comments, signatures, contracts and examples, which is everything a caller needs, and replaces bodies with `{ … }`. On `examples/03-errors` it is 23% of the file. It prints its size to stderr, so the saving is always visible.

**The index (spec 13.9):** `query` and `context` answer from an index of every declaration and reference. A project keeps it in `.kavach/index` and checks the program again only when a file or `kavach.toml` changed (compared by hash), so in a project they need no file. A single file outside a project is checked on every call.

**Next (spec 13.11):** refreshing the index file by file as files change, fixes inside messages, then `kavach edit` and the token benchmark.

| File | Job |
|---|---|
| `src/outline.rs` | `outline`, `outline_item` |
| `src/query.rs` | `query`: callers, uses, fails, type, find |
| `src/json.rs` | Messages as JSON lines |
| `src/context.rs` | `context`: a declaration plus what it uses, within a token budget |
| `src/index.rs` | `Index`: the declarations and references of a checked program |
| `src/store.rs` | `index_for`: a project's index in `.kavach/index`, rebuilt when a file changed |
| `src/mcp.rs` | `serve`: the MCP server (JSON-RPC over stdin/stdout) |
| `tests/tools.rs` | Exact expected answers for each tool |
| `tests/index.rs` | The index is saved, used, rebuilt; names with their module |
| `tests/mcp.rs` | The MCP protocol and every tool through it |
| `tests/llms.rs` | `llms.txt` stays small, and its code parses |
