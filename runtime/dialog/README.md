# runtime/dialog/

The system's dialogs for `std.app` (spec 14.30), through `rfd` (decision 0118): open, save, folder, message and confirm. Apart from `native/`, so a program that shows no dialog doesn't link `rfd` (decision 0024).

| File | Job |
|---|---|
| `Cargo.toml` | The crate `kavach-runtime-dialog` and its one dependency, `rfd`, on Linux through the desktop's portal |
| `src/lib.rs` | `show(which, title, text)`: each dialog, and what it gives back as `std.app` reads it; a test that only Yes confirms |

The dialogs need a person at the screen, so the tests don't show them.
