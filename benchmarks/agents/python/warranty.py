"""The agent benchmark's Python side for the warranty desk case
(README.md): what to do about each customer message, which needs the
order and the product's warranty first, the two usual ways, one after
another.

  plain    The code looks up the order and the warranty, puts the facts
           in a prompt that asks for JSON, parses and checks the answer
           by hand; a replacement over 1,000 is escalated by a check.
  agentic  An agent loop as frameworks run one: the policy, two lookup
           tools (the order, a product's warranty) and five action tools;
           the model looks up what it needs, then acts, until it stops.
           The replace tool refuses an item over 1,000 and escalates.

Prints `id action` per message, as the Kavach side does.
"""
import json
import sys

from triage import call

POLICY = (
    "Warranty policy: replace an item that stopped working while still under warranty (delivered "
    "no more days ago than its warranty in months times 30). Decline when the warranty has ended. "
    "Refund an order the carrier lost. Tell the customer to wait when the order is in transit. "
    "Escalate a threat, and anything you are unsure about."
)
ACTIONS = ["replace", "refund", "wait", "decline", "escalate"]


def plain(t, db):
    o = db["orders"][t["order"]]
    months = db["warranty_months"][o["product"]]
    prompt = (
        f"{POLICY}\n\nOrder {t['order']}: {o['product']}, {o['price']} USD, status {o['status']}, "
        f"delivered {o['days']} days ago, warranty {months} months.\nThe customer writes: {t['message']}\n\n"
        'Reply with JSON only: {"action": "replace" or "refund" or "wait" or "decline" or "escalate"}.'
    )
    for _ in range(3):
        reply = call({"messages": [{"role": "user", "content": prompt}]})
        text = "".join(b.get("text", "") for b in reply["content"] if b["type"] == "text")
        try:
            v = json.loads(text[text.index("{"): text.rindex("}") + 1])
        except ValueError:
            continue
        if v.get("action") in ACTIONS:
            if v["action"] == "replace" and o["price"] > 1000:
                return "escalate"
            return v["action"]
    raise RuntimeError("no valid answer")


def tool(name, what, field="order"):
    return {"name": name, "description": what,
            "input_schema": {"type": "object", "properties": {field: {"type": "string"}}, "required": [field]}}


TOOLS = [
    tool("lookup_order", "The order's product, price in USD, status and days since delivery."),
    tool("warranty_months", "How many months a product's warranty lasts.", "product"),
    tool("replace", "Send the customer a new item."),
    tool("refund", "Refund the order."),
    tool("wait", "Tell the customer the order is on its way."),
    tool("decline", "Tell the customer the warranty has ended."),
    tool("escalate", "Hand the message to a person."),
]


def agentic(t, db):
    messages = [{"role": "user", "content": f"Order {t['order']}. The customer writes: {t['message']}"}]
    done = None
    for _ in range(8):
        reply = call({"system": POLICY + " Look up what you need, then act with one of the tools.",
                      "messages": messages, "tools": TOOLS})
        messages.append({"role": "assistant", "content": reply["content"]})
        uses = [b for b in reply["content"] if b["type"] == "tool_use"]
        if not uses:
            break
        results = []
        for u in uses:
            name, arg = u["name"], u["input"]
            if name == "lookup_order":
                result = json.dumps(db["orders"].get(arg.get("order"), "no such order"))
            elif name == "warranty_months":
                result = str(db["warranty_months"].get(arg.get("product"), "unknown product"))
            elif name == "replace" and db["orders"].get(t["order"], {}).get("price", 0) > 1000:
                result, name = "Refused: a replacement over 1,000 needs a supervisor. Escalated.", "escalate"
                done = done or name
            else:
                result = f"Done: {name}."
                done = done or name
            results.append({"type": "tool_result", "tool_use_id": u["id"], "content": result})
        messages.append({"role": "user", "content": results})
    return done or "escalate"


def main():
    way, path, db_path = sys.argv[1], sys.argv[2], sys.argv[3]
    db = json.load(open(db_path, encoding="utf8"))
    act = {"plain": plain, "agentic": agentic}[way]
    for t in json.load(open(path, encoding="utf8")):
        try:
            print(f"{t['id']} {act(t, db)}")
        except RuntimeError:
            print(f"{t['id']} error")


if __name__ == "__main__":
    main()
