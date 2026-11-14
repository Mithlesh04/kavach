"""Can AI write it? Runs the tasks of `tasks/` in one language of
`languages/`, through Claude models, under each condition, and grades each
answer by running its hidden tests. See README.md ("Run it yourself") and
method.md.

    python run.py --check                      # no key: every reference solution passes
    python run.py --estimate                   # no key: what a run would cost at most
    python run.py --models haiku --conditions zero,docs
"""

import argparse
import datetime
import importlib.util
import json
import os
import shutil
import sys
import tempfile
import time

HERE = os.path.dirname(os.path.abspath(__file__))
MODELS = {
    "haiku": "claude-haiku-5-5",
    "sonnet": "claude-sonnet-5-5",
    "opus": "claude-opus-5-5",
    "fable": "claude-fable-5-2",
}
CONDITIONS = {
    "zero": "No help: the task only",
    "docs": "The language's guide for AI tools (`llms.txt`) given",
    "repair": "The guide, and the errors of each failed try sent back",
    "tools": "The guide, and the language's own tools (`kavach mcp`) to use",
}
MAX_TOKENS = 8000


def load_language(name):
    spec = importlib.util.spec_from_file_location(name, os.path.join(HERE, "languages", name, "language.py"))
    lang = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(lang)
    return lang


def load_tasks(language, extension, only):
    with open(os.path.join(HERE, "tasks", "tasks.json"), encoding="utf-8") as f:
        shared = {t["id"]: t for t in json.load(f)["tasks"]}
    out = []
    for tid, t in shared.items():
        if only and tid not in only:
            continue
        path = os.path.join(HERE, "languages", language, "tasks", f"{tid}.json")
        if not os.path.exists(path):
            continue
        with open(path, encoding="utf-8") as f:
            own = json.load(f)
        with open(os.path.join(HERE, "languages", language, "solutions", tid + extension), encoding="utf-8") as f:
            own["solution"] = f.read()
        own["description"] = t["description"]
        own["kind"] = t["kind"]
        out.append(own)
    return out


def grade(lang, task, code):
    """(passed, failure kind, what the run said)."""
    printed, errors, timed_out = lang.run(lang.program(task, code))
    want = [t["prints"] for t in task["tests"]]
    kind = lang.failure(printed, errors, timed_out, want)
    passed = kind == "none" and printed == want
    said = errors.strip() or ("" if passed else f"printed {printed}, wanted {want}")
    return passed, ("none" if passed else kind), said[:2000]


def check(lang, tasks):
    """Every reference solution must pass its tests, or the task is wrong."""
    bad = 0
    for t in tasks:
        passed, kind, said = grade(lang, t, t["solution"])
        bad += not passed
        print(("ok   " if passed else "FAIL ") + t["id"] + ("" if passed else f": {kind}: {said[:300]}"))
    print(f"{len(tasks) - bad} of {len(tasks)} reference solutions pass")
    return bad == 0


def prices():
    with open(os.path.join(HERE, "prices.json"), encoding="utf-8") as f:
        return json.load(f)


def cost(model, usage, table):
    """Dollars for one response's `usage`, at `prices.json`'s rates."""
    p = table["models"][model]
    million = 1_000_000
    return (
        usage.input_tokens * p["input"]
        + usage.output_tokens * p["output"]
        + (getattr(usage, "cache_read_input_tokens", 0) or 0) * p["cache_read"]
        + (getattr(usage, "cache_creation_input_tokens", 0) or 0) * p["cache_write"]
    ) / million


def text_of(response):
    return "".join(b.text for b in response.content if b.type == "text")


def attempt(client, lang, task, model, condition, rounds, table):
    """One task, one model, one condition: tries until it passes or the
    rounds run out. Gives the record kept in the results."""
    import anthropic  # only for paid runs

    system = []
    if condition != "zero":
        system = [{"type": "text", "text": f"The {lang.NAME} guide:\n\n{lang.docs()}",
                   "cache_control": {"type": "ephemeral"}}]
    ask = lang.ask(task, task["description"])
    server, scratch, tools = None, None, []
    if condition == "tools":
        from mcp_client import Server
        server = Server(lang.mcp_command())
        scratch = tempfile.mkdtemp()
        tools = [WRITE_CODE] + server.tools()
        ask += TOOLS_NOTE.format(file=lang.SCRATCH)
    messages = [{"role": "user", "content": ask}]
    tries = rounds if condition in ("repair", "tools") else 1
    record = {"task": task["id"], "kind": task["kind"], "model": model, "condition": condition,
              "passed": False, "tries": 0, "calls": 0, "tool_calls": 0, "input_tokens": 0,
              "output_tokens": 0, "cache_read_tokens": 0, "dollars": 0.0, "seconds": 0.0,
              "failures": []}
    started = time.monotonic()
    try:
        while record["tries"] < tries and record["calls"] < tries * TURNS:
            try:
                response = client.messages.create(model=model, max_tokens=MAX_TOKENS, system=system,
                                                  messages=messages, **({"tools": tools} if tools else {}))
            except anthropic.APIError as e:
                record["failures"].append({"kind": "api", "said": str(e)[:500]})
                break
            record["calls"] += 1
            u = response.usage
            record["input_tokens"] += u.input_tokens + (getattr(u, "cache_creation_input_tokens", 0) or 0)
            record["output_tokens"] += u.output_tokens
            record["cache_read_tokens"] += getattr(u, "cache_read_input_tokens", 0) or 0
            record["dollars"] += cost(model, u, table)
            if response.stop_reason == "refusal":
                record["failures"].append({"kind": "refusal", "said": ""})
                break
            messages.append({"role": "assistant", "content": response.content})
            if response.stop_reason == "tool_use":
                results = []
                for b in response.content:
                    if b.type == "tool_use":
                        record["tool_calls"] += 1
                        text, is_error = use_tool(lang, server, scratch, task, b.name, b.input)
                        results.append({"type": "tool_result", "tool_use_id": b.id,
                                        "content": text[:4000] or "(nothing)", "is_error": is_error})
                messages.append({"role": "user", "content": results})
                continue
            record["tries"] += 1
            code = lang.code_of(text_of(response))
            passed, kind, said = grade(lang, task, code)
            record["code"] = code
            if passed:
                record["passed"] = True
                break
            record["failures"].append({"kind": kind, "said": said[:600]})
            messages.append({"role": "user", "content":
                             f"That didn't work:\n\n{said}\n\nGive the whole corrected code, in one ```{lang.FENCE} block."})
    finally:
        if server:
            server.close()
            shutil.rmtree(scratch, ignore_errors=True)
    record["seconds"] = round(time.monotonic() - started, 2)
    return record


# The `tools` condition: the language's MCP tools work on one file, which
# the model fills with this tool of the runner's own.
WRITE_CODE = {
    "name": "write_code",
    "description": "Saves code to the file the other tools work on, replacing what it held. Add a main function to try it with the run tool.",
    "input_schema": {"type": "object", "properties": {"code": {"type": "string", "description": "The whole file"}},
                     "required": ["code"]},
}
TOOLS_NOTE = ("\n\nYou have tools: `write_code` saves code to `{file}`, and the others check, "
              "run or test that file (their `file` is always it). Try your code before you answer. "
              "Then give the final code, with no `main`, in one block.")
# API calls allowed for each graded try, tool calls included.
TURNS = 8


def use_tool(lang, server, scratch, task, name, arguments):
    """Runs one tool call of the model's; gives (text, is it an error)."""
    path = os.path.join(scratch, lang.SCRATCH)
    if name == "write_code":
        code = str(arguments.get("code", ""))
        given = task["given"] if task["given"] and task["given"] not in code else ""
        with open(path, "w", encoding="utf-8", newline="\n") as f:
            f.write(f"{given}\n\n{code}\n" if given else code + "\n")
        return f"saved to {lang.SCRATCH}", False
    if not os.path.exists(path):
        return f"{lang.SCRATCH} is empty: save code with write_code first", True
    text, is_error = server.call(name, {**dict(arguments), "file": path})
    return text.replace(path, lang.SCRATCH).replace(scratch, "."), is_error


def main():
    a = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    a.add_argument("--language", default="kavach")
    a.add_argument("--models", default="haiku", help="any of " + ",".join(MODELS))
    a.add_argument("--conditions", default="zero", help="any of " + ",".join(CONDITIONS))
    a.add_argument("--tasks", default="", help="task ids, comma-separated (default: all)")
    a.add_argument("--rounds", type=int, default=3, help="tries per task under repair and tools")
    a.add_argument("--budget", type=float, default=5.0, help="stop when this many dollars are spent")
    a.add_argument("--check", action="store_true", help="check the tasks, with no model")
    a.add_argument("--estimate", action="store_true", help="the most a run could cost, with no model")
    a.add_argument("--out", default="", help="results file (default results/<date>-<language>.json)")
    args = a.parse_args()
    lang = load_language(args.language)
    only = {t for t in args.tasks.split(",") if t}
    tasks = load_tasks(args.language, lang.EXTENSION, only)
    if args.check:
        sys.exit(0 if check(lang, tasks) else 1)
    models = [MODELS[m] for m in args.models.split(",")]
    conditions = args.conditions.split(",")
    table = prices()
    if args.estimate:
        estimate(lang, tasks, models, conditions, args.rounds, table)
        return
    import anthropic  # the official SDK: pip install -r requirements.txt

    client = anthropic.Anthropic()  # reads ANTHROPIC_API_KEY
    out = args.out or os.path.join(HERE, "results", f"{datetime.date.today()}-{args.language}.json")
    os.makedirs(os.path.dirname(out), exist_ok=True)
    records, spent = [], 0.0
    for model in models:
        for condition in conditions:
            for t in tasks:
                if spent >= args.budget:
                    print(f"stopped: ${spent:.2f} spent, the budget is ${args.budget:.2f}")
                    break
                r = attempt(client, lang, t, model, condition, args.rounds, table)
                spent += r["dollars"]
                records.append(r)
                print(f"{model} {condition} {t['id']}: {'pass' if r['passed'] else 'fail'} "
                      f"in {r['tries']} ({r['dollars']:.4f}$, total {spent:.2f}$)")
                save(out, args, records, spent)
    print(f"done: {len(records)} answers, ${spent:.2f}, in {out}")


def save(out, args, records, spent):
    with open(out, "w", encoding="utf-8", newline="\n") as f:
        json.dump({"language": args.language, "date": str(datetime.date.today()),
                   "rounds": args.rounds, "max_tokens": MAX_TOKENS, "dollars": round(spent, 4),
                   "answers": records}, f, indent=1)


def estimate(lang, tasks, models, conditions, rounds, table):
    """A ceiling: every try sends the task, the guide when given, and gets
    back an answer of 1,500 tokens (thinking included), with every try of
    repair and tools used. Characters / 4 stands in for tokens."""
    guide = len(lang.docs()) / 4
    total = 0.0
    for model in models:
        p = table["models"][model]
        for c in conditions:
            tries = rounds if c in ("repair", "tools") else 1
            for t in tasks:
                ask_tokens = len(lang.ask(t, t["description"])) / 4 + (guide if c != "zero" else 0)
                for n in range(tries):
                    sent = ask_tokens + n * 2000  # earlier tries and their errors resent
                    total += (sent * p["input"] + 1500 * p["output"]) / 1_000_000
    print(f"at most about ${total:.2f} for {len(tasks)} tasks, {len(models)} models, {len(conditions)} conditions")


if __name__ == "__main__":
    main()
