"""The agent benchmark's Python side for the small case (README.md): sort
support tickets the two usual ways, one ticket after another.

  plain  A prompt that asks for JSON; the reply is parsed and checked by
         hand, and asked again (twice at most) when it doesn't fit.
  tools  A forced tool call whose input schema is the answer, as agent
         frameworks give typed output (LangChain's with_structured_output).

Prints `id kind needs_person` per ticket, as the Kavach side does. The
key comes from ANTHROPIC_API_KEY; ANTHROPIC_BASE_URL sends the calls to the
runner's counting proxy. Only the standard library, so nothing to install.
"""
import json
import os
import sys
import urllib.request

MODEL = "claude-haiku-5-5"
KINDS = ["Billing", "Bug", "Account", "Feature", "Other"]
SCHEMA = {
    "type": "object",
    "properties": {
        "kind": {"type": "string", "enum": KINDS, "description": "what the ticket is about"},
        "urgency": {"type": "integer", "description": "1 (can wait) to 5 (customers are losing money now)"},
        "needs_person": {"type": "boolean", "description": "true if a person must answer, not a canned reply"},
    },
    "required": ["kind", "urgency", "needs_person"],
}


def call(body):
    base = os.environ.get("ANTHROPIC_BASE_URL", "https://api.anthropic.com")
    req = urllib.request.Request(
        base + "/v1/messages",
        data=json.dumps({"model": MODEL, "max_tokens": 1024, **body}).encode(),
        headers={
            "x-api-key": os.environ["ANTHROPIC_API_KEY"],
            "anthropic-version": "2023-06-01",
            "content-type": "application/json",
        },
    )
    with urllib.request.urlopen(req, timeout=120) as r:
        return json.loads(r.read())


def valid(v):
    return (
        isinstance(v, dict)
        and v.get("kind") in KINDS
        and isinstance(v.get("urgency"), int)
        and isinstance(v.get("needs_person"), bool)
    )


def plain(text):
    prompt = (
        f"Triage this support ticket: {text}\n\n"
        "Reply with JSON only, with these keys: kind (one of Billing, Bug, Account, Feature, Other: "
        "what the ticket is about), urgency (1 can wait to 5 customers are losing money now), "
        "needs_person (true if a person must answer, not a canned reply)."
    )
    for _ in range(3):
        reply = call({"messages": [{"role": "user", "content": prompt}]})
        text_out = "".join(b.get("text", "") for b in reply["content"] if b["type"] == "text")
        try:
            v = json.loads(text_out[text_out.index("{"): text_out.rindex("}") + 1])
        except ValueError:
            continue
        if valid(v):
            return v
    raise RuntimeError("no valid answer")


def tools(text):
    tool = {"name": "record_triage", "description": "Record the triage of a support ticket", "input_schema": SCHEMA}
    for _ in range(3):
        reply = call({
            "messages": [{"role": "user", "content": f"Triage this support ticket: {text}"}],
            "tools": [tool],
            "tool_choice": {"type": "tool", "name": "record_triage"},
        })
        uses = [b for b in reply["content"] if b["type"] == "tool_use"]
        if uses and valid(uses[0]["input"]):
            return uses[0]["input"]
    raise RuntimeError("no valid answer")


def main():
    way, path = sys.argv[1], sys.argv[2]
    ask = {"plain": plain, "tools": tools}[way]
    for t in json.load(open(path, encoding="utf8")):
        try:
            v = ask(t["text"])
        except RuntimeError:
            print(f"{t['id']} error")
            continue
        print(f"{t['id']} {v['kind']} {str(v['needs_person']).lower()}")


if __name__ == "__main__":
    main()
