"""The agent benchmark's Python side for the crash case (method.md): the
refund requests of the large case, one after another, paying refunds
through the bank's web service, the two usual ways.

  plain     The plain-prompt way of the large case. Started again after a
            crash, it starts from the first request, as such a script does.
  progress  The same, writing each finished request's id to a file, and
            skipping those when started again: the care a script needs
            by hand. A crash between a refund and its line in the file
            still pays that refund again.

Neither sends the bank an idempotency key: that is code no one wrote
here, and with it the bank would drop the repeat as it does for Kavach.

    python crash.py <plain|progress> <data file> <bank url> <progress file>
"""
import json
import os
import sys
import urllib.request

from refunds import plain


def main():
    way, path, bank, progress = sys.argv[1:5]
    done = set()
    if way == "progress" and os.path.exists(progress):
        done = set(open(progress, encoding="utf8").read().split())
    paid = 0
    for o in json.load(open(path, encoding="utf8")):
        if str(o["id"]) in done:
            continue
        try:
            action = plain(o)
        except RuntimeError:
            action = "error"
        if action == "refund":
            url = f"{bank}/refund?order={o['id']}&amount={o['amount']}"
            with urllib.request.urlopen(urllib.request.Request(url, data=b"", method="POST"), timeout=60):
                paid += 1
        if way == "progress":
            with open(progress, "a", encoding="utf8") as f:
                f.write(f"{o['id']}\n")
    print(f"paid {paid}")


if __name__ == "__main__":
    main()
