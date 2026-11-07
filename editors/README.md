# editors/

Editor extensions. Highlighting is in the extension; errors, hover, go to definition and completion come from the language server, `kavach lsp` ([tools/lsp](../tools/README.md)), which the extension starts.

To install the VS Code extension, follow [its steps](vscode/README.md#install).

| Folder | Editor | Phase |
|---|---|---|
| [vscode/](vscode/README.md) | VS Code, Cursor and other VS Code-based editors | Highlighting, errors, hover, go to definition, completion: now |

Add new editors as sibling folders (`editors/jetbrains/`, `editors/neovim/`, …).
