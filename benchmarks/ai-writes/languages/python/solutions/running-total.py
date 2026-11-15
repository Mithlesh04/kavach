from itertools import accumulate


def running(xs: list[int]) -> list[int]:
    return list(accumulate(xs))
