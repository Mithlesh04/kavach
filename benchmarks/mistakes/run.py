"""Checks each mistake in cases/ with `kavach check` and prints the table
for README.md: which step catches it before a run, and the message. A
copy of each case is given to `kavach fix`, to see if it has an automatic fix.

    python benchmarks/mistakes/run.py [path to kavach]
"""
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).parent
KAVACH = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "target/debug/kavach")
if sys.platform == "win32":
    KAVACH = KAVACH.with_suffix(".exe")
KAVACH = str(KAVACH.resolve())


def first_message(path):
    out = subprocess.run([KAVACH, "check", str(path)], capture_output=True, text=True)
    for line in (out.stdout + out.stderr).splitlines():
        m = re.match(r"^(error|warning): (.*)", line)
        if m:
            return m.group(1), m.group(2)
    return None, None


def fixed_for_you(path):
    """Whether `kavach fix` fixes a copy, so that it has no message left."""
    with tempfile.TemporaryDirectory(dir=HERE) as folder:
        copy = pathlib.Path(folder) / path.name
        shutil.copy(path, copy)
        subprocess.run([KAVACH, "fix", str(copy)], capture_output=True, text=True)
        return first_message(copy) == (None, None)


def main():
    rows, caught, fixed = [], 0, 0
    for path in sorted((HERE / "cases").glob("*.kav")):
        title = path.read_text(encoding="utf8").splitlines()[0]
        title = title.removeprefix("// Mistake: ").removesuffix(".")
        kind, message = first_message(path)
        if kind is None:
            step = "only when it runs"
        else:
            step = "compile error" if kind == "error" else "warning"
            if fixed_for_you(path):
                step += ", auto-fix"
                fixed += 1
        caught += kind is not None
        shown = (message or "").replace("|", "\\|")
        rows.append(f"| [{title}](cases/{path.name}) | {step} | {shown} |")
    print("| Mistake | Caught by | Message |")
    print("|---|---|---|")
    print("\n".join(rows))
    print(f"\n{caught} of {len(rows)} caught before a run, {fixed} of them with an auto-fix.")


main()
