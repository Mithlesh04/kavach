"""What the runner needs to know about Python: how to ask for a solution,
put it together with the hidden tests, run it, and say why it failed.
Python is the benchmark's reference: the same tasks, in a language every
model has seen a great deal of. See README.md in this folder."""

import os
import re
import subprocess
import sys
import tempfile

NAME = "Python"
FENCE = "python"
EXTENSION = ".py"
# The conditions Python takes part in. It has no guide for AI tools and no
# MCP server, so `docs` and `tools` are left out, and `repair` gets no guide.
CONDITIONS = ("zero", "repair")

# The file the `tools` condition's tools would work on.
SCRATCH = "main.py"


def mcp_command():
    """Python has no MCP tools here: the `tools` condition isn't offered."""
    raise SystemExit("Python has no MCP tools condition: use --conditions zero,repair")


def docs():
    """Python has no guide for AI tools here: nothing is given."""
    return ""


def ask(task, description):
    """The request for one task: what to write, the code given, what the
    task's Kavach words mean in Python, and the signature to fill in."""
    given = f"\nThis code is already there; use it, don't repeat it:\n\n```{FENCE}\n{task['given']}\n```\n" if task["given"] else ""
    note = f"\n{task['note']}\n" if task.get("note") else ""
    return (
        f"Write this in Python 3.\n\n{description}\n{given}{note}\n"
        f"Write `{task['signature']}`. Give only the code, in one ```{FENCE} block, "
        "with no main code and no tests."
    )


def code_of(answer):
    """The code in the answer's first fenced block, or the whole answer."""
    m = re.search(r"```[a-zA-Z0-9]*\n(.*?)```", answer, re.S)
    return (m.group(1) if m else answer).strip() + "\n"


# Added after the solution: prints each test's value the way Kavach prints
# it, so both languages' tests expect the very same lines.
HELPERS = r'''
def _kv_show(value, top=True):
    if value is None:
        return "none"
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, str):
        if top:
            return value
        return '"' + value.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n") + '"'
    if isinstance(value, (list, set, frozenset)):
        return "[" + ", ".join(_kv_show(v, False) for v in value) + "]"
    if isinstance(value, tuple):
        return "(" + ", ".join(_kv_show(v, False) for v in value) + ")"
    if isinstance(value, dict):
        if not value:
            return "[:]"
        return "[" + ", ".join(_kv_show(k, False) + ": " + _kv_show(v, False) for k, v in value.items()) + "]"
    return str(value)


def _kv_catch(call, error, fallback):
    """Kavach's `call() catch _ => fallback`, for the error type `error`."""
    try:
        return call()
    except error:
        return fallback


def _kv_changed(call, value):
    """Calls `call(value)`, which changes `value` in place, and gives it."""
    call(value)
    return value
'''


def program(task, code):
    """The given code, the solution, the helpers, and a print of each
    hidden test's call, in order."""
    given = task["given"] if task["given"] and task["given"] not in code else ""
    calls = "\n".join(f"print(_kv_show({t['call']}))" for t in task["tests"])
    return f"{given}\n\n{code}\n{HELPERS}\n{calls}\n"


def run(source, timeout=60):
    """Runs the program with the Python running the runner, isolated (`-I`)
    and in UTF-8; gives (printed lines, error text, timed out)."""
    with tempfile.TemporaryDirectory() as d:
        path = os.path.join(d, SCRATCH)
        with open(path, "w", encoding="utf-8", newline="\n") as f:
            f.write(source)
        try:
            out = subprocess.run([sys.executable, "-I", "-X", "utf8", path], capture_output=True,
                                 text=True, encoding="utf-8", timeout=timeout, cwd=d)
        except subprocess.TimeoutExpired:
            return [], "", True
        return out.stdout.splitlines(), out.stderr.replace(path, SCRATCH), False


# Python finds these before running anything, or they are the nearest it has
# to a compiler's type and name errors.
SYNTAX = {"SyntaxError", "IndentationError", "TabError"}
TYPE = {"NameError", "UnboundLocalError", "TypeError", "AttributeError", "ImportError",
        "ModuleNotFoundError"}


def failure(printed, errors, timed_out, want):
    """Why a run failed, one of: syntax, type, panic, timeout, logic. The
    uncaught exception's name, on the traceback's last line, says which."""
    if timed_out:
        return "timeout"
    names = re.findall(r"^([A-Za-z_][\w.]*)(?::|$)", errors, re.M)
    raised = next((n.rsplit(".", 1)[-1] for n in reversed(names) if n[:1].isupper()), "")
    if raised in SYNTAX:
        return "syntax"
    if raised in TYPE:
        return "type"
    if raised or "Traceback" in errors:
        return "panic"
    return "logic" if printed != want else "none"
