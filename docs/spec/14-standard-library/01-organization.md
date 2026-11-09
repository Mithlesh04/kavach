# 14.1 How it is organized

- The **prelude** (10.2) is always available: built-in types, their methods (14.2), `print`, `debug`, `panic`, `env`.
- Every other module is brought in with `use`, then used through its name:

  ```kavach
  use std.fs

  fn main() {
      const text = fs.read_text("notes.txt") catch ""
  }
  ```

- A function in a module that touches the outside world needs that capability (8.1). `fs.read_text` needs `Fs`, so a function calling it needs `uses Fs` (or infers it).
- Functions that can fail return errors as values (chapter 5). `std.fs` errors are an `FsError` union that code can `match` on (14.3); the other modules' errors print as a clear message (`invalid pattern …`).
- Most of `std` is written in Kavach, in `stdlib/` (it ships with the compiler). The parts that need the operating system (files, clocks, the terminal) or exact low-level work (regex engines) are provided by the runtime. A program can't tell the difference.

| Module | What it has | Capability |
|---|---|---|
| prelude | Methods of numbers, text, lists, maps and sets (14.2) | none |
| `std.fs` | Files and folders (14.3) | `Fs` |
| `std.time` | Clocks, waiting, dates and times of day (14.4) | `Clock` to wait; reading the time needs none |
| `std.math` | Math functions and constants (14.5) | none |
| `std.io` | Reading the terminal (14.6) | `Console` |
| `std.cli` | Command-line arguments and exit codes (14.6) | `Env` |
| `std.regex` | Regular expressions (14.8) | none |
| `std.json` | JSON text to and from `Json` values and typed values ([2.12](../02-types/12-json.md)) | none |
| `std.text` | Padding, words, capitalizing (14.9) | none |
| `std.collections` | Chunks, zip, unique items, counting (14.9) | none |
| `std.test` | `Sample` and `Gen`, for types that make their own generated values (14.7) | none |
| `std.log` | Log events the program defines (14.12) | none |
| `std.term` | Colors and styles for the terminal (14.13) | `Console` |
| `std.grpc` | gRPC clients and servers from Kavach types (14.14) | `Net` |
