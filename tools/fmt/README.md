# tools/fmt (`kavach-fmt`)

The formatter behind `kavach fmt` (spec [12.7](../../docs/spec/12-maintainability.md#127-one-way-to-write-it)): one style, no options. It fixes names in the wrong style (spec 1.5): a declaration such as `orderCount` becomes `order_count` together with every use, unless it is `pub` (other files may use it) or the new name is already taken. Then it changes only the layout of lines (indentation, trailing spaces, blank lines, line endings), never what is on them, so comments stay where they are. After the layout pass, the file's tokens must be the same as before; otherwise nothing is written.

| File | Job |
|---|---|
| `src/lib.rs` | `format`: indentation from open delimiters, continued lines, comments, blank lines |
| `src/names.rs` | `fix_names`: declarations in the wrong style (spec 1.5) renamed with every use; `pub` names left to a person |
| `src/tests.rs` | Table tests, a second run changes nothing, and every `.kav` file in the repository is formatted |
