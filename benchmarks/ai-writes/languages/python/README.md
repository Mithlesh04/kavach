# benchmarks/ai-writes/languages/python/

Python's side of the benchmark: the same 75 tasks, in a language every model has seen a great deal of. Its numbers are a reference for Kavach's: how well a model does when the language is no obstacle.

| File or folder | What it holds |
|---|---|
| `language.py` | Asks for a solution (a ```python block, with no main code), puts it together with the given code and a print of each hidden test, runs it, and sorts a failure by its exception |
| `tasks/<id>.json` | Each task's given code, signature, note, and hidden tests: a Python call and the line it prints |
| `solutions/<id>.py` | Each task's reference solution, with its given code |

Check every reference solution against its tests, with no model and no cost:

```
python run.py --check --language python
```

## Signatures and given code

Each task asks for a Python function with type hints (`def largest(xs: list[int]) -> int | None`), so it needs Python 3.10 or later. Kavach's types become Python's:

| Kavach | Python |
|---|---|
| `Int`, `Float`, `String`, `Bool` | `int`, `float`, `str`, `bool` |
| `Char` | `str` of one character |
| `[Int]`, `[String: Int]`, `(Int, Int)` | `list[int]`, `dict[str, int]`, `tuple[int, int]` |
| `Int?` (optional) | `int \| None` |
| A record, `type Rect { … }` | A `@dataclass` |
| A union, `type Shape = Circle(…) \| Square(…)` | One `@dataclass` for each case, and `Shape = Circle \| Square` |
| An error type, `-> Int ! BadInput` | An exception class, with a subclass for each case, which the function raises |
| An interface | An abstract class (`abc.ABC`) |
| `fn last_or<T>(…)` | A `TypeVar` named `T`, given |
| `inout xs: [Int]` | A list changed in place |
| `Decimal` | `decimal.Decimal` |

The descriptions are shared with Kavach, so some use Kavach's words (`require`, `with`, `inout`, `try`, `json.decode<T>`, a type written as Kavach writes it). For those tasks, `tasks/<id>.json` has a `note`, sent after the given code, that says what they mean in Python: "check the requirements with `assert`", "use `dataclasses.replace`", and so on. `checked-add`'s note says the numbers are 64-bit, since Python's integers never overflow.

## The hidden tests

The runner prints each test's value through `_kv_show`, a helper added after the solution, which writes a value the way Kavach prints it. So every expected line is the same as Kavach's, word for word:

| Value | Printed |
|---|---|
| `True`, `False`, `None` | `true`, `false`, `none` |
| `"hi"` alone | `hi` |
| `["a", "b"]`, `(0, 1)` | `["a", "b"]`, `(0, 1)` |
| `{"a": 1}`, `{}` | `["a": 1]`, `[:]` |
| `2.0`, `Decimal("5.30")` | `2.0`, `5.30` |

Two more helpers stand in for Kavach's calls:

- `_kv_catch(lambda: doubled("x"), BadInput, -1)` is Kavach's `doubled("x") catch _ => -1`: it gives `-1` when the call raises that error type. Any other exception still stops the program.
- `_kv_changed(swap_ends, [1, 2, 3])` calls `swap_ends` on the list and gives the list, as Kavach's `inout` test does.

A test whose function returns a `float` wraps the call in `float(…)`, as Kavach's type would: Python's `sum([])` is the `int` `0`, which is a fine `float` there.

## Running and failure kinds

The program runs with the same Python that runs `run.py` (`sys.executable`), with `-I` (isolated) and `-X utf8`, in a new temporary folder, for at most 60 seconds. Python has no compile step, so a failure is sorted by the name of the uncaught exception on the traceback's last line:

| Kind | When |
|---|---|
| timeout | The program ran for more than 60 seconds |
| syntax | `SyntaxError`, `IndentationError`, `TabError` |
| type | `NameError`, `UnboundLocalError`, `TypeError`, `AttributeError`, `ImportError`, `ModuleNotFoundError`: the nearest Python has to a compiler's type and name errors, though they are found only when the line runs |
| panic | Any other uncaught exception |
| logic | It ran to its end and printed something else |

There is no `capability` kind: Python has no capabilities.

## Conditions

Python takes part in `zero` and `repair` only (`CONDITIONS` in `language.py`). It has no guide for AI tools like Kavach's `llms.txt`, so `docs` isn't offered and `repair` sends no system prompt: it is `zero` with the errors of each failed try sent back. It has no MCP server here, so `tools` isn't offered either. `run.py` stops with a message when asked for a condition a language doesn't offer.
