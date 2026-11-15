from dataclasses import dataclass


@dataclass
class Circle:
    radius: float


@dataclass
class Square:
    side: float


Shape = Circle | Square


def area(s: Shape) -> float:
    match s:
        case Circle(radius=r):
            return 3.14 * r * r
        case Square(side=side):
            return side * side
