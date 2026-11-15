from collections.abc import Callable


def adder(step: int) -> Callable[[int], int]:
    return lambda x: x + step
