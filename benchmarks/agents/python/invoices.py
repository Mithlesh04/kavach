"""The agent benchmark's Python side for the invoices case (README.md):
read each invoice into its currency, line items and total, the two usual
ways, one after another.

  plain  A prompt that asks for JSON, parsed and checked by hand (the
         currency one of four, each line's quantity a whole number and
         its price a number, the total a number), asked again when it
         doesn't fit.
  tools  A forced tool call whose input schema is the answer, as agent
         frameworks give typed output (LangChain's with_structured_output),
         checked the same way.

Prints `id currency lines cents` per invoice, as the Kavach side does.
"""
import json
import sys
from decimal import Decimal, InvalidOperation

from triage import call

CURRENCIES = ["USD", "EUR", "GBP", "INR"]
SCHEMA = {
    "type": "object",
    "properties": {
        "currency": {"type": "string", "enum": CURRENCIES},
        "lines": {
            "type": "array",
            "description": "only the goods or services bought: not discounts, tax, delivery or shipping",
            "items": {
                "type": "object",
                "properties": {
                    "description": {"type": "string"},
                    "quantity": {"type": "integer"},
                    "unit_price": {"type": "string", "description": "a decimal number"},
                },
                "required": ["description", "quantity", "unit_price"],
            },
        },
        "total": {"type": "string", "description": "the amount due, with tax, delivery and discounts: a decimal number"},
    },
    "required": ["currency", "lines", "total"],
}


def number(x):
    try:
        return Decimal(str(x))
    except InvalidOperation:
        return None


def checked(v):
    """`id`-less answer as `currency lines cents`, or None when it doesn't fit."""
    if not isinstance(v, dict) or v.get("currency") not in CURRENCIES or not isinstance(v.get("lines"), list):
        return None
    for line in v["lines"]:
        if not (isinstance(line, dict) and isinstance(line.get("description"), str)
                and isinstance(line.get("quantity"), int) and number(line.get("unit_price")) is not None):
            return None
    total = number(v.get("total"))
    if total is None:
        return None
    return f"{v['currency']} {len(v['lines'])} {int(total * 100)}"


def plain(text):
    prompt = (
        f"Read this invoice.\n\n{text}\n\n"
        "Reply with JSON only, with these keys: currency (one of USD, EUR, GBP, INR), lines (a list of "
        "the goods or services bought, not discounts, tax, delivery or shipping, each with description, "
        "quantity as a whole number, and unit_price as a decimal number), total (the amount due, with "
        "tax, delivery and discounts, as a decimal number)."
    )
    for _ in range(3):
        reply = call({"messages": [{"role": "user", "content": prompt}]})
        out = "".join(b.get("text", "") for b in reply["content"] if b["type"] == "text")
        try:
            v = json.loads(out[out.index("{"): out.rindex("}") + 1])
        except ValueError:
            continue
        answer = checked(v)
        if answer:
            return answer
    raise RuntimeError("no valid answer")


def tools(text):
    tool = {"name": "record_invoice", "description": "Record what an invoice says", "input_schema": SCHEMA}
    for _ in range(3):
        reply = call({
            "messages": [{"role": "user", "content": f"Read this invoice.\n\n{text}"}],
            "tools": [tool],
            "tool_choice": {"type": "tool", "name": "record_invoice"},
        })
        uses = [b for b in reply["content"] if b["type"] == "tool_use"]
        answer = uses and checked(uses[0]["input"])
        if answer:
            return answer
    raise RuntimeError("no valid answer")


def main():
    way, path = sys.argv[1], sys.argv[2]
    read = {"plain": plain, "tools": tools}[way]
    for t in json.load(open(path, encoding="utf8")):
        try:
            print(f"{t['id']} {read(t['text'])}")
        except RuntimeError:
            print(f"{t['id']} error")


if __name__ == "__main__":
    main()
