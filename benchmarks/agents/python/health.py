"""The agent benchmark's Python side for the health-records case
(README.md): the medicines a clinical note says the patient takes now, as
a list, the two usual ways, one note after another.

  plain  A prompt that asks for a JSON list, parsed and checked by hand,
         asked again when it doesn't fit.
  tools  A forced tool call whose input holds the list, as frameworks give
         typed output.

Prints `id name dosexN, …` per note (or `id none`), as the Kavach side does.
"""
import json
import sys

from triage import call

QUESTION = (
    "List the medicines this clinical note says the patient takes now, with the dose and how often. "
    "Leave out medicines stopped, declined or used only as needed. Note: "
)
ITEM = {
    "type": "object",
    "properties": {
        "name": {"type": "string", "description": "the medicine's name, in lowercase, without the dose"},
        "dose_mg": {"type": "integer", "description": "one dose, in mg"},
        "times_per_day": {"type": "integer", "description": "how many doses a day"},
    },
    "required": ["name", "dose_mg", "times_per_day"],
}


def valid(meds):
    return isinstance(meds, list) and all(
        isinstance(m, dict) and isinstance(m.get("name"), str)
        and isinstance(m.get("dose_mg"), int) and isinstance(m.get("times_per_day"), int)
        for m in meds
    )


def plain(text):
    prompt = (
        QUESTION + text + "\n\nReply with a JSON list only, each item "
        '{"name": the medicine\'s name in lowercase without the dose, "dose_mg": one dose in mg, '
        '"times_per_day": how many doses a day}; [] if there are none.'
    )
    for _ in range(3):
        reply = call({"messages": [{"role": "user", "content": prompt}]})
        out = "".join(b.get("text", "") for b in reply["content"] if b["type"] == "text")
        try:
            meds = json.loads(out[out.index("["): out.rindex("]") + 1])
        except ValueError:
            continue
        if valid(meds):
            return meds
    raise RuntimeError("no valid answer")


def tools(text):
    tool = {"name": "record_medicines", "description": "Record the medicines the patient takes now",
            "input_schema": {"type": "object", "properties": {"medicines": {"type": "array", "items": ITEM}},
                             "required": ["medicines"]}}
    for _ in range(3):
        reply = call({"messages": [{"role": "user", "content": QUESTION + text}],
                      "tools": [tool], "tool_choice": {"type": "tool", "name": "record_medicines"}})
        uses = [b for b in reply["content"] if b["type"] == "tool_use"]
        if uses and valid(uses[0]["input"].get("medicines")):
            return uses[0]["input"]["medicines"]
    raise RuntimeError("no valid answer")


def main():
    way, path = sys.argv[1], sys.argv[2]
    ask = {"plain": plain, "tools": tools}[way]
    for n in json.load(open(path, encoding="utf8")):
        try:
            meds = ask(n["text"])
        except RuntimeError:
            print(f"{n['id']} error")
            continue
        each = sorted(f"{m['name']} {m['dose_mg']}x{m['times_per_day']}" for m in meds)
        print(f"{n['id']} {', '.join(each) if each else 'none'}")


if __name__ == "__main__":
    main()
