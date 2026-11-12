# 10.4 kavach.toml

```toml
[package]
name = "my-app"
version = "0.1.0"
edition = "2026"
unsafe = false            # must be true to use `unsafe` blocks (chapter 6)

[dependencies]
geometry = { path = "../geometry" }
stripe = { git = "https://github.com/example/stripe-kavach", tag = "v1.2.0" }
```

Other sections are described where they are used: AI models (`[ai]`) and agents' folder (`[agents]`) in chapter 8, release builds (`[build.release]`) in chapter 7, tests (`[test]`) in chapter 11, and limits, workspaces and architecture (`[limits]`, `[workspace]`, `[architecture]`) in chapter 12. Every key, with its values and default, is listed in 10.11.
