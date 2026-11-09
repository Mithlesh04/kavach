# tools/playground/ — the docs site's playground engine

A small binary ([decision 0078](../../docs/decisions/0078-browser-playground.md)):
it reads a Kavach program on standard input, checks it with
`kavach-driver` (the same pipeline every tool uses), and writes the
messages as JSON on standard output — severity, byte range, text, and
the full rendering the CLI would print.

| File | What it is |
|---|---|
| [src/main.rs](src/main.rs) | The whole engine: read, check, answer in JSON |

The docs site builds it for the browser and runs one fresh instance per
check:

```
rustup target add wasm32-wasip1
cargo build -p kavach-playground --release --target wasm32-wasip1
```

Natively it works the same (`cargo test -p kavach-playground`), so its
tests need no browser. It checks programs; it does not run them —
running in the browser is a later milestone of decision 0078.
