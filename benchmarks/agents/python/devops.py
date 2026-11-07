"""The agent benchmark's Python side for the DevOps case (README.md): alerts
against a runbook, the two usual ways, one after another.

  plain    A prompt that asks for JSON, parsed and checked by hand, asked
           again when it doesn't fit; a restart in production is paged
           instead, by a check written next to it (no one approves).
  agentic  An agent loop as frameworks run one: the runbook and four tools
           (restart, scale, page, ignore); it calls one, gets the result
           back and answers again, until it stops. The restart tool
           refuses production and pages instead.

Prints `id action` per alert, as the Kavach side does.
"""
import json
import sys

from triage import call

RUNBOOK = (
    "Runbook: restart a service whose process crashed or is stuck. Scale up when load or latency "
    "is high but the service is healthy. Page a person for data loss, security alerts, expired "
    "certificates, or anything you are unsure about. Ignore noise from test environments."
)
ACTIONS = ["restart", "scale", "page", "ignore"]


def plain(a):
    prompt = (
        f"{RUNBOOK}\n\nService {a['service']} in {a['env']}: {a['alert']}\n\n"
        'Reply with JSON only: {"action": "restart" or "scale" or "page" or "ignore"}.'
    )
    for _ in range(3):
        reply = call({"messages": [{"role": "user", "content": prompt}]})
        text = "".join(b.get("text", "") for b in reply["content"] if b["type"] == "text")
        try:
            v = json.loads(text[text.index("{"): text.rindex("}") + 1])
        except ValueError:
            continue
        if v.get("action") in ACTIONS:
            if v["action"] == "restart" and a["env"] == "production":
                return "page"
            return v["action"]
    raise RuntimeError("no valid answer")


TOOLS = [
    {"name": name, "description": what,
     "input_schema": {"type": "object", "properties": {"service": {"type": "string"}}, "required": ["service"]}}
    for name, what in [
        ("restart", "Restart the service."),
        ("scale", "Add capacity to the service."),
        ("page", "Wake the on-call person."),
        ("ignore", "Close the alert as noise."),
    ]
]


def agentic(a):
    messages = [{"role": "user", "content": f"Service {a['service']} in {a['env']}: {a['alert']}"}]
    done = None
    for _ in range(6):
        reply = call({"system": RUNBOOK + " Act on the alert with one of the tools.",
                      "messages": messages, "tools": TOOLS})
        messages.append({"role": "assistant", "content": reply["content"]})
        uses = [b for b in reply["content"] if b["type"] == "tool_use"]
        if not uses:
            break
        results = []
        for u in uses:
            name = u["name"]
            if name == "restart" and a["env"] == "production":
                result, name = "Refused: restarting in production needs the on-call person. Paged.", "page"
            else:
                result = f"Done: {name}."
            done = done or name
            results.append({"type": "tool_result", "tool_use_id": u["id"], "content": result})
        messages.append({"role": "user", "content": results})
    return done or "page"


def main():
    way, path = sys.argv[1], sys.argv[2]
    act = {"plain": plain, "agentic": agentic}[way]
    for a in json.load(open(path, encoding="utf8")):
        try:
            print(f"{a['id']} {act(a)}")
        except RuntimeError:
            print(f"{a['id']} error")


if __name__ == "__main__":
    main()
