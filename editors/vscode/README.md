# Kavach for VS Code

Kavach in VS Code, Cursor and other VS Code-based editors. Syntax highlighting for:

- `.kav` files
- ```` ```kavach ```` code blocks inside Markdown files (the spec and docs are full of them)
- a shield icon for `.kav` files in the explorer and tabs (shown when your file icon theme has no icon of its own for Kavach, which is true for the default theme)
- comment toggling (`Ctrl+/`), bracket matching, auto-closing, auto-indent, and `///` continuing on Enter

And through the language server (`kavach lsp`, `tools/lsp`), started when a `.kav` file opens:

- errors and warnings as you type
- hover: the type of what is under the cursor
- go to definition (`F12`)
- completion: the names in scope, and after a `.` the fields and methods of the value

To install it, follow the steps in [Install](#install) below.

## Files

| File | Job |
|---|---|
| `package.json` | Extension manifest: registers the `kavach` language, the grammars and the `kavach.path` setting |
| `extension.js` | Starts `kavach lsp` and connects it to VS Code: open files, errors, hover, go to definition, completion |
| [`client/`](client/README.md) | The messages to and from the server (JSON-RPC as LSP frames it), in plain Node: no npm packages |
| `syntaxes/kavach.tmLanguage.json` | The grammar: which text is a keyword, string, type, comment… |
| `syntaxes/kavach-markdown.tmLanguage.json` | Highlights ```` ```kavach ```` blocks in Markdown |
| `language-configuration.json` | Comments, brackets, auto-closing, indentation |
| `icons/kavach.svg` | File icon for `.kav` files: a shield (*kavach* means armor) with a K |
| `tests/` | Grammar tests (each `^` marks a token and the scope it must get), and `client.test.js`, which drives the real server through `client/rpc.js` |

## Install

The extension isn't on the VS Code Marketplace yet: you build it from this repository. Run the commands from the repository's root folder.

**1. Build Kavach.** The extension runs the `kavach` program for errors, hover, go to definition and completion. You need [Rust](https://rustup.rs):

```
cargo build --release -p kavach-cli
```

This makes `target/release/kavach.exe` on Windows, `target/release/kavach` on Linux and macOS.

**2. Package the extension.** No npm is needed:

```
powershell -File scripts/package-vscode.ps1     # Windows
sh scripts/package-vscode.sh                    # Linux, macOS, Git Bash
```

This writes `.kavach/kavach-vscode-0.2.0.vsix`.

**3. Install it in VS Code:**

```
code --install-extension .kavach/kavach-vscode-0.2.0.vsix
```

Or in VS Code: open the Extensions view (`Ctrl+Shift+X`), click `…` at its top, choose **Install from VSIX…**, and pick the file.

**4. Tell the extension where Kavach is.** Skip this if `kavach` is on your `PATH` (typing `kavach` in a terminal runs it). Otherwise open the settings (`Ctrl+,`), search for **Kavach: Path**, and enter the full path of the program from step 1, for example:

```
D:\Projects\kavach\target\release\kavach.exe
```

In `settings.json` this is `"kavach.path": "D:\\Projects\\kavach\\target\\release\\kavach.exe"`.

**5. Reload VS Code:** `Ctrl+Shift+P`, then **Developer: Reload Window**.

**6. Check that it works.** Open a `.kav` file and write a mistake, such as `const x: Int = "text"`: a red line appears under it. Hover over a name to see its type, press `F12` on a name to go to where it is declared, and type a variable's name and `.` to see its fields and methods. If VS Code says it can't start `kavach lsp`, the path in step 4 is wrong.

**After you change Kavach or the extension,** repeat steps 1 and 2, install again with `code --install-extension .kavach/kavach-vscode-0.2.0.vsix --force`, and reload (step 5).

If you have npm, `cd editors/vscode` then `npm run package` makes the same `.vsix` with Microsoft's `vsce` tool.

To work on the grammar or the extension itself, open the `editors/vscode` folder in VS Code and press `F5`: a second window opens with the extension loaded from the folder.

## Test

```
cd editors/vscode
npm test
```

Run them before committing a grammar change. The client test needs only Node and a built `kavach` (`cargo build -p kavach-cli`):

```
node tests/client.test.js ../../target/debug/kavach
```

Run it after changing `extension.js`, `client/` or the language server. When you add a keyword or syntax to the spec, add it to the grammar **and** add a test line for it in `tests/`.

## Scopes

Colors come from your theme. The grammar only names each token, using the standard TextMate scope names so every theme works:

| Kavach | Scope |
|---|---|
| `fn`, `type`, `let`, `agent`, `capability`, `test` | `storage.type` |
| `const` | `storage.type.const` and `variable.other.constant`, so themes colour it apart from `let` |
| `pub`, `secret`, `extern`, `deprecated` | `storage.modifier` |
| `if`, `match`, `for`, `return`, `try`, `catch`, `fail`, `when` | `keyword.control` |
| A loop's label: `outer` in `outer: for` and `break outer` | `entity.name.label` |
| `require`, `ensure`, `invariant`, `example`, `fails`, `assert` | `keyword.control.contract` |
| `can`, `limit`, `approve`, `by`, `per`, `on`, `uses` | `keyword.other.agent` |
| `Int`, `String`, `Decimal`, `Json`, … | `support.type.primitive` |
| A unit after a number: `5.seconds`, `250.ms`, `2.minutes`, `1.hours`, `3.days` | `keyword.other.unit` (as CSS's `px`), so it doesn't look like a field |
| `Console`, `Net`, `Ai`, … (built-in capabilities) | `support.class.capability` |
| Other `PascalCase` names | `entity.name.type` |
| `print`, `debug`, `panic`, `ask`, `reveal`, `env` | `support.function.builtin` |
| `self`, `result` | `variable.language` |
| Other names: variables and parameters, also inside `{…}` in a string | `variable.other.readwrite` |
| A field after `.` (`user.name`) | `variable.other.property` |
| `{` and `}` around code in a string (`"{a + b.c}"`) | `punctuation.definition.template-expression` (as JavaScript's `${…}`) |
| The code inside them | `meta.embedded.line`, then its own scopes, so it is coloured like code, not like the string |
| `var` (not Kavach since decision 0034) | `invalid.illegal` |
| `// note` and `/* note */` | `comment.line.double-slash`, `comment.block` |
| `/// doc comment` | `comment.line.documentation` |

One color is the extension's own, because themes paint every comment alike: doc comments are teal (`#4FA8A8`, and `#1D7A7A` in light themes), next to your theme's plain comments. A `///` is more than a note: on a field of a type given to `ask`, the model reads it (spec 8.4). Change it with `editor.tokenColorCustomizations` in your settings.

## Where a name comes from

With the language server running (`kavach lsp`), two more colors show what the grammar alone can't tell:

| Name | Semantic token | Color |
|---|---|---|
| A module brought in with `use` (`math` in `math.sqrt`), what is used through it (`sqrt`), and names brought in one by one (`use std.net.{connect}`) | the name's kind, with the modifier `imported` | lavender (`#C3A6FF`, `#7A3E9D` in light themes) |
| A built-in method (`xs.push(2)`, `s.len()`) | `method` with `defaultLibrary` | gold (`#D7BA7D`, `#8A6D00`) |
| A method the program defines (`order.total()`) | `method` | your theme's method color |

So a call shows whether it is the language's or the program's. The colors are the extension's own (`editor.semanticTokenColorCustomizations`), for Kavach files only. The quick fix "Mark `helper` as a `gpu fn`" adds `gpu` to a plain `fn` that a `gpu fn` calls, when it fits the GPU's rules (spec 15.1). "Remove this arm" and "Remove this branch" take out a `match` arm that can't match and an `else if` that can't be true (spec 4.7).
