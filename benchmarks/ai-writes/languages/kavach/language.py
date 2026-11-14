"""What the runner needs to know about Kavach: how to ask for a solution,
put it together with the hidden tests, run it, and say why it failed.
Every language folder under `languages/` has a `language.py` with these
same names, so the runner (`run.py`) needs no change for a new one."""

import os
import re
import shutil
import subprocess
import tempfile

NAME = "Kavach"
FENCE = "kavach"
EXTENSION = ".kav"
HERE = os.path.dirname(os.path.abspath(__file__))
# `llms.txt` at the repository's root: Kavach's guide for AI tools.
DOCS = os.path.normpath(os.path.join(HERE, "..", "..", "..", "..", "llms.txt"))


def tool():
    """The `kavach` command: `KAVACH` names it, else the one on `PATH`."""
    found = os.environ.get("KAVACH") or shutil.which("kavach")
    if not found:
        raise SystemExit("kavach not found: put it on PATH or set KAVACH to its path")
    return found


# The file the `tools` condition's tools work on.
SCRATCH = "main.kav"


def mcp_command():
    """Kavach's MCP server (spec 13), for the `tools` condition."""
    return [tool(), "mcp"]


def docs():
    """Kavach's guide for AI tools, for the conditions that give it."""
    with open(DOCS, encoding="utf-8") as f:
        return f.read()


def ask(task, description):
    """The request for one task: what to write, the code given, and the
    signature to fill in."""
    given = f"\nThis code is already there; use it, don't repeat it:\n\n```{FENCE}\n{task['given']}\n```\n" if task["given"] else ""
    return (
        f"Write this in the Kavach programming language.\n\n{description}\n{given}\n"
        f"Write `{task['signature']}`. Give only the code, in one ```{FENCE} block, "
        "with no `main` function and no tests."
    )


def code_of(answer):
    """The code in the answer's first fenced block, or the whole answer."""
    m = re.search(r"```[a-zA-Z]*\n(.*?)```", answer, re.S)
    return (m.group(1) if m else answer).strip() + "\n"


def program(task, code):
    """The solution with the given code and a `main` that prints each
    hidden test's call, in order."""
    given = task["given"] if task["given"] and task["given"] not in code else ""
    calls = "\n".join(f"    print({t['call']})" for t in task["tests"])
    return f"{given}\n\n{code}\n\nfn main() {{\n{calls}\n}}\n"


def run(source, timeout=60):
    """Runs the program; gives (printed lines, error text, timed out)."""
    with tempfile.TemporaryDirectory() as d:
        path = os.path.join(d, "main.kav")
        with open(path, "w", encoding="utf-8", newline="\n") as f:
            f.write(source)
        try:
            out = subprocess.run([tool(), "run", path], capture_output=True, text=True,
                                 encoding="utf-8", timeout=timeout)
        except subprocess.TimeoutExpired:
            return [], "", True
        return out.stdout.splitlines(), out.stderr.replace(path, "main.kav"), False


def failure(printed, errors, timed_out, want):
    """Why a run failed, one of: syntax, type, capability, panic, timeout,
    logic. Kavach's messages say which (`kavach check --format json` has
    no category yet, so the wording is matched)."""
    if timed_out:
        return "timeout"
    first = next((l for l in errors.splitlines() if l.startswith("error")), "")
    if first:
        text = first.lower()
        if re.search(r"expected (the end|an expression|`|a |an )|unexpected|not kavach|start a new line|plain text|block after|never closed", text):
            return "syntax"
        if re.search(r"\buses\b|capabilit", text):
            return "capability"
        return "type"
    if "panic" in errors:
        return "panic"
    return "logic" if printed != want else "none"
