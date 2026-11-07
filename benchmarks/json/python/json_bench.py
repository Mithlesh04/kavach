# JSON: the same work as kavach/json.kav, with Python's json module
# (its C parser and encoder). Decoding into classes uses dataclasses.
import json
import time
from dataclasses import dataclass
from typing import Optional


@dataclass
class Item:
    sku: str
    qty: int
    price: float


@dataclass
class Order:
    id: int
    customer: str
    paid: bool
    note: Optional[str]
    items: list


def order(i):
    items = [Item(f"SKU-{(i + k) % 97}", 1 + k, (i % 1000) / 4.0) for k in range(1 + i % 4)]
    note = "leave at the door" if i % 5 == 0 else None
    return Order(i, f"customer {i % 1000}", i % 3 == 0, note, items)


def to_json(o):
    return {"id": o.id, "customer": o.customer, "paid": o.paid, "note": o.note,
            "items": [{"sku": t.sku, "qty": t.qty, "price": t.price} for t in o.items]}


def main():
    orders = [order(i) for i in range(300_000)]

    t0 = time.monotonic()
    text = json.dumps([to_json(o) for o in orders], separators=(",", ":"))
    t1 = time.monotonic()
    doc = json.loads(text)
    t2 = time.monotonic()
    qty = 0
    total = 0.0
    for o in doc:
        for item in o["items"]:
            qty += item["qty"]
            total += item["price"]
    t3 = time.monotonic()
    decoded = [Order(o["id"], o["customer"], o["paid"], o.get("note"),
                     [Item(t["sku"], t["qty"], t["price"]) for t in o["items"]])
               for o in json.loads(text)]
    t4 = time.monotonic()

    ms = lambda a, b: int((b - a) * 1000)
    print(f"{len(text) // 1_000_000} MB, {qty} items, total {total}, {len(decoded)} decoded")
    print(f"stringify {ms(t0, t1)} ms")
    print(f"parse {ms(t1, t2)} ms")
    print(f"read {ms(t2, t3)} ms")
    print(f"decode {ms(t3, t4)} ms")


main()
