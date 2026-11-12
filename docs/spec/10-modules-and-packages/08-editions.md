# 10.8 Editions

Kavach promises: **code that compiles today keeps compiling.**

To improve the language without breaking that promise, changes that would break old code are released in a new **edition**, chosen per package:

```toml
edition = "2026"
```

- Packages on different editions work together in one program.
- The editions are `2026`, the first; no edition is the newest. An edition Kavach doesn't have (`"2062"`) is an error that names the ones it has.
- `kavach fix --edition 2029` upgrades code automatically.
- Editions come out rarely (every few years) and only with a migration tool.
