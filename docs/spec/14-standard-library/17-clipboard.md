# 14.17 std.clipboard

The system clipboard, behind the capability `Clipboard` (8.1): what a person copied can be private (a password, a message), so reading it needs permission, and so does writing over it.

| Function | Gives |
|---|---|
| `clipboard.read_text() -> String ! ClipboardError uses Clipboard` | The clipboard's text; fails with `Empty` when it holds none |
| `clipboard.write_text(text) ! ClipboardError uses Clipboard` | Puts `text` on the clipboard, in place of what was there |

`ClipboardError` is `Empty`, or `Unavailable(reason)` on a machine with no clipboard (a server, a container, a terminal with no desktop).

```kavach
use std.clipboard

fn main() ! Error uses Clipboard, Console {
    try clipboard.write_text("order 7: refunded")
    print(try clipboard.read_text())       // order 7: refunded
}
```

- On Linux with X11, text a program writes stays on the clipboard only while the program runs, as X11 keeps it with the program that wrote it; a clipboard manager (most desktops have one) keeps it after.
- Under `kavach test` the clipboard is a fake in memory that starts empty (11.5), so a test never reads or changes the real one.
- The clipboard comes through the `arboard` crate ([decision 0066](../../decisions/0066-clipboard-through-arboard.md)). Images are an open question (14).
