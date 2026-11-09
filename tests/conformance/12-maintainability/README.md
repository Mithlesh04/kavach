# 12. Maintainability at scale

Conformance tests for spec chapter [12. Maintainability](../../../docs/spec/12-maintainability.md). Format: [../README.md](../README.md). This folder's [kavach.toml](kavach.toml) sets small limits, so the tests stay short.

| Spec | Test | Rule | Expects |
|---|---|---|---|
| 12.1 | [files-over-the-limit-fail.kav](files-over-the-limit-fail.kav) | A file longer than `file_lines` is a compile error. | compile error |
| 12.1 | [every-line-counts.kav](every-line-counts.kav) | Every line counts toward the file limit, including comments and blank lines. | compile error |
| 12.1 | [long-functions-only-warn.kav](long-functions-only-warn.kav) | A function longer than `fn_lines` is only a warning; the program still runs. | runs |
| 12.1 | [limit-exceptions-live-in-kavach-toml.kav](limit-exceptions-live-in-kavach-toml.kav) | A file can get its own limit in `[limits.files]` of kavach.toml. | runs |
| 12.6 | [deprecated-calls-warn.kav](deprecated-calls-warn.kav) | A call to a deprecated function gives a warning that shows its message; the function still works. | warning |
| 12.6 | [deprecated-fix-names-must-exist.kav](deprecated-fix-names-must-exist.kav) | The names in a `fix` expression must exist. | compile error |
| 12.6 | [deprecated-goes-before-a-function.kav](deprecated-goes-before-a-function.kav) | `deprecated` only goes before a function. | compile error |
