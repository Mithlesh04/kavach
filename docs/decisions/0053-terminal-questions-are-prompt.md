# 0053. Terminal questions are `prompt`, not `ask`

- **Status:** Accepted
- **Date:** 2026-11-03

## Context

Mithlesh found `term.ask("Your name?")` confusing (to-do item 4). Two things make it so. `ask` is already the AI call, `ask<Verdict>("Is this fine?")` (spec 8.4), so one word meant two very different things: a question to a model, with costs and limits, and a question at the keyboard. And developers coming from other languages look for another word: JavaScript has `prompt()`, Python `input(prompt)`, and command-line libraries (Inquirer, Prompt Toolkit, `dialoguer`) say *prompt*.

The rest of `std.term` was reviewed too: colors and styles (`term.red`, `term.bold`), the progress bar (`term.progress`, `step`, `done`), and `term.confirm` and `term.choose`. Those names are what they do, so they stay. The module's name stays as well: it is the terminal's module, for more than questions. The other modules' functions were read for the same trap, a name another part of Kavach already uses or one that says less than it does; none was found. Two smaller points stay as they are: `http.text` beside `http.json_response` (`json` names the `std.json` module there), and the progress bar's `term.step(bar, n)` and `term.done(bar)`, which could become methods (`bar.step(n)`) later.

Mithlesh asked for the best call, for speed and simple, clear code (2026-11-03).

## Decision

**The four functions that ask a question at the terminal are named `prompt`:** `term.prompt`, `term.prompt_int`, `term.prompt_float` and `term.prompt_secret`. A call to an old name is a compile error that names the new one (`term.ask` is now `term.prompt`). Deprecation with automatic fixes (spec 12.6) isn't built yet, and Kavach has few programs to move.

## Consequences

- `ask` means the AI everywhere; `prompt` means the keyboard.
- An existing program that uses an old name stops compiling, with an error that says the one-word fix.
- The docs site's input samples move to the new names.

## Alternatives considered

- **`term.input`**, as Python's `input`: also clear, but `input` reads as a noun (the input), and a function that prints a question first is a prompt.
- **A module of its own (`std.prompt`)**: one more `use` line for a program that already colors its output with `std.term`.
- **Keeping `ask`**: one word with two meanings, one of which costs money.
