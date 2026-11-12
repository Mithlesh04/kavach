# 10.9 Running code quickly

Kavach compiles to native machine code, but trying code should feel as quick as a scripting language.

| You want to run | Do this |
|---|---|
| One file | `kavach run hello.kav`: compiles it in memory with the fast backend and runs it. No project needed (1.3). |
| One file, again on every save | `kavach run hello.kav --watch` |
| One file, as a program to ship | `kavach build hello.kav`: writes the executable `hello` (`hello.exe` on Windows) in the current folder. `-o path` names it. |
| One function | `kavach run orders.kav --fn total_paid`: runs a function that takes no arguments |
| A few lines, interactively | `kavach repl`: type code, see each result at once. Earlier lines stay defined. |
| One expression | `kavach eval "[1, 2, 3].sum(n => n)"` prints `6` |
| A function's examples | `kavach test orders.kav --fn total_paid` runs its `example` lines and generated tests (chapter 11) |

In the editor (Phase 2, with the language server):

- A **▶ Run** button appears above `fn main`, every `test` and every `example` line.
- **Run selection** runs the selected lines in a REPL and shows the result next to the code.
- Output of `print` appears in the editor's output panel, not a separate terminal.

The REPL and `--watch` use the same compiler as `kavach build`, so code behaves exactly the same when it ships.
