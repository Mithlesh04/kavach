"""Runs the agent benchmark (README.md): each case, each way, through a
proxy that counts every call's tokens, then grades the answers against
the data's expected ones.

    ANTHROPIC_API_KEY=… python run.py <kavach binary> [case]

The key is read from the environment only, and never written anywhere.
"""
import http.server
import json
import os
import subprocess
import sys
import threading
import time
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
counts = {"calls": 0, "input": 0, "output": 0}
lock = threading.Lock()


class Proxy(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        body = self.rfile.read(int(self.headers.get("content-length", 0)))
        keep = ("x-api-key", "anthropic-version", "content-type", "anthropic-beta")
        headers = {k: v for k, v in self.headers.items() if k.lower() in keep}
        req = urllib.request.Request("https://api.anthropic.com" + self.path, data=body, headers=headers)
        try:
            with urllib.request.urlopen(req, timeout=180) as r:
                status, data = r.status, r.read()
        except urllib.error.HTTPError as e:
            status, data = e.code, e.read()
        usage = json.loads(data).get("usage") or {}
        with lock:
            counts["calls"] += 1
            counts["input"] += usage.get("input_tokens", 0)
            counts["output"] += usage.get("output_tokens", 0)
        self.send_response(status)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def log_message(self, *args):
        pass


def measure(command, env):
    with lock:
        counts.update(calls=0, input=0, output=0)
    start = time.time()
    out = subprocess.run(command, capture_output=True, text=True, env=env, cwd=HERE)
    seconds = time.time() - start
    if out.returncode != 0:
        print(out.stdout, out.stderr, file=sys.stderr)
    with lock:
        return dict(counts), seconds, out.stdout


def grade(stdout, want):
    """How many items got the expected answer: `want` maps an id to it."""
    got = {}
    for line in stdout.splitlines():
        parts = line.split(maxsplit=1)
        if len(parts) == 2:
            got[parts[0]] = parts[1].strip()
    return sum(got.get(k) == v for k, v in want.items()), len(want)


def cases(kavach):
    """Each case: its data file, the expected answer of each item, and the
    ways it runs."""
    main_kav = os.path.join(HERE, "kavach", "src", "main.kav")
    k = lambda case, path, *more: [kavach, "run", main_kav, "--", case, path, *more]
    py = lambda script, way, path: [sys.executable, f"python/{script}.py", way, path]
    tickets = os.path.join(HERE, "data", "tickets.json")
    orders = os.path.join(HERE, "data", "orders.json")
    yield "Small: sort 20 support tickets", tickets, (
        lambda d: f"{d['kind']} {str(d['needs_person']).lower()}"
    ), [
        ("Kavach, all at once", k("triage", tickets)),
        ("Kavach, one by one", k("triage", tickets, "one-by-one")),
        ("Python, plain prompt", py("triage", "plain", tickets)),
        ("Python, tool call", py("triage", "tools", tickets)),
    ]
    yield "Large: 30 refund requests under a policy", orders, (lambda d: d["expect"]), [
        ("Kavach agent, all at once", k("refunds", orders)),
        ("Kavach agent, one by one", k("refunds", orders, "one-by-one")),
        ("Python, plain prompt", py("refunds", "plain", orders)),
        ("Python, agentic tool loop", py("refunds", "agentic", orders)),
    ]
    notes = os.path.join(HERE, "data", "notes.json")
    yield "Health records: medicines in 10 clinical notes", notes, (lambda d: d["meds"]), [
        ("Kavach, all at once", k("health", notes)),
        ("Kavach, one by one", k("health", notes, "one-by-one")),
        ("Python, plain prompt", py("health", "plain", notes)),
        ("Python, tool call", py("health", "tools", notes)),
    ]
    alerts = os.path.join(HERE, "data", "alerts.json")
    yield "DevOps: 12 alerts against a runbook", alerts, (lambda d: d["expect"]), [
        ("Kavach agent, all at once", k("devops", alerts)),
        ("Kavach agent, one by one", k("devops", alerts, "one-by-one")),
        ("Python, plain prompt", py("devops", "plain", alerts)),
        ("Python, agentic tool loop", py("devops", "agentic", alerts)),
    ]
    contracts = os.path.join(HERE, "data", "contracts.json")
    yield "Legal review: the terms of 12 contract clauses", contracts, (lambda d: d["expect"]), [
        ("Kavach, all at once", k("legal", contracts)),
        ("Kavach, one by one", k("legal", contracts, "one-by-one")),
        ("Python, plain prompt", py("legal", "plain", contracts)),
        ("Python, tool call", py("legal", "tools", contracts)),
    ]
    shipments = os.path.join(HERE, "data", "shipments.json")
    yield "Logistics: 12 shipment exceptions under a policy", shipments, (lambda d: d["expect"]), [
        ("Kavach agent, all at once", k("logistics", shipments)),
        ("Kavach agent, one by one", k("logistics", shipments, "one-by-one")),
        ("Python, plain prompt", py("logistics", "plain", shipments)),
        ("Python, agentic tool loop", py("logistics", "agentic", shipments)),
    ]
    claims = os.path.join(HERE, "data", "claims.json")
    yield "Complex: 12 insurance claims through three agents", claims, (lambda d: d["expect"]), [
        ("Kavach agents, all at once", k("claims", claims)),
        ("Kavach agents, one by one", k("claims", claims, "one-by-one")),
        ("Python, plain prompts", py("claims", "plain", claims)),
        ("Python, agentic tool loop", py("claims", "agentic", claims)),
    ]


def main():
    kavach = os.path.abspath(sys.argv[1])
    only = sys.argv[2] if len(sys.argv) > 2 else None
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Proxy)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    url = f"http://127.0.0.1:{server.server_port}"
    env = dict(os.environ, KAVACH_AI_URL=url, ANTHROPIC_BASE_URL=url)
    saved = {}
    for title, data_file, expect, ways in cases(kavach):
        if only and only.lower() not in title.lower():
            continue
        data = json.load(open(data_file, encoding="utf8"))
        want = {str(d["id"]): expect(d) for d in data}
        print(f"\n{title}\n")
        print("| Way | Calls | Input tokens | Output tokens | Time | Right |")
        print("|---|---|---|---|---|---|")
        for name, command in ways:
            c, seconds, stdout = measure(command, env)
            right, total = grade(stdout, want)
            print(f"| {name} | {c['calls']} | {c['input']:,} | {c['output']:,} | {seconds:.1f} s | {right}/{total} |", flush=True)
            answers = dict(line.split(maxsplit=1) for line in stdout.splitlines() if " " in line)
            saved.setdefault(title, {})[name] = {**c, "seconds": round(seconds, 1), "right": right,
                                                 "total": total, "answers": answers}
    # Every answer, for reading the cases one by one (results/README.md).
    os.makedirs(os.path.join(HERE, "results"), exist_ok=True)
    # A run of some cases keeps the day's other cases.
    path = os.path.join(HERE, "results", time.strftime("%Y-%m-%d") + ".json")
    kept = json.load(open(path, encoding="utf8")) if os.path.exists(path) else {}
    kept.update(saved)
    # One case to a line, so the file stays short as cases are added.
    lines = [f"{json.dumps(case)}: {json.dumps(r, ensure_ascii=False)}" for case, r in kept.items()]
    with open(path, "w", encoding="utf8", newline="\n") as f:
        f.write("{\n" + ",\n".join(lines) + "\n}\n")


main()
