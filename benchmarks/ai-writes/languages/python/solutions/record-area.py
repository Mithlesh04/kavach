from dataclasses import dataclass


@dataclass
class Rect:
    width: float
    height: float


def total_area(rects: list[Rect]) -> float:
    return sum((r.width * r.height for r in rects), 0.0)
