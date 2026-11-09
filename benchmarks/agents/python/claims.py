"""The agent benchmark's Python side for the complex case (README.md):
insurance claims, the two usual ways, one after another.

  plain    The same three steps as the Kavach agents, on the same models
           (intake and fraud on claude-haiku-5-5, the payout decision on
           claude-sonnet-5-5), each a prompt asking for JSON, parsed and
           checked by hand, asked again when it doesn't fit.
  agentic  One agent on the careful model with the policy and three tools
           (pay, deny, send to review), run as frameworks run one: it calls
           a tool, gets the result back, and answers again, until it stops.
           A payout over 2,000 is refused by the tool and sent to review.

Prints `id outcome` per claim, as the Kavach side does.
"""
import json
import sys

import triage
from triage import call

QUICK, CAREFUL = "claude-haiku-5-5", "claude-sonnet-5-5"
POLICY = (
    "Policy: covers car accidents, home water damage from burst pipes, home theft reported to the "
    "police, and trip cancellation for illness of the traveller or close family with a doctor's or "
    "hospital letter. It does not cover wear and tear, floods from rivers or rain, lost luggage "
    "without an airline report, or cosmetic damage."
)


def ask_json(model, prompt, keys):
    triage.MODEL = model
    for _ in range(3):
        reply = call({"messages": [{"role": "user", "content": prompt}]})
        text = "".join(b.get("text", "") for b in reply["content"] if b["type"] == "text")
        try:
            v = json.loads(text[text.index("{"): text.rindex("}") + 1])
        except ValueError:
            continue
        if all(isinstance(v.get(k), bool) for k in keys):
            return v
    raise RuntimeError("no valid answer")


def plain(c):
    claim = f"Claim {c['id']}, {c['amount']} USD: {c['text']}"
    i = ask_json(QUICK, f"{POLICY}\n\n{claim}\n\nReply with JSON only: "
                 '{"covered": true if the policy covers what happened, '
                 '"missing_documents": true if the documents the policy asks for are missing}.',
                 ["covered", "missing_documents"])
    if not i["covered"]:
        return "deny"
    f = ask_json(QUICK, f"Does this insurance claim look fraudulent? {claim}\n\nReply with JSON only: "
                 '{"suspicious": true if the claim shows signs of fraud: urgency, pressure for cash, '
                 'repeated claims, no evidence}.', ["suspicious"])
    if f["suspicious"] or i["missing_documents"]:
        return "review"
    p = ask_json(CAREFUL, f"{POLICY}\n\nThe claim is covered and its documents are in order. "
                 f"Should we pay it? {claim}\n\nReply with JSON only: "
                 '{"pay": true if the claim should be paid in full}.', ["pay"])
    if not p["pay"]:
        return "deny"
    return "review" if c["amount"] > 2000 else "pay"


TOOLS = [
    {"name": name, "description": what,
     "input_schema": {"type": "object", "properties": {"claim_id": {"type": "integer"}}, "required": ["claim_id"]}}
    for name, what in [
        ("pay", "Pay the claim in full."),
        ("deny", "Deny the claim: the policy doesn't cover it."),
        ("send_to_review", "Send the claim to a person: documents are missing, or it may be fraud."),
    ]
]


def agentic(c):
    triage.MODEL = CAREFUL
    system = (POLICY + " Check that the policy covers the claim, that its documents are in order and "
              "that it shows no signs of fraud (urgency, pressure for cash, repeated claims, no evidence). "
              "Then act with one of the tools.")
    messages = [{"role": "user", "content": f"Claim {c['id']}, {c['amount']} USD: {c['text']}"}]
    done = None
    for _ in range(6):
        reply = call({"system": system, "messages": messages, "tools": TOOLS})
        messages.append({"role": "assistant", "content": reply["content"]})
        uses = [b for b in reply["content"] if b["type"] == "tool_use"]
        if not uses:
            break
        results = []
        for u in uses:
            name = {"send_to_review": "review"}.get(u["name"], u["name"])
            if name == "pay" and c["amount"] > 2000:
                result, name = "Refused: payouts over 2,000 need a senior person. Sent to review.", "review"
            else:
                result = f"Done: {name}."
            done = done or name
            results.append({"type": "tool_result", "tool_use_id": u["id"], "content": result})
        messages.append({"role": "user", "content": results})
    return done or "review"


def main():
    way, path = sys.argv[1], sys.argv[2]
    act = {"plain": plain, "agentic": agentic}[way]
    for c in json.load(open(path, encoding="utf8")):
        try:
            print(f"{c['id']} {act(c)}")
        except RuntimeError:
            print(f"{c['id']} error")


if __name__ == "__main__":
    main()
