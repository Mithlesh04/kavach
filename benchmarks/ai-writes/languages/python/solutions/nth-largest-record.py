from dataclasses import dataclass


@dataclass
class Player:
    name: str
    score: int


def top(players: list[Player], k: int) -> list[str]:
    ranked = sorted(players, key=lambda p: p.score, reverse=True)
    return [p.name for p in ranked[:k]]
