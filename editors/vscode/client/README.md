# editors/vscode/client

The extension's side of the Language Server Protocol, in plain Node with no npm packages, so the extension installs as it is.

| File | Job |
|---|---|
| `rpc.js` | Starts `kavach lsp` and exchanges JSON-RPC messages with it over standard input and output, framed with `Content-Length` headers |
