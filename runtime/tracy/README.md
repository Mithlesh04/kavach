# runtime/tracy/

Spans and the GPU's timed work shown live in the Tracy profiler (spec 14.12), through the `tracy-client` crate (decision 0123). Linked only into `kavach run --tracy` and programs built with `kavach build --tracy`, whose `main` calls `kv_tracy_start` (`runtime/native/src/rt_tracy.rs`); other programs hold none of it (decision 0024).

| File | Job |
|---|---|
| `Cargo.toml` | The crate `kavach-runtime-tracy`: `tracy-client`, started by hand, listening on this machine only. `.cargo/config.toml` builds Tracy's client without call stacks |
| `src/lib.rs` | `start`, which gives `kavach_runtime_core::profiler` its hooks: each span a zone on its thread, ended from the top as zones nest, and the GPU's work in a Tracy GPU context |
