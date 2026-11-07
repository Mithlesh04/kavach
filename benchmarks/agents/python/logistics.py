"""The agent benchmark's Python side for the logistics case (README.md):
shipment exceptions under a policy, the two usual ways, one after another.

  plain    A prompt that asks for JSON, parsed and checked by hand, asked
           again when it doesn't fit; a reroute costing over 300 is
           escalated instead, by a check written next to it (no one
           approves).
  agentic  An agent loop as frameworks run one: the policy and four tools
           (reroute, refund, wait, escalate); it calls one, gets the result
           back and answers again, until it stops. The reroute tool
           refuses a cost over 300 and escalates instead.

Prints `id action` per shipment, as the Kavach side does.
"""
import json
import sys

from triage import call

POLICY = (
    "Policy: reroute a shipment that went to the wrong place. Refund one the carrier has lost "
    "(no scan for over a week, or the search closed). Wait when it is only delayed and has a new "
    "date within two days. Escalate damage, customs holds, and anything you are unsure about."
)
ACTIONS = ["reroute", "refund", "wait", "escalate"]


def plain(s):
    prompt = (
        f"{POLICY}\n\nShipment {s['id']}: {s['note']}\n\n"
        'Reply with JSON only: {"action": "reroute" or "refund" or "wait" or "escalate"}.'
    )
    for _ in range(3):
        reply = call({"messages": [{"role": "user", "content": prompt}]})
        text = "".join(b.get("text", "") for b in reply["content"] if b["type"] == "text")
        try:
            v = json.loads(text[text.index("{"): text.rindex("}") + 1])
        except ValueError:
            continue
        if v.get("action") in ACTIONS:
            if v["action"] == "reroute" and s["reroute_cost"] > 300:
                return "escalate"
            return v["action"]
    raise RuntimeError("no valid answer")


TOOLS = [
    {"name": name, "description": what,
     "input_schema": {"type": "object", "properties": {"shipment": {"type": "integer"}}, "required": ["shipment"]}}
    for name, what in [
        ("reroute", "Send the shipment on to the right place."),
        ("refund", "Refund the customer for a lost shipment."),
        ("wait", "Leave the shipment; it is only delayed."),
        ("escalate", "Hand the shipment to a person."),
    ]
]


def agentic(s):
    messages = [{"role": "user", "content": f"Shipment {s['id']}: {s['note']}"}]
    done = None
    for _ in range(6):
        reply = call({"system": POLICY + " Act on the shipment with one of the tools.",
                      "messages": messages, "tools": TOOLS})
        messages.append({"role": "assistant", "content": reply["content"]})
        uses = [b for b in reply["content"] if b["type"] == "tool_use"]
        if not uses:
            break
        results = []
        for u in uses:
            name = u["name"]
            if name == "reroute" and s["reroute_cost"] > 300:
                result, name = "Refused: a reroute over 300 needs a dispatcher. Escalated.", "escalate"
            else:
                result = f"Done: {name}."
            done = done or name
            results.append({"type": "tool_result", "tool_use_id": u["id"], "content": result})
        messages.append({"role": "user", "content": results})
    return done or "escalate"


def main():
    way, path = sys.argv[1], sys.argv[2]
    act = {"plain": plain, "agentic": agentic}[way]
    for s in json.load(open(path, encoding="utf8")):
        try:
            print(f"{s['id']} {act(s)}")
        except RuntimeError:
            print(f"{s['id']} error")


if __name__ == "__main__":
    main()
