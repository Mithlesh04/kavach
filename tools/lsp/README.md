# tools/lsp (`kavach-lsp`)

The Kavach language server, behind `kavach lsp` (spec [10.5](../../docs/spec/10-modules-and-packages/05-command-line.md)). It speaks the [Language Server Protocol](https://microsoft.github.io/language-server-protocol/) over stdin and stdout, so any editor that has an LSP client can use it.

It uses the same compiler driver as `kavach check`, so the messages are the same everywhere. It needs no outside crate besides `serde_json`: the protocol's framing is a header and a JSON body.

**What works:** errors and warnings as you type. When a file is opened or changed, the server checks it and sends its diagnostics, once no other message is waiting: edits that come faster than a check are checked once, for the latest text (messages are read on a thread of their own). The check runs on a thread of its own too, so hover, completion and colors are answered while a file is checked; a check whose text changed before it finished is not published, and the file is checked again. In a project, it checks the whole program (`src/main.kav` first, spec 10.1–10.3), using the editor's text of every open file, so an unsaved change in one file shows its effect in the others. An error in `kavach.toml` shows on the first line of the file. **Hover** shows the type of the expression under the cursor (`total: Int`), and **go to definition** jumps to where a name is declared, in any file of the project. Both come from the same compile as the diagnostics, kept until a document changes. **Semantic tokens** mark where a name comes from: a module brought in with `use` and the names used through it, and built-in methods apart from the program's own. A **quick fix** marks a plain `fn` that a `gpu fn` calls as a `gpu fn`, when it fits; others take out code that never runs (spec 4.7): a `match` arm that can't match, an `else if` that can't be true; a call to a deprecated function becomes its `fix` (spec 12.6); and the compiler's suggestions are made (spec 12.7: `=` made `==` in a condition, `..=` made `..`, `x % 2 == 1` made `x % 2 != 0`, a changed `const` made `let`, a `pub` function's `uses` added; a misspelled name changed to the closest, offered but not applied by `kavach fix`, as it may be wrong). `kavach fix` applies the same fixes to whole files and folders, leaving out any that would add an error.

**Next:** a VS Code client (item 2 of the [to-do list](../../docs/vision/todo.md)).

| File | Job |
|---|---|
| `src/lib.rs` | `serve`: the message loop, open documents, requests and notifications |
| `src/rpc.rs` | Reading and writing messages: `Content-Length` header, JSON body; where they come from (`Source`), and whether another is waiting |
| `src/checking.rs` | The checking thread: checks sent to it, what they found sent back among the editor's messages |
| `src/diagnostics.rs` | Checking a document (alone or with its project) and turning messages into diagnostics |
| `src/navigate.rs` | Hover (the type under the cursor) and go to definition |
| `src/complete.rs` | Completion: the names in scope, or after a `.` the fields and methods of the value |
| `src/deprecated.rs` | A call to a deprecated function rewritten with its `fix`: parameters replaced by the arguments |
| `src/suggest.rs` | The compiler's suggestions made: the change a message asks for |
| `src/fixes.rs` | Quick fixes: code that never runs taken out (spec 4.7), found in the compile's syntax tree, and a plain `fn` marked `gpu`; `kavach fix` |
| `src/tokens.rs` | Semantic tokens: modules and names brought in with `use` (`imported`), built-in methods (`defaultLibrary`) against the program's own |
| `src/position.rs` | Byte offsets to LSP positions (line, UTF-16 column) and back |
| `src/uri.rs` | `file://` URIs to paths and back |
| `src/tests.rs` | Whole conversations with the server, and tables for positions and URIs |

## Try it

```
cargo build -p kavach-cli
```

Then point your editor's LSP client at `target/debug/kavach lsp` for `.kav` files.
