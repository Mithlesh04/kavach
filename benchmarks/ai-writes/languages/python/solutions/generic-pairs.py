from typing import TypeVar

T = TypeVar("T")


def pairs(xs: list[T]) -> list[tuple[T, T]]:
    return list(zip(xs, xs[1:]))
