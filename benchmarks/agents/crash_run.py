"""A crash halfway (method.md): each way pays the large case's refunds
through a bank's web service; the program is killed while the bank
answers its 6th refund (the bank has the money moved, the program never
hears it), then started again. Counted: refunds paid twice, repeats the
bank dropped by their key, and the model calls and tokens of both runs.

    ANTHROPIC_API_KEY=… python crash_run.py <kavach binary> [--model <name>]
"""
import http.server
import json
import os
import shutil
import subprocess
import sys
import threading
import time
import urllib.parse

import run

HERE = os.path.dirname(os.path.abspath(__file__))
# The refund during which the program is killed.
CUT_AT = 6


class Bank(http.server.BaseHTTPRequestHandler):
    """Pays each refund once per key; a refund without a key is paid
    every time it comes. Holds the answer to the `CUT_AT`th payment for
    a while, so the program can be killed waiting for it."""

    paid = []       # (order, amount)
    dropped = 0     # repeats of a key it had seen
    keys = set()
    cut = threading.Event()
    lock = threading.Lock()

    def do_POST(self):
        q = dict(urllib.parse.parse_qsl(urllib.parse.urlparse(self.path).query))
        hold = False
        with Bank.lock:
            key = q.get("key")
            if key and key in Bank.keys:
                Bank.dropped += 1
            else:
                if key:
                    Bank.keys.add(key)
                Bank.paid.append((int(q["order"]), int(q["amount"])))
                hold = len(Bank.paid) == CUT_AT and not Bank.cut.is_set()
        if hold:
            Bank.cut.set()
            time.sleep(5)
        try:
            self.send_response(200)
            self.send_header("content-length", "2")
            self.end_headers()
            self.wfile.write(b"ok")
        except ConnectionError:
            pass  # the program was killed while it waited: the money moved anyway

    def log_message(self, *args):
        pass

    @classmethod
    def reset(cls):
        cls.paid, cls.dropped, cls.keys = [], 0, set()
        cls.cut.clear()


def clean(progress):
    """Forgets this case's state: the agent's folder and the progress file."""
    shutil.rmtree(os.path.join(HERE, "kavach", "agents", "Refunder"), ignore_errors=True)
    if os.path.exists(progress):
        os.remove(progress)


def crash_and_resume(first, again, env):
    """Runs `first` until the bank holds the cut payment, kills it, then
    runs `again` to the end."""
    p = subprocess.Popen(first, cwd=HERE, env=env, stdin=subprocess.DEVNULL,
                         stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    while not Bank.cut.wait(0.1):
        if p.poll() is not None:
            break
    p.kill()
    p.communicate()
    out = subprocess.run(again, cwd=HERE, env=env, stdin=subprocess.DEVNULL,
                         capture_output=True, text=True)
    if out.returncode != 0:
        print(out.stdout, out.stderr, file=sys.stderr)


def main():
    args = sys.argv[1:]
    if "--native" in args:
        args.remove("--native")
        run.NATIVE.append("--native")
    if "--model" in args:
        at = args.index("--model")
        run.MODEL = args[at + 1]
        del args[at:at + 2]
    kavach = os.path.abspath(args[0])
    proxy = http.server.ThreadingHTTPServer(("127.0.0.1", 0), run.Proxy)
    bank = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Bank)
    for s in (proxy, bank):
        threading.Thread(target=s.serve_forever, daemon=True).start()
    url = f"http://127.0.0.1:{proxy.server_port}"
    bank_url = f"http://127.0.0.1:{bank.server_port}"
    env = dict(os.environ, KAVACH_AI_URL=url, ANTHROPIC_BASE_URL=url)
    orders = os.path.join(HERE, "data", "orders.json")
    progress = os.path.join(HERE, "kavach", "agents", "crash-progress.txt")
    main_kav = os.path.join(HERE, "kavach", "src", "main.kav")
    k = lambda mode: [kavach, "run", *run.NATIVE, main_kav, "--", "crash", orders, bank_url, mode]
    py = lambda way: [sys.executable, "python/crash.py", way, orders, bank_url, progress]
    ways = [
        ("Kavach agent, durable handler", k("start"), k("resume")),
        ("Python, plain prompt, started again", py("plain"), py("plain")),
        ("Python, with a progress file", py("progress"), py("progress")),
    ]
    print(f"\nA crash at the {CUT_AT}th refund" + (f" ({run.MODEL})" if run.MODEL else "") + "\n")
    print("| Way | Refunds paid | Paid twice | Extra money | Repeats dropped by key | Calls | Input tokens | Output tokens | Cost |")
    print("|---|---|---|---|---|---|---|---|---|")
    saved = {}
    for name, first, again in ways:
        clean(progress)
        os.makedirs(os.path.dirname(progress), exist_ok=True)
        Bank.reset()
        with run.lock:
            run.counts.update(run.fresh())
        crash_and_resume(first, again, env)
        c = dict(run.counts)
        times = {}
        for order, amount in Bank.paid:
            times[order] = times.get(order, 0) + 1
        twice = sorted(o for o, n in times.items() if n > 1)
        extra = sum(a for o, a in Bank.paid) - sum({o: a for o, a in Bank.paid}.values())
        print(f"| {name} | {len(Bank.paid)} | {len(twice)} | {extra} USD | {Bank.dropped} | {c['calls']} "
              f"| {c['input']:,} | {c['output']:,} | ${c['cost']:.4f} |", flush=True)
        saved[name] = {"paid": Bank.paid, "twice": twice, "extra": extra, "dropped": Bank.dropped,
                       "calls": c["calls"], "input": c["input"], "output": c["output"],
                       "cost": round(c["cost"], 5)}
    clean(progress)
    day = time.strftime("%Y-%m-%d") + (f"-{run.MODEL}" if run.MODEL else "") + ("-native" if run.NATIVE else "")
    path = os.path.join(HERE, "results", f"crash-{day}.json")
    with open(path, "w", encoding="utf8", newline="\n") as f:
        f.write("{\n" + ",\n".join(f"{json.dumps(k)}: {json.dumps(v)}" for k, v in saved.items()) + "\n}\n")


if __name__ == "__main__":
    main()
