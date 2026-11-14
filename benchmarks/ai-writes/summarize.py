"""Turns a results file of `run.py` into the README's tables: Pass@1,
repair rounds, cost and time, and failure kinds, as Markdown.

    python summarize.py results/2026-11-14-kavach.json
"""

import collections
import json
import sys

NAMES = {
    "claude-haiku-5-5": "Claude Haiku 5.5",
    "claude-sonnet-5-5": "Claude Sonnet 5.5",
    "claude-opus-5-5": "Claude Opus 5.5",
    "claude-fable-5-2": "Claude Fable 5.2",
}
CONDITIONS = ["zero", "docs", "repair", "tools"]


def table(title, cell, cells):
    lines = [f"## {title}", "", "| Model | " + " | ".join(CONDITIONS) + " |",
             "|---" * (len(CONDITIONS) + 1) + "|"]
    for model, name in NAMES.items():
        row = [cell(cells.get((model, c), [])) for c in CONDITIONS]
        lines.append(f"| {name} | " + " | ".join(row) + " |")
    return "\n".join(lines) + "\n"


def main():
    with open(sys.argv[1], encoding="utf-8") as f:
        answers = json.load(f)["answers"]
    cells = collections.defaultdict(list)
    for a in answers:
        cells[(a["model"], a["condition"])].append(a)

    def passed(rs):
        if not rs:
            return "–"
        n = sum(r["passed"] for r in rs)
        return f"{100 * n / len(rs):.0f}% ({n}/{len(rs)})"

    def rounds(rs):
        ok = [r["tries"] for r in rs if r["passed"]]
        return f"{sum(ok) / len(ok):.2f}" if ok else "–"

    def cost(rs):
        if not rs:
            return "–"
        return f"${sum(r['dollars'] for r in rs):.2f}, {sum(r['seconds'] for r in rs) / 60:.0f} min"

    def kinds(rs):
        failed = [r["failures"][-1]["kind"] for r in rs if not r["passed"] and r["failures"]]
        if not rs:
            return "–"
        if not failed:
            return "none"
        counts = collections.Counter(failed).most_common()
        return ", ".join(f"{k} {n}" for k, n in counts)

    print(table("Pass@1", passed, cells))
    print(table("Repair rounds", rounds, cells))
    print(table("Cost and time", cost, cells))
    print(table("Failure kinds", kinds, cells))
    total = sum(a["dollars"] for a in answers)
    print(f"{len(answers)} answers, ${total:.2f} in all")


if __name__ == "__main__":
    main()
