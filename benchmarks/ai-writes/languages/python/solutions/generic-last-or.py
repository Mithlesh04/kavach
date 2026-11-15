from typing import TypeVar

T = TypeVar("T")


def last_or(xs: list[T], fallback: T) -> T:
    return xs[-1] if xs else fallback
