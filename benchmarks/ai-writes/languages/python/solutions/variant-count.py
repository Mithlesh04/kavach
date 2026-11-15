from dataclasses import dataclass


@dataclass
class Click:
    pass


@dataclass
class Key:
    code: int


@dataclass
class Scroll:
    dy: int


Event = Click | Key | Scroll


def key_count(events: list[Event]) -> int:
    return sum(1 for e in events if isinstance(e, Key))
