import json
from dataclasses import dataclass


@dataclass
class Item:
    name: str
    qty: int


def total_qty(text: str) -> int:
    try:
        items = [Item(**d) for d in json.loads(text)]
    except (ValueError, TypeError):
        return -1
    return sum(i.qty for i in items)
